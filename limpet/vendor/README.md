# Limpet SDK source snapshot

SDK 0.3.0, tag sdk-v0.3.0 of tomaszstaniak/limpet (local aros-xterm repository),
commit 6fa16a87fcdcc75f0790e967242f5485fb1e4ffc. Headers, format/query sources and
the host DOS type stand-in were copied byte-for-byte with git show. The project
MIT licence is in LICENSE.txt. No SDK implementation is rewritten in Rust.

The host build compiles only the pure formatting/decoding implementation. The
AROS build adds the SDK query/short-write transport and csrc/output.c, compiled
against the selected AROS SDK. Host tests do not establish AROS transport works.
