//! Portable process/pipe acceptance fixture. Parent and child use the same binary.
use process_stdio::framing::{read_frame, write_frame};
use std::{
    env,
    io::{self, BufReader, Read, Write},
    process::{Command, ExitCode, Stdio},
    thread,
    time::Duration,
};
fn check(ok: bool, message: &str) -> io::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::InvalidData, message))
    }
}
fn self_command(mode: &str) -> io::Result<Command> {
    let mut command = Command::new(env::current_exe()?);
    command.args(["--child", mode]);
    Ok(command)
}
fn child(mode: &str) -> io::Result<u8> {
    match mode {
        "lsp" | "lsp-bad" => {
            let mut input = BufReader::new(io::stdin());
            let mut output = io::stdout();
            let mut error = io::stderr();
            let mut index = 0;
            while let Some(body) = read_frame(&mut input)? {
                writeln!(error, "DIAG {index}")?;
                error.flush()?;
                write_frame(
                    &mut output,
                    if mode == "lsp-bad" { b"wrong" } else { &body },
                )?;
                index += 1;
            }
            writeln!(error, "EOF")?;
            error.flush()?;
            Ok(7)
        }
        "output" => {
            let mut output = io::stdout();
            let mut error = io::stderr();
            for _ in 0..16 {
                output.write_all(&[b'A'; 8192])?;
                output.flush()?;
                error.write_all(&[b'B'; 8192])?;
                error.flush()?;
            }
            Ok(9)
        }
        "wait" => {
            let mut bytes = Vec::new();
            io::stdin().read_to_end(&mut bytes)?;
            check(bytes.is_empty(), "wait fixture received unexpected bytes")?;
            Ok(0)
        }
        "sleep" => {
            thread::sleep(Duration::from_secs(2));
            Ok(0)
        }
        "args" => {
            for argument in env::args().skip(3) {
                println!("{argument:?}");
            }
            Ok(0)
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unknown child mode",
        )),
    }
}
fn lsp(expect_invalid: bool) -> io::Result<String> {
    let mut child = self_command(if expect_invalid { "lsp-bad" } else { "lsp" })?
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut input = child.stdin.take().expect("piped stdin");
    let mut output = BufReader::new(child.stdout.take().expect("piped stdout"));
    let mut error = child.stderr.take().expect("piped stderr");
    let drain = thread::spawn(move || -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        error.read_to_end(&mut bytes)?;
        Ok(bytes)
    });
    let exchange = (|| -> io::Result<()> {
        for body in [
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"text\":\"Zażółć\"}",
            "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"shutdown\"}",
        ] {
            // Fragment both headers and UTF-8 payloads, rather than relying on
            // a single pipe write preserving a complete protocol message.
            let mut bytes = Vec::new();
            write_frame(&mut bytes, body.as_bytes())?;
            for fragment in bytes.chunks(3) {
                input.write_all(fragment)?;
                input.flush()?;
            }
            check(
                read_frame(&mut output)?.as_deref() == Some(body.as_bytes()),
                "response mismatch",
            )?;
        }
        Ok(())
    })();
    drop(input); // EOF is the fixture's graceful shutdown request.
    let eof = if exchange.is_ok() {
        read_frame(&mut output).and_then(|frame| check(frame.is_none(), "expected stdout EOF"))
    } else {
        Ok(())
    };
    drop(output);
    // Always reap and join before returning a protocol or I/O error.
    let status = child.wait();
    let diagnostics = drain.join();
    let status = status?;
    let diagnostics = diagnostics.map_err(|_| io::Error::other("stderr thread panicked"))??;
    check(status.code() == Some(7), "child exit status mismatch")?;
    if expect_invalid {
        check(exchange.is_err(), "invalid response was accepted")?;
        check(
            diagnostics == b"DIAG 0\nEOF\n",
            "invalid-response stderr mismatch",
        )?;
        return Ok(
            "result=pass invalid_response_rejected=true child_exit=7 reader_joined=true".into(),
        );
    }
    exchange?;
    eof?;
    check(
        diagnostics == b"DIAG 0\nDIAG 1\nEOF\n",
        "stderr was mixed or incomplete",
    )?;
    Ok(
        "result=pass frames=2 separate_stderr=true stdout_eof=true child_exit=7 reader_joined=true"
            .into(),
    )
}
fn run_case(case: &str, report: Option<&str>) -> io::Result<String> {
    let checkpoint = |stage| -> io::Result<()> {
        if let Some(path) = report {
            std::fs::write(format!("{path}.{stage}"), stage)?;
        }
        Ok(())
    };
    match case {
        "status" => {
            use std::os::unix::process::ExitStatusExt;
            for code in [0, 1, 7, 9, 20, 127, 255] {
                let raw = if cfg!(target_os = "aros") {
                    code
                } else {
                    code << 8
                };
                let status = std::process::ExitStatus::from_raw(raw);
                check(
                    status.code() == Some(code),
                    "raw exit status decoding mismatch",
                )?;
                check(status.success() == (code == 0), "exit success mismatch")?;
            }
            Ok("result=pass exit_status_codes=0,1,7,9,20,127,255".into())
        }
        "lsp" => lsp(false),
        "lsp-invalid" => lsp(true),
        "output" => {
            let output = self_command("output")?.output()?;
            check(output.status.code() == Some(9), "output fixture status")?;
            check(
                output.stdout.len() == 131072 && output.stdout.iter().all(|&b| b == b'A'),
                "stdout flood mismatch",
            )?;
            check(
                output.stderr.len() == 131072 && output.stderr.iter().all(|&b| b == b'B'),
                "stderr flood mismatch",
            )?;
            Ok("result=pass stdout_bytes=131072 stderr_bytes=131072 child_exit=9".into())
        }
        "wait" => {
            checkpoint("before-spawn")?;
            let mut child = self_command("wait")?
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            let _ = checkpoint("after-spawn");
            let input = child.stdin.take().unwrap();
            let running = child.try_wait();
            let _ = checkpoint("after-try-wait");
            drop(input);
            let _ = checkpoint("after-stdin-close");
            let first = child.wait()?;
            checkpoint("after-wait")?;
            check(
                running?.is_none(),
                "try_wait reported exit while stdin remains open",
            )?;
            let second = child.wait()?;
            check(
                first.success() && first == second && child.try_wait()? == Some(first),
                "wait caching/status mismatch",
            )?;
            Ok(
                "result=pass try_wait_running=true stdin_eof=true repeated_wait=true child_exit=0"
                    .into(),
            )
        }
        "kill" => {
            let mut child = self_command("sleep")?
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            let killed = child.kill();
            let status = child.wait()?;
            match killed {
                Ok(()) => {
                    check(
                        !status.success(),
                        "kill claimed success but child exited naturally",
                    )?;
                    Ok(format!(
                        "result=pass kill=supported child_status={status} reaped=true"
                    ))
                }
                Err(error) => {
                    check(
                        status.success(),
                        "self-bounded child did not exit normally after refused kill",
                    )?;
                    Ok(format!(
                        "result=unsupported kill_error={error:?} natural_exit=0 reaped=true"
                    ))
                }
            }
        }
        "spawn-error" => {
            let missing = env::current_exe()?.with_file_name("rust-process-fixture-does-not-exist");
            for _ in 0..8 {
                match Command::new(&missing)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    Err(error) => check(
                        error.kind() == io::ErrorKind::NotFound,
                        &format!("missing executable error kind: {error:?}"),
                    )?,
                    Ok(mut child) => {
                        let _ = child.wait();
                        return Err(io::Error::other("missing executable spawned"));
                    }
                }
            }
            let output = self_command("args")?.output()?;
            check(output.status.success(), "spawn after errors failed")?;
            Ok("result=pass missing_executable_errors=8 next_spawn=true".into())
        }
        "args" => {
            let values = ["two words", "quote\"inside", "Zażółć", "star*value", ""];
            let output = self_command("args")?.args(values).output()?;
            let expected: String = values.iter().map(|value| format!("{value:?}\n")).collect();
            check(output.status.success(), "argument fixture status")?;
            check(
                output.stdout == expected.as_bytes(),
                &format!(
                    "argument bytes did not round trip: expected={expected:?} actual={:?}",
                    String::from_utf8_lossy(&output.stdout)
                ),
            )?;
            check(
                output.stderr.is_empty(),
                "unexpected argument fixture stderr",
            )?;
            Ok("result=pass spaced_quoted_unicode_star_empty_arguments=true".into())
        }
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "unknown case")),
    }
}
fn main() -> ExitCode {
    let arguments: Vec<_> = env::args().collect();
    if arguments.get(1).map(String::as_str) == Some("--child") {
        return match child(arguments.get(2).map(String::as_str).unwrap_or("")) {
            Ok(code) => ExitCode::from(code),
            Err(error) => {
                eprintln!("child error: {error}");
                ExitCode::from(20)
            }
        };
    }
    let value = |name| {
        arguments
            .windows(2)
            .find(|pair| pair[0] == name)
            .map(|pair| pair[1].as_str())
    };
    let case = value("--case").unwrap_or("lsp");
    let report = value("--report");
    if let Some(path) = report {
        if let Err(error) =
            std::fs::write(format!("{path}.start"), format!("CASE_START case={case}\n"))
        {
            eprintln!("start report: {error}");
            return ExitCode::from(20);
        }
    }
    let result = run_case(case, report);
    let text = match &result {
        Ok(detail) => format!("CASE_DONE case={case} {detail}\n"),
        Err(error) => format!("CASE_FAILED case={case} error={error:?}\n"),
    };
    if let Some(path) = report {
        if let Err(error) = std::fs::write(path, &text) {
            eprintln!("final report: {error}");
            return ExitCode::from(20);
        }
    }
    print!("{text}");
    if result.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(20)
    }
}
