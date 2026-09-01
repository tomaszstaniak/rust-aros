#[cfg(target_os = "aros")]
pub fn read_output(
    out: ChildPipe,
    stdout: &mut Vec<u8>,
    err: ChildPipe,
    stderr: &mut Vec<u8>,
) -> io::Result<()> {
    // No poll() over pipes on AROS: drain stderr on a helper thread while this
    // thread drains stdout, then join. Blocking reads end when the child exits.
    let t = crate::thread::spawn(move || {
        let mut buf = Vec::new();
        let r = err.read_to_end(&mut buf);
        (r, buf)
    });
    let r_out = out.read_to_end(stdout);
    let (r_err, buf) = t
        .join()
        .map_err(|_| io::const_error!(io::ErrorKind::Other, "stderr reader thread panicked"))?;
    stderr.extend_from_slice(&buf);
    r_out.map(drop)?;
    r_err.map(drop)
}

#[cfg(not(target_os = "aros"))]
