# Changelog — tinyquant-bruteforce

All notable changes to this crate will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this crate adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.0.0] - 2026-04-15

### Added

- Initial public release on crates.io.
- `BruteForceBackend`: exhaustive cosine-similarity search over a
  corpus of `CompressedVector` entries.
- Runtime SIMD cosine kernel routed through `tinyquant-core`'s
  dispatch (`simd` feature).
- `SearchResult` type with vector id, score, and rank.
- `dashmap` feature for concurrent index updates.
