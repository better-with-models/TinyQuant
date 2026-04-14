# rust/crates/tinyquant-sys/include

This directory contains `tinyquant.h`, the cbindgen-generated C header for the `tinyquant-sys` ABI. The file is produced automatically by `build.rs` when `cargo build -p tinyquant-sys` runs; do not edit it by hand. CI enforces that the committed file matches the build output via `git diff --exit-code rust/crates/tinyquant-sys/include/tinyquant.h`.

## What lives here

- `tinyquant.h` — the complete public C ABI surface: the `TinyQuantCompressionPolicy` and `TinyQuantErrorKind` enums, the `TinyQuantError` struct, all opaque handle typedefs, and every `tq_*` function declaration. The header opens with a `TINYQUANT_H_VERSION` macro substituted by `build.rs` from `CARGO_PKG_VERSION`.

## How this area fits the system

C consumers (embedding applications, Python `ctypes` fallbacks, or FFI tests) include this header directly. The header is regenerated on every `cargo build -p tinyquant-sys`; `build.rs` uses cbindgen with the settings in `../cbindgen.toml` and then post-processes the output to replace the `@version@` placeholder. The committed copy must stay byte-identical to the generated copy — CI fails on any diff.

## Common edit paths

Do not edit `tinyquant.h` manually. To update it:

1. Modify the Rust source in `../src/` or the `../cbindgen.toml` configuration.
2. Run `cargo build -p tinyquant-sys` from `rust/`.
3. Commit the regenerated `tinyquant.h` alongside the source change.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
