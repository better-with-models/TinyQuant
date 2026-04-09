# Changelog

All notable changes to TinyQuant will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-04-08

### Added

- Codec module with compress/decompress pipeline: rotation
  preconditioning, scalar quantization, and optional residual correction
- CodecConfig immutable configuration with deterministic config hashing
- Codebook training via uniform quantile estimation
- CompressedVector with versioned binary serialization (bit-packed
  indices, LSB-first)
- RotationMatrix via QR decomposition of seeded random matrix
- Batch compress/decompress operations
- Corpus aggregate root with insert, decompress, and domain event
  emission
- Three compression policies: COMPRESS, PASSTHROUGH, FP16
- Domain events: CorpusCreated, VectorsInserted, CorpusDecompressed,
  CompressionPolicyViolationDetected
- BruteForceBackend with exhaustive cosine similarity search
- SearchBackend protocol for pluggable backend implementations
- PgvectorAdapter for PostgreSQL + pgvector wire format integration
- Custom domain errors: DimensionMismatchError, ConfigMismatchError,
  CodebookIncompatibleError, DuplicateVectorError
- Calibration test suite: score fidelity (VAL-01), compression ratio
  (VAL-02), determinism (VAL-03), research alignment (VAL-04)
- End-to-end test suite with 10 pipeline scenarios
- Architecture tests for dependency direction enforcement
- Property-based tests with Hypothesis
- Obsidian documentation wiki with design docs, behavior specs,
  validation plan, and research synthesis
