//! Actual language-server acceptance: no simulated responses.
use aros_lsp_transport::{FrameReader, FrameWriter, ReadPolicy};
use serde_json::{json, Value};
use std::io::{self, Read};
use std::process::{Command, Stdio};

struct Registration(u32);
inventory::collect!(Registration);
inventory::submit!(Registration(11));

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
struct Client<R, W> {
    reader: FrameReader<R>,
    writer: FrameWriter<W>,
}
impl<R: Read, W: io::Write> Client<R, W> {
    fn send(&mut self, message: Value) -> Result<()> {
        println!("SEND {message}");
        self.writer.write_frame(&serde_json::to_vec(&message)?)?;
        Ok(())
    }
    fn receive(&mut self) -> Result<Value> {
        let bytes = self.reader.read_frame()?.ok_or("unexpected server EOF")?;
        let message: Value = serde_json::from_slice(&bytes)?;
        println!("RECV {message}");
        Ok(message)
    }
    fn response(&mut self, id: i64) -> Result<Value> {
        let message = self.receive()?;
        if message["id"] != id || message.get("error").is_some() {
            return Err(format!("unexpected response for {id}: {message}").into());
        }
        Ok(message["result"].clone())
    }
    fn responses(&mut self, ids: &[i64]) -> Result<std::collections::BTreeMap<i64, Value>> {
        let mut results = std::collections::BTreeMap::new();
        for _ in ids {
            let message = self.receive()?;
            let id = message["id"].as_i64().ok_or("response has no numeric id")?;
            if !ids.contains(&id)
                || results.contains_key(&id)
                || message.get("error").is_some()
                || message.get("result").is_none()
            {
                return Err(format!("unexpected batch response: {message}").into());
            }
            results.insert(id, message["result"].clone());
        }
        Ok(results)
    }
    fn diagnostics(&mut self) -> Result<Value> {
        let message = self.receive()?;
        if message["method"] != "textDocument/publishDiagnostics" {
            return Err(format!("expected diagnostics: {message}").into());
        }
        Ok(message["params"]["diagnostics"].clone())
    }
    fn exercise(&mut self) -> Result<()> {
        self.send(
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "processId":null,"rootUri":null,"capabilities":{}}}),
        )?;
        let initialized = self.response(1)?;
        if initialized["capabilities"]["documentFormattingProvider"] != true {
            return Err("server did not advertise formatting".into());
        }
        self.send(json!({"jsonrpc":"2.0","method":"initialized","params":{}}))?;
        let uri = "file:///acceptance.wat";
        self.send(json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{
            "textDocument":{"uri":uri,"languageId":"wat","version":1,"text":"(module (func (result i32) i32.const))"}}}))?;
        let bad = self.diagnostics()?;
        if bad.as_array().ok_or("invalid diagnostics")?.is_empty() {
            return Err("invalid WAT produced no diagnostic".into());
        }
        println!("PASS invalid WAT diagnostics");
        self.send(json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{
            "textDocument":{"uri":uri,"version":2},
            "contentChanges":[{"text":"(module (func $answer (export \"żółw\") (result i32) i32.const 42))"}]}}))?;
        let repaired = self.diagnostics()?;
        if !repaired
            .as_array()
            .ok_or("invalid repaired diagnostics")?
            .is_empty()
        {
            return Err(format!("repair still has diagnostics: {repaired}").into());
        }
        println!("PASS Unicode document repair clears diagnostics");
        self.send(
            json!({"jsonrpc":"2.0","id":2,"method":"textDocument/formatting","params":{
            "textDocument":{"uri":uri},"options":{"tabSize":2,"insertSpaces":true}}}),
        )?;
        let edits = self.response(2)?;
        let mut formatted =
            "(module (func $answer (export \"żółw\") (result i32) i32.const 42))".to_owned();
        let edits = edits
            .as_array()
            .filter(|x| !x.is_empty())
            .ok_or("no formatting edits")?;
        // LSP positions count UTF-16 code units. Apply from the end so the
        // original positions remain valid while earlier text is unchanged.
        for edit in edits.iter().rev() {
            let offset = |position: &Value| -> Result<usize> {
                if position["line"] != 0 {
                    return Err("unexpected multi-line input position".into());
                }
                let wanted = position["character"]
                    .as_u64()
                    .ok_or("invalid edit position")? as usize;
                let mut units = 0;
                for (byte, ch) in formatted.char_indices() {
                    if units == wanted {
                        return Ok(byte);
                    }
                    units += ch.len_utf16();
                }
                if units == wanted {
                    Ok(formatted.len())
                } else {
                    Err("edit position outside document".into())
                }
            };
            let start = offset(&edit["range"]["start"])?;
            let end = offset(&edit["range"]["end"])?;
            formatted.replace_range(
                start..end,
                edit["newText"].as_str().ok_or("invalid edit text")?,
            );
        }
        if !formatted.contains("żółw") || !formatted.contains("42") || !formatted.contains('\n')
        {
            return Err("formatting lost document content or did not format".into());
        }
        println!("PASS real formatting preserves Unicode and instructions");
        self.send(
            json!({"jsonrpc":"2.0","id":3,"method":"textDocument/documentSymbol","params":{
            "textDocument":{"uri":uri}}}),
        )?;
        let symbols = self.response(3)?;
        if !symbols.to_string().contains("answer") {
            return Err("document symbols missing function".into());
        }
        println!("PASS threaded document-symbol analysis");
        let ids: Vec<_> = (10..26).collect();
        for id in &ids {
            let (method, params) = if id % 2 == 0 {
                (
                    "textDocument/formatting",
                    json!({"textDocument":{"uri":uri},"options":{"tabSize":2,"insertSpaces":true}}),
                )
            } else {
                (
                    "textDocument/documentSymbol",
                    json!({"textDocument":{"uri":uri}}),
                )
            };
            self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?;
        }
        let batch = self.responses(&ids)?;
        for (id, result) in batch {
            if id % 2 == 0 {
                if !result.as_array().is_some_and(|edits| {
                    !edits.is_empty()
                        && edits
                            .iter()
                            .all(|edit| edit["newText"].is_string() && edit["range"].is_object())
                }) {
                    return Err(format!("invalid parallel formatting result for {id}").into());
                }
            } else if !result.to_string().contains("answer") {
                return Err(format!("invalid parallel symbols result for {id}").into());
            }
        }
        println!("PASS 16 outstanding mixed requests matched by ID");
        self.send(json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}))?;
        if !self.response(4)?.is_null() {
            return Err("invalid shutdown response".into());
        }
        Ok(())
    }
}
fn main() -> Result<()> {
    let registered: Vec<_> = inventory::iter::<Registration>
        .into_iter()
        .map(|x| x.0)
        .collect();
    if registered != [11] {
        return Err("inventory constructor did not register before main".into());
    }
    println!("PASS inventory constructor before main");
    let server = std::env::args()
        .nth(1)
        .ok_or("usage: wat-lsp-acceptance SERVER")?;
    let mut child = Command::new(server)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stderr = child.stderr.take().unwrap();
    let drain = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).map(|_| bytes)
    });
    let mut client = Client {
        reader: FrameReader::new(child.stdout.take().unwrap(), ReadPolicy::DosPipe),
        writer: FrameWriter::new(child.stdin.take().unwrap()),
    };
    let result = client.exercise();
    // Even assertion failures close the session and reap the child. A blocked
    // peer needs an external supervisor: AROS Child::kill remains unsupported.
    let exit = client.send(json!({"jsonrpc":"2.0","method":"exit","params":null}));
    drop(client.writer);
    let eof = client.reader.read_frame();
    let status = child.wait()?;
    let stderr = drain.join().map_err(|_| "stderr reader panicked")??;
    println!("SERVER_STATUS {status}; STDERR_BYTES {}", stderr.len());
    if !stderr.is_empty() {
        println!("SERVER_STDERR {}", String::from_utf8_lossy(&stderr));
    }
    result?;
    exit?;
    if eof?.is_some() {
        return Err("unexpected trailing server frame".into());
    }
    if !status.success() {
        return Err("server failed".into());
    }
    println!("PASS shutdown, EOF, child wait and stderr join");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn client(messages: &[Value]) -> Client<Cursor<Vec<u8>>, Vec<u8>> {
        let mut wire = Vec::new();
        for message in messages {
            aros_lsp_transport::write_frame(&mut wire, &serde_json::to_vec(message).unwrap())
                .unwrap();
        }
        Client {
            reader: FrameReader::new(Cursor::new(wire), ReadPolicy::Buffered),
            writer: FrameWriter::new(Vec::new()),
        }
    }
    #[test]
    fn batch_matches_results_by_id_instead_of_arrival_order() {
        let mut client = client(&[
            json!({"jsonrpc":"2.0","id":12,"result":["symbol"]}),
            json!({"jsonrpc":"2.0","id":11,"result":["edit"]}),
        ]);
        let results = client.responses(&[11, 12]).unwrap();
        assert_eq!(results[&11], json!(["edit"]));
        assert_eq!(results[&12], json!(["symbol"]));
    }
    #[test]
    fn duplicate_or_unrequested_responses_fail_the_batch() {
        for messages in [
            vec![
                json!({"jsonrpc":"2.0","id":11,"result":[]}),
                json!({"jsonrpc":"2.0","id":11,"result":[]}),
            ],
            vec![
                json!({"jsonrpc":"2.0","id":99,"result":[]}),
                json!({"jsonrpc":"2.0","id":12,"result":[]}),
            ],
            vec![
                json!({"jsonrpc":"2.0","id":11,"error":{"code":-32603,"message":"failed"}}),
                json!({"jsonrpc":"2.0","id":12,"result":[]}),
            ],
            vec![
                json!({"jsonrpc":"2.0","id":11}),
                json!({"jsonrpc":"2.0","id":12,"result":[]}),
            ],
        ] {
            let error = client(&messages).responses(&[11, 12]).unwrap_err();
            assert!(
                error.to_string().contains("unexpected batch response"),
                "{error}"
            );
        }
    }
}
