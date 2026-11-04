//! Diagnose the same SDK calls through Rust FFI and GCC fixed-arity bridges.
use std::ffi::c_int;
use std::fmt::Write;
unsafe extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    fn close(fd: c_int) -> c_int;
    #[link_name = "___geterrnoptr"]
    fn errno_ptr() -> *mut c_int;
    fn rs_probe_pipe(fds: *mut c_int) -> c_int;
    fn rs_probe_fcntl(fd: c_int, cmd: c_int, arg: c_int, has_arg: c_int) -> c_int;
    fn rs_probe_close(fd: c_int) -> c_int;
    fn rs_probe_errno() -> c_int;
    fn rs_probe_clear_errno();
}
pub fn run() -> String {
    let mut log = String::new();
    for bridge in [false, true] {
        let mode = if bridge { "gcc" } else { "rust" };
        unsafe {
            let clear = || {
                *errno_ptr() = 0;
                rs_probe_clear_errno();
            };
            let mut fds = [-1; 2];
            clear();
            let result = if bridge {
                rs_probe_pipe(fds.as_mut_ptr())
            } else {
                pipe(fds.as_mut_ptr())
            };
            writeln!(
                log,
                "mode={mode} pipe={result} fds={fds:?} rust_errno={} gcc_errno={}",
                *errno_ptr(),
                rs_probe_errno()
            )
            .unwrap();
            if result == 0 {
                for (step, cmd, arg, has_arg) in [
                    ("get", 2, 0, false),
                    ("set", 3, 1, true),
                    ("get-set", 2, 0, false),
                    ("clear", 3, 0, true),
                    ("get-clear", 2, 0, false),
                ] {
                    clear();
                    let result = if bridge {
                        rs_probe_fcntl(fds[0], cmd, arg, has_arg as c_int)
                    } else if has_arg {
                        fcntl(fds[0], cmd, arg)
                    } else {
                        fcntl(fds[0], cmd)
                    };
                    writeln!(
                        log,
                        "mode={mode} step={step} result={result} rust_errno={} gcc_errno={}",
                        *errno_ptr(),
                        rs_probe_errno()
                    )
                    .unwrap();
                }
                for fd in fds {
                    clear();
                    let result = if bridge {
                        rs_probe_close(fd)
                    } else {
                        close(fd)
                    };
                    writeln!(
                        log,
                        "mode={mode} close={result} rust_errno={} gcc_errno={}",
                        *errno_ptr(),
                        rs_probe_errno()
                    )
                    .unwrap();
                }
            }
            clear();
            let result = if bridge {
                rs_probe_fcntl(-1, 2, 0, 0)
            } else {
                fcntl(-1, 2)
            };
            writeln!(
                log,
                "mode={mode} invalid-fd={result} rust_errno={} gcc_errno={}",
                *errno_ptr(),
                rs_probe_errno()
            )
            .unwrap();
        }
    }
    format!("result=diagnostic\n{log}")
}

#[repr(C)]
#[derive(Default, Debug)]
struct SpawnProbe {
    pid: i64,
    waited: i64,
    signal_result: usize,
    child_errno: c_int,
    parent_errno: c_int,
    status: c_int,
    exec_return: c_int,
}
unsafe extern "C" {
    fn rs_probe_spawn(program: *const std::ffi::c_char, reset_signal: c_int, out: *mut SpawnProbe);
}
pub fn spawn(mode: c_int) -> std::io::Result<String> {
    use std::os::unix::ffi::OsStrExt;
    let program = std::ffi::CString::new(std::env::current_exe()?.as_os_str().as_bytes())?;
    let mut result = SpawnProbe {
        exec_return: -999,
        ..SpawnProbe::default()
    };
    unsafe {
        rs_probe_spawn(program.as_ptr(), mode, &mut result);
    }
    Ok(format!("result=diagnostic mode={mode} {result:?}"))
}
