# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-sys/src`

This directory contains all Rust source for the `tinyquant-sys` C ABI crate. Each module has a narrow, well-documented role: `codec_abi.rs` and `corpus_abi.rs` hold the `#[allow(unsafe_code)]` `extern "C"` entry points; `error.rs` defines `TinyQuantError` and the `tq_error_free` free function; `handle.rs` declares opaque handle types; `internal.rs` holds shared helpers that must not cross the C boundary. `lib.rs` re-exports the public surface and enforces the `TINYQUANT_H_VERSION` compile-time parity check. Any non-Rust FFI consumer and the cbindgen header generation depend on the stability of this module boundary.

## What this area contains

- primary responsibility: `lib.rs` (public re-exports, `TINYQUANT_H_VERSION` parity assertion), `codec_abi.rs` (codec `extern "C"` functions), `corpus_abi.rs` (corpus `extern "C"` functions and `TinyQuantCompressionPolicy`), `error.rs` (`TinyQuantError`, `TinyQuantErrorKind`, `tq_error_free`), `handle.rs` (opaque `CodebookHandle`, `CodecConfigHandle`, `CompressedVectorHandle`, `CorpusHandle`, `ByteBufferHandle`), `internal.rs` (internal helpers)
- main entrypoints: `lib.rs` for the module map and public re-exports; `codec_abi.rs` and `corpus_abi.rs` for the actual `extern "C"` surface
- common changes: adding `extern "C"` entry points in `codec_abi.rs` or `corpus_abi.rs`, updating handle types in `handle.rs`, bumping `TINYQUANT_H_VERSION` in `lib.rs` when the crate version changes

## Layout

```text
src/
├── codec_abi.rs
├── corpus_abi.rs
├── error.rs
├── handle.rs
├── internal.rs
├── lib.rs
└── README.md
```

## Common workflows

### Update existing behavior

1. Read the local README and the files you will touch before editing.
2. Follow the local invariants before introducing new files or abstractions.
3. Update nearby docs when the change affects layout, commands, or invariants.
4. Run the narrowest useful verification first, then the broader project gate.

### Add a new file or module

1. Confirm the new file belongs in this directory rather than a sibling.
2. Update the layout section if the structure changes in a way another agent must notice.
3. Add or refine local docs when the new file introduces a new boundary or invariant.

## Invariants — Do Not Violate

- keep this directory focused on its stated responsibility
- do not invent APIs, workflows, or invariants that the code does not support
- update this file when structure or safe-editing rules change

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../../AGENTS.md)
