# Limpet SDK source snapshot

SDK 0.3.0, tag sdk-v0.3.0 of tomaszstaniak/limpet (local aros-xterm repository),
commit 6fa16a87fcdcc75f0790e967242f5485fb1e4ffc. Headers, format/query sources and
the host DOS type stand-in were copied byte-for-byte with git show. The project
MIT licence is in LICENSE.txt. No SDK implementation is rewritten in Rust.

The host build compiles only the pure formatting/decoding implementation. The
AROS build adds the SDK query/short-write transport and csrc/output.c, compiled
against the selected AROS SDK. Host tests do not establish AROS transport works.

Presentation capability constants updated from Limpet commit
fd5f4f4 (2026-10-21): explicit style bits, OSC8, and full eza 0.23.5
Nerd Font coverage plus one-cell width bits. Only the public protocol header
changes; the SDK format/query implementation and binary layout remain pinned
to the existing snapshot. The header also incorporates the previously bound
modified-Enter bit from SDK0.4.0. Older known-bit behavior remains tested.
