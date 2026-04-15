# Changelog — tinyquant-cli

All notable changes to this crate will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this crate adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.0.0] - 2026-04-15

### Added

- Initial public release on crates.io.
- `tinyquant` binary with subcommands: `codec train`, `codec compress`,
  `codec decompress`, `corpus ingest`, `corpus search`, `info`,
  `verify`.
- Shell completion generation via `clap_complete`.
- Man-page generation via `clap_mangen`.
- Rayon-parallel batch pipeline (`rayon` feature).
- Progress bars via `indicatif` (`progress` feature, on by default).
- Memory-mapped I/O (`mmap` feature, on by default).
- jemalloc allocator on non-MSVC targets (`jemalloc` feature, on by
  default).
- JSON tracing output (`tracing-json` feature).
- 8 MiB release-binary size budget enforced in CI.
