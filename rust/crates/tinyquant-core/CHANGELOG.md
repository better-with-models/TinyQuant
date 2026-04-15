# Changelog — tinyquant-core

All notable changes to this crate will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this crate adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.0.0] - 2026-04-15

### Added

- Initial public release on crates.io.
- `no_std`-compatible codec core: rotation preconditioning via QR
  decomposition, scalar quantization (2-bit / 4-bit / 8-bit),
  optional FP16 residual correction.
- `CodecConfig` with deterministic SHA-2 config hashing.
- `Codebook` training via uniform quantile estimation.
- `CompressedVector` with versioned bit-packed binary serialisation
  (LSB-first).
- `RotationMatrix` seeded via `rand_chacha`.
- `Corpus` aggregate root with domain-event emission
  (`CorpusCreated`, `VectorsInserted`, `CorpusDecompressed`,
  `CompressionPolicyViolationDetected`).
- Three compression policies: `COMPRESS`, `PASSTHROUGH`, `FP16`.
- Runtime SIMD dispatch (AVX2 / NEON / scalar fallback) behind the
  `simd` feature flag.
- `std` and `avx512` feature gates.
