use std::{
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
#[test]
fn actual_process_and_pipe_cases() {
    let root = std::env::temp_dir().join(format!(
        "aros-process-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    for case in [
        "status",
        "alloc-spawn",
        "lsp",
        "lsp-bytewise",
        "lsp-invalid-bytewise",
        "lsp-invalid",
        "output",
        "wait",
        "kill",
        "spawn-error",
        "exec-denied",
        "args",
    ] {
        let report: PathBuf = root.join(format!("{case}.txt"));
        let result = Command::new(env!("CARGO_BIN_EXE_process-stdio"))
            .args(["--case", case, "--report"])
            .arg(&report)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "case={case} stdout={} stderr={}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        let text = std::fs::read_to_string(report).unwrap();
        assert!(text.contains("CASE_DONE"), "{text}");
        if case != "kill" {
            assert!(text.contains("result=pass"), "{text}");
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}
