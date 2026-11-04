//! Deterministic probe of the real Salsa cancellation path, not an LSP race.
use serde_json::json;
use wat_service::LanguageService;

fn main() {
    let mut service = LanguageService::default();
    let uri = "file:///cancellation.wat";
    service.did_open(serde_json::from_value(json!({
        "textDocument": {"uri": uri, "languageId": "wat", "version": 1,
            "text": "(module (func $answer (export \"answer\") (result i32) i32.const 42))"}
    })).unwrap());
    let params = || serde_json::from_value(json!({"textDocument": {"uri": uri}})).unwrap();
    let baseline = service.document_symbol(params()).expect("baseline symbols");
    assert!(!baseline.is_empty());
    println!("PASS uncancelled Salsa query");
    let snapshot = service.clone();
    let token = snapshot.cancellation_token();
    token.cancel();
    assert!(token.is_cancelled());
    println!("CANCELLATION_REQUESTED before query");
    // With unwinding Salsa catches its own cancellation and returns None.
    // With panic=abort this line is expected to terminate the process.
    assert!(snapshot.document_symbol(params()).is_none(), "cancelled query returned symbols");
    println!("PASS cancelled Salsa query caught without terminating process");
}
