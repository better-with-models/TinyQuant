# Changelog — tinyquant-pgvector

All notable changes to this crate will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this crate adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.0.0] - 2026-04-15

### Added

- Initial public release on crates.io.
- `PgvectorAdapter`: wire-format bridge between `CompressedVector`
  and PostgreSQL + pgvector.
- Regex-based connection string parsing via `once_cell`.
- `live-db` feature for the real `postgres::Client` integration
  (requires Rust > 1.81 due to `getrandom 0.4` transitive dep).
- `test-containers` feature for integration tests via testcontainers.
