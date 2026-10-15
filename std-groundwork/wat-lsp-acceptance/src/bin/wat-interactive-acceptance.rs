//! Real LSP live edits/cancellation with continuous stdout and stderr drains.
use aros_lsp_transport::{FrameReader, ReadPolicy};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::{
    io::Read,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

type Result<T> = std::result::Result<T, String>;
struct Outstanding {
    expected: BTreeSet<i64>,
    received: BTreeSet<i64>,
    diagnostics: usize,
    cancelled: usize,
}
impl Outstanding {
    fn new(ids: impl IntoIterator<Item = i64>) -> Self {
        Self {
            expected: ids.into_iter().collect(),
            received: BTreeSet::new(),
            diagnostics: 0,
            cancelled: 0,
        }
    }
    fn observe(&mut self, message: &Value) -> Result<()> {
        if message["jsonrpc"] != "2.0" {
            return Err("invalid JSON-RPC version".into());
        }
        if message["method"] == "textDocument/publishDiagnostics" {
            if message["params"]["uri"] != "file:///interactive.wat"
                || !message["params"]["diagnostics"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
                || message.get("id").is_some()
                || message.get("result").is_some()
                || message.get("error").is_some()
            {
                return Err("invalid interactive diagnostics".into());
            }
            self.diagnostics += 1;
            return Ok(());
        }
        if message.get("method").is_some() {
            return Err("response has a method".into());
        }
        let id = message["id"].as_i64().ok_or("response has no numeric ID")?;
        if !self.expected.contains(&id) || self.received.contains(&id) {
            return Err(format!("duplicate/unrequested response {id}"));
        }
        if let Some(error) = message.get("error") {
            if error["code"] != -32800 || message.get("result").is_some() {
                return Err(format!("unexpected request error: {message}"));
            }
            self.cancelled += 1;
        } else {
            let result = message.get("result").ok_or("response has no result")?;
            if result.is_null() {
                self.cancelled += 1;
            } else if !result.is_array() {
                return Err("invalid symbols response".into());
            }
        }
        self.received.insert(id);
        Ok(())
    }
    fn complete(&self, diagnostics: usize) -> bool {
        self.received == self.expected && self.diagnostics == diagnostics
    }
}

struct Outbound<W> {
    inner: W,
    failed: bool,
}
impl<W: std::io::Write> Outbound<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            failed: false,
        }
    }
    fn send_many(&mut self, messages: &[Value]) -> Result<()> {
        if self.failed {
            return Err("outbound transport has failed".into());
        }
        let result = (|| {
            if messages.is_empty() || messages.len() > 3 {
                return Err("invalid send group".into());
            }
            let mut wire = Vec::new();
            for message in messages {
                let bytes = serde_json::to_vec(message).map_err(|e| e.to_string())?;
                println!(
                    "SEND {}",
                    json!({"id":message.get("id"),"method":message.get("method"),
                    "version":message["params"]["textDocument"].get("version"),"bytes":bytes.len()})
                );
                aros_lsp_transport::write_frame(&mut wire, &bytes).map_err(|e| e.to_string())?;
                if wire.len() > aros_lsp_transport::MAX_BODY {
                    return Err("send group too large".into());
                }
            }
            self.inner.write_all(&wire).map_err(|e| e.to_string())?;
            self.inner.flush().map_err(|e| e.to_string())
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
}
enum Event {
    Message(Value),
    Eof,
    Error(String),
}
fn receive(events: &mpsc::Receiver<Event>) -> Result<Value> {
    match events
        .recv_timeout(Duration::from_secs(30))
        .map_err(|e| format!("response deadline: {e}"))?
    {
        Event::Message(message) => {
            println!(
                "RECV {}",
                json!({"id":message.get("id"),"method":message.get("method"),
                "result_items":message.get("result").and_then(Value::as_array).map(Vec::len),
                "result_null":message.get("result").is_some_and(Value::is_null),
                "diagnostics":message["params"]["diagnostics"].as_array().map(Vec::len),
                "error":message.get("error")})
            );
            Ok(message)
        }
        Event::Eof => Err("unexpected server EOF".into()),
        Event::Error(error) => Err(error),
    }
}
fn send(writer: &mut Outbound<impl std::io::Write>, message: Value) -> Result<()> {
    writer.send_many(&[message])
}
fn expect_response(events: &mpsc::Receiver<Event>, id: i64) -> Result<Value> {
    let message = receive(events)?;
    if message["id"] != id || message.get("error").is_some() || message.get("result").is_none() {
        return Err(format!("unexpected response for {id}"));
    }
    Ok(message["result"].clone())
}
fn document(version: i64, final_document: bool) -> String {
    let mut text = "(module\n".to_owned();
    for index in 0..1024 {
        let name = if final_document && index == 0 {
            "final_answer".to_owned()
        } else {
            format!("v{version}_f{index}")
        };
        text.push_str(&format!(
            "(func {dollar}{name} (export \"{name}\") (result i32) i32.const {version})\n",
            dollar = '$'
        ));
    }
    text.push(')');
    text
}
fn exercise(
    writer: &mut Outbound<impl std::io::Write>,
    events: &mpsc::Receiver<Event>,
) -> Result<()> {
    let uri = "file:///interactive.wat";
    send(
        writer,
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}),
    )?;
    if expect_response(events, 1)?["capabilities"]["documentSymbolProvider"] != true {
        return Err("missing symbol capability".into());
    }
    send(
        writer,
        json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
    )?;
    send(
        writer,
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{
        "uri":uri,"languageId":"wat","version":1,"text":document(1,false)}}}),
    )?;
    let mut opened = Outstanding::new([]);
    opened.observe(&receive(events)?)?;
    if !opened.complete(1) {
        return Err("missing open diagnostics".into());
    }
    let ids: Vec<i64> = (10..22).collect();
    for (iteration, id) in ids.iter().enumerate() {
        let version = iteration as i64 + 2;
        writer.send_many(&[
            json!({"jsonrpc":"2.0","id":id,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":uri}}}),
            json!({"jsonrpc":"2.0","method":"$/cancelRequest","params":{"id":id}}),
            json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{
                "textDocument":{"uri":uri,"version":version},"contentChanges":[{"text":document(version,false)}]}}),
        ])?;
    }
    let mut state = Outstanding::new(ids);
    while !state.complete(12) {
        state.observe(&receive(events)?)?;
        if state.diagnostics > 12 {
            return Err("too many change diagnostics".into());
        }
    }
    if state.cancelled == 0 {
        return Err("all requests completed normally; interruption was not exercised".into());
    }
    println!(
        "PASS 12 edit/cancel cycles, all replies and diagnostics accounted; null_or_cancelled={}",
        state.cancelled
    );
    send(
        writer,
        json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{
        "textDocument":{"uri":uri,"version":100},"contentChanges":[{"text":document(100,true)}]}}),
    )?;
    let mut final_change = Outstanding::new([]);
    final_change.observe(&receive(events)?)?;
    if !final_change.complete(1) {
        return Err("missing final change diagnostics".into());
    }
    send(
        writer,
        json!({"jsonrpc":"2.0","id":90,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":uri}}}),
    )?;
    let recovered = expect_response(events, 90)?;
    let text = recovered.to_string();
    if !text.contains("final_answer") || text.contains("v1_f") || !text.contains("v100_f1023") {
        return Err("recovery query returned stale or incomplete document".into());
    }
    println!("PASS recovery query sees final document and last function");
    send(
        writer,
        json!({"jsonrpc":"2.0","id":99,"method":"shutdown","params":null}),
    )?;
    if !expect_response(events, 99)?.is_null() {
        return Err("invalid shutdown result".into());
    }
    Ok(())
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let server = std::env::args()
        .nth(1)
        .ok_or("usage: wat-interactive-acceptance SERVER")?;
    let mut child = Command::new(server)
        .args(std::env::args().skip(2))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut writer = Outbound::new(child.stdin.take().unwrap());
    let stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let (tx, rx) = mpsc::channel();
    let output = thread::spawn(move || {
        let mut reader = FrameReader::new(stdout, ReadPolicy::DosPipe);
        loop {
            let event = match reader.read_frame() {
                Ok(Some(body)) => match serde_json::from_slice(&body) {
                    Ok(message) => Event::Message(message),
                    Err(error) => Event::Error(error.to_string()),
                },
                Ok(None) => Event::Eof,
                Err(error) => Event::Error(error.to_string()),
            };
            let terminal = !matches!(&event, Event::Message(_));
            if tx.send(event).is_err() || terminal {
                break;
            }
        }
    });
    let errors = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).map(|_| bytes)
    });
    let result = exercise(&mut writer, &rx);
    if let Err(error) = &result {
        println!("CASE_ERROR {error}");
    }
    let exit = send(
        &mut writer,
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    );
    drop(writer);
    // A deadline is diagnostic, not forced termination: this native profile
    // still needs its external VM supervisor if a peer stops making progress.
    let status = child.wait()?;
    output.join().map_err(|_| "stdout reader panicked")?;
    let stderr = errors.join().map_err(|_| "stderr reader panicked")??;
    println!("SERVER_STATUS {status}; STDERR_BYTES {}", stderr.len());
    if !stderr.is_empty() {
        println!("SERVER_STDERR {}", String::from_utf8_lossy(&stderr));
    }
    result?;
    exit?;
    match rx.recv_timeout(Duration::from_secs(30))? {
        Event::Eof => {}
        _ => return Err("unexpected trailing server output".into()),
    }
    if !status.success() {
        return Err("server failed".into());
    }
    println!("PASS interactive shutdown, EOF, child wait and both reader joins");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn diagnostic() -> Value {
        json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":"file:///interactive.wat","diagnostics":[]}})
    }
    #[test]
    fn partial_group_write_failure_prevents_later_exit_frame() {
        use std::{cell::Cell, rc::Rc};
        struct Broken(Rc<Cell<usize>>);
        impl std::io::Write for Broken {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                let previous = self.0.get();
                self.0.set(previous + 1);
                if previous == 0 {
                    Ok(bytes.len().min(3))
                } else {
                    Err(std::io::ErrorKind::BrokenPipe.into())
                }
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let calls = Rc::new(Cell::new(0));
        let mut writer = Outbound::new(Broken(calls.clone()));
        assert!(writer.send_many(&[json!({"id":1})]).is_err());
        assert!(writer.send_many(&[json!({"method":"exit"})]).is_err());
        assert_eq!(calls.get(), 2);
    }
    #[test]
    fn complete_request_and_cancel_frames_share_one_underlying_write() {
        use std::{cell::RefCell, rc::Rc};
        let writes = Rc::new(RefCell::new(Vec::<Vec<u8>>::new()));
        struct Sink(Rc<RefCell<Vec<Vec<u8>>>>);
        impl std::io::Write for Sink {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0.borrow_mut().push(bytes.to_vec());
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let messages = [
            json!({"jsonrpc":"2.0","id":10,"method":"textDocument/documentSymbol","params":{}}),
            json!({"jsonrpc":"2.0","method":"$/cancelRequest","params":{"id":10}}),
        ];
        Outbound::new(Sink(writes.clone()))
            .send_many(&messages)
            .unwrap();
        assert_eq!(writes.borrow().len(), 1);
        let wire = writes.borrow()[0].clone();
        let mut reader = FrameReader::new(std::io::Cursor::new(wire), ReadPolicy::DosPipe);
        for expected in messages {
            assert_eq!(
                serde_json::from_slice::<Value>(&reader.read_frame().unwrap().unwrap()).unwrap(),
                expected
            );
        }
        assert!(reader.read_frame().unwrap().is_none());
    }
    #[test]
    fn interleaved_diagnostics_and_cancelled_results_complete_exactly_once() {
        let mut state = Outstanding::new([10, 11]);
        state
            .observe(&json!({"jsonrpc":"2.0","id":11,"result":null}))
            .unwrap();
        state.observe(&diagnostic()).unwrap();
        assert!(!state.complete(1));
        state
            .observe(&json!({"jsonrpc":"2.0","id":10,"result":[]}))
            .unwrap();
        assert!(state.complete(1));
        assert_eq!(state.cancelled, 1);
    }
    #[test]
    fn valid_documents_reject_error_diagnostics_and_hybrid_messages() {
        for message in [
            json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":"file:///interactive.wat","diagnostics":[{"severity":1,"message":"bad"}]}}),
            json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","id":10,"result":[],"params":{"uri":"file:///interactive.wat","diagnostics":[]}}),
            json!({"jsonrpc":"2.0","method":"bogus","id":10,"result":[]}),
        ] {
            assert!(Outstanding::new([10]).observe(&message).is_err());
        }
    }
    #[test]
    fn request_cancelled_error_is_accounted_without_failing_the_session() {
        let mut state = Outstanding::new([10]);
        state
            .observe(
                &json!({"jsonrpc":"2.0","id":10,"error":{"code":-32800,"message":"cancelled"}}),
            )
            .unwrap();
        assert!(state.complete(0));
        assert_eq!(state.cancelled, 1);
    }
    #[test]
    fn duplicate_unrequested_and_malformed_messages_are_rejected() {
        for message in [
            json!({"jsonrpc":"2.0","id":99,"result":[]}),
            json!({"jsonrpc":"2.0","id":10}),
            json!({"jsonrpc":"2.0","id":10,"error":{"code":-32603}}),
            json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"diagnostics":null}}),
        ] {
            let mut state = Outstanding::new([10]);
            assert!(state.observe(&message).is_err());
        }
        let mut state = Outstanding::new([10]);
        let response = json!({"jsonrpc":"2.0","id":10,"result":[]});
        state.observe(&response).unwrap();
        assert!(state.observe(&response).is_err());
    }
}
