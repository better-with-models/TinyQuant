# Changelog — tinyquant-sys

All notable changes to this crate will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this crate adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.0.0] - 2026-04-15

### Added

- Initial public release on crates.io.
- C ABI façade exposing the TinyQuant codec as a `cdylib` and
  `staticlib` for FFI consumers.
- `cbindgen`-generated `include/tinyquant.h` header.
- `panic-probe` feature for testing panic-across-FFI-boundary
  behaviour.
- `simd` feature forwarding to `tinyquant-core` and `tinyquant-io`.
