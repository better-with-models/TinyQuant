# Changelog — tinyquant-py / tinyquant-rs

All notable changes to this crate (published as the `tinyquant-rs`
Python extension wheel) will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this crate adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.0.0] - 2026-04-15

### Added

- Initial public release on crates.io and as a Python wheel
  (`tinyquant-rs`).
- PyO3 abi3-py312 bindings exposing `CodecConfig`, `Codebook`,
  `CompressedVector`, `RotationMatrix`, `Codec`, `Corpus`,
  `CompressionPolicy`, `BruteForceBackend`, and `SearchResult`.
- NumPy array interop via `numpy` crate.
- `simd` feature forwarding SIMD dispatch to `tinyquant-core`.
- Importable as `import tinyquant_rs` from Python ≥ 3.12.
