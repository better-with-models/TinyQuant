# Changelog — tinyquant

All notable changes to this package will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this package adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.0.0] - 2026-04-15

### Added

- Initial public release on npm.
- napi-rs N-API bindings (Node ≥ 20.10, Bun ≥ 1.1) exposing
  `CodecConfig`, `Codebook`, `CompressedVector`, `RotationMatrix`,
  `Codec`, `Corpus`, `CompressionPolicy`, `VectorEntry`,
  `BruteForceBackend`, and `SearchResult` as camelCase classes with
  hand-written JSDoc.
- `TinyQuantError.fromNative` parses Rust message-prefix class names
  so structured error handling survives the FFI boundary.
- Dual ESM / CJS distribution (`dist/index.js`, `dist/index.cjs`,
  `.d.ts`, `.d.cts`) with source maps.
- Runtime binary loader resolving pre-built `.node` binaries from
  `binaries/` for: `x86_64-unknown-linux-gnu`,
  `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`,
  `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`.
