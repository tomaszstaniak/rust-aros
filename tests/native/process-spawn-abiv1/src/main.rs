use std::io::ErrorKind;
use std::process::{Command, Stdio};

fn main() {
    for program in ["C:Version", "RAM:rust-spawn-probe-missing"] {
        let error = Command::new(program).spawn().unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Unsupported, "{program}: {error}");
    }
    let error = Command::new("C:Version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Unsupported);
    println!("PROCESS_SPAWN_UNSUPPORTED_OK");
}
