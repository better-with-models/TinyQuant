# rust/crates/tinyquant-sys/src

This directory contains the Rust source for the `tinyquant-sys` C ABI facade. `lib.rs` is the crate root: it declares the top-level lint policy (`#![no_std]`, `#![deny(unsafe_code)]`), re-exports the public handle types and error structs, and hosts the compile-time version-parity assertion between `TINYQUANT_H_VERSION` and `CARGO_PKG_VERSION`. Unsafe code is confined to three files that carry an explicit narrow `#[allow(unsafe_code)]` at the top of each file.

## What lives here

- `lib.rs` — crate root; lint policy, public re-exports, and compile-time version assertion.
- `codec_abi.rs` — `extern "C"` entry points for the codec layer (`tq_codec_*`); `#[allow(unsafe_code)]` scoped to this file for `Box::into_raw` / `Box::from_raw` handle minting.
- `corpus_abi.rs` — `extern "C"` entry points for the corpus layer (`tq_corpus_*`); same narrow unsafe pattern.
- `error.rs` — `TinyQuantErrorKind` repr-C enum, `TinyQuantError` struct, `set_error`, `set_panic_error`, `tq_error_free`, and `tq_error_free_message`; `#[allow(unsafe_code)]` for `CString::from_raw`.
- `handle.rs` — opaque `#[repr(C)]` zero-sized handle types (`CodecConfigHandle`, `CodebookHandle`, `CompressedVectorHandle`, `CorpusHandle`, `ByteBufferHandle`); safe — no unsafe code.
- `internal.rs` — crate-private layout types (`CodecConfigBox`, etc.) shared between `codec_abi` and `corpus_abi` to prevent field-offset drift; safe.

## How this area fits the system

Every `tq_*` symbol exported by the shared library is defined in `codec_abi.rs` or `corpus_abi.rs`. `handle.rs` provides the type-erased pointer targets that C consumers hold as `FooHandle*`. `internal.rs` provides the concrete Rust types behind those pointers; keeping them in one place guarantees `codec_abi` and `corpus_abi` always agree on layout. `build.rs` (one level up) calls cbindgen over this directory to generate `include/tinyquant.h`.

## Common edit paths

- `codec_abi.rs` — most new entry points for codec operations.
- `corpus_abi.rs` — new entry points for corpus operations.
- `error.rs` — adding a new `TinyQuantErrorKind` variant (note: changing existing discriminants is a breaking change).
- `handle.rs` — adding a new opaque handle type when a new Rust object needs a C-visible pointer.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
