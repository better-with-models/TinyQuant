# Changelog — tinyquant-cpu

All notable changes to the `tinyquant-cpu` PyPI distribution will be
documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.0.0] - 2026-04-15

### Added

- First Rust-backed release. The `tinyquant_cpu` import namespace now
  re-exports the compiled `tinyquant_rs` extension built from the
  `tinyquant-py` crate, replacing the pure-Python implementation.
- Phase 24.1 developer shim under `src/tinyquant_cpu/` re-exports
  `tinyquant_rs` so editor tooling resolves the public API without
  requiring a built wheel.
- Fat-wheel assembly script at `scripts/packaging/assemble_fat_wheel.py`
  combines per-arch wheels into a single `py3-none-any` wheel.

## [0.1.1] - 2026-04-09

- Documentation and CI maintenance release.
- No user-facing API changes; codec, corpus, and backend modules are
  byte-identical to 0.1.0.

## [0.1.0] - 2026-04-08

- Initial release of the pure-Python implementation.
- Codec, Codebook, RotationMatrix, Corpus, BruteForceBackend,
  PgvectorAdapter, and domain error types.
