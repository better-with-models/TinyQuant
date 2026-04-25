# Changelog — tinyquant-io

All notable changes to this crate will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this crate adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.0.0] - 2026-04-15

### Added

- Initial public release on crates.io.
- Binary serialisation and deserialisation for `CompressedVector`,
  `Codebook`, `CodecConfig`, and `RotationMatrix`.
- Memory-mapped file I/O (`mmap` feature, Linux/macOS/Windows).
- Optional locked-mmap support (`mmap-lock` feature).
- Rayon-parallel batch I/O helpers (`rayon` feature).
- Heap-profiling harness via `dhat` (`dhat-heap` feature).
