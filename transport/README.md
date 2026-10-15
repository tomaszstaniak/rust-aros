# Blocking Content-Length transport

A small, dependency-free framing layer for synchronous LSP processes.
It transports bytes; the caller owns JSON parsing and child-process lifecycle.

Use a persistent reader and writer for each session:

    let mut input = FrameReader::new(raw_input, ReadPolicy::DosPipe);
    let mut output = FrameWriter::new(raw_output);
    output.write_frame(b"{}")?;
    let response = input.read_frame()?;

Import these types from aros_lsp_transport. Buffered reads are available with
ReadPolicy::Buffered. DosPipe uses one-byte header reads, then exact body reads
of the declared size. This avoids requesting 8 KiB from an open DOS pipe that
waits for the whole requested amount. The input must itself be unbuffered:
wrapping std::io::Stdin does not bypass its internal buffer. The WAT port owns
descriptor 0 exclusively through File on AROS.

Defaults limit a complete header to 8192 bytes and a body to 1 MiB. Limits::new
configures both reader and writer. CRLF and exactly one decimal Content-Length
are required; unknown headers are accepted. EOF before any new header is clean;
partial headers or bodies are errors. Any read/write/flush error is terminal
for that reader/writer. Keep the same objects across messages; a new object
cannot remember a previous partial-frame failure.

The compatibility free functions exist for the process fixture. Their caller
must stop after any error. No background thread, timeout, cancellation, async
reactor or generic PIPE repair is supplied. DosPipe is an explicit policy for
the tested blocking protocol, not a replacement for buffered file I/O.

Run host tests from the repository root:

    cargo +stable test --manifest-path transport/Cargo.toml
