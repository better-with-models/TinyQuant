---
title: Codec Decompression Behavior
tags:
  - design
  - behavior
  - bdd
  - codec
  - decompression
date-created: 2026-04-08
status: active
category: design
source-count: 6
---

# Codec Decompression Behavior

> [!info] Capability
> Reconstruct FP32 vectors from compressed representations, producing vectors
> suitable for similarity search by external backends.

## Actor

Library consumer or corpus read-path code.

## Preconditions

- A CompressedVector produced by the codec
- The same CodecConfig and Codebook used during compression

---

## Scenarios

### Decompress a single vector

```gherkin
Feature: Codec decompression

  Scenario: Decompress a CompressedVector back to FP32
    Given a CompressedVector produced with a known CodecConfig and Codebook
    When the codec decompresses the vector
    Then the result is an FP32 vector of the original dimension
```

### Deterministic decompression

```gherkin
  Scenario: Decompression is deterministic
    Given the same CompressedVector, CodecConfig, and Codebook
    When the codec decompresses twice
    Then both FP32 outputs are identical
```

### Config mismatch rejection

```gherkin
  Scenario: Reject decompression with a mismatched config
    Given a CompressedVector with config_hash "abc123"
    And a CodecConfig whose hash is "def456"
    When the codec attempts to decompress
    Then the operation fails with a config mismatch error
    And no FP32 vector is produced
```

### Round-trip fidelity

```gherkin
  Scenario: Compress-then-decompress preserves approximate similarity
    Given a CodecConfig with bit_width 4 and residual_enabled true
    And a pair of FP32 vectors A and B
    And their original cosine similarity is S_original
    When both vectors are compressed and then decompressed
    And the cosine similarity of the decompressed vectors is S_decompressed
    Then the absolute difference between S_original and S_decompressed is small
```

> [!note] Precision
> "Small" is deliberately informal here. The quantitative contract is defined
> in [[behavior-layer/score-fidelity|Score Fidelity]].

### Residual correction improves round-trip quality

```gherkin
  Scenario: Residual correction reduces reconstruction error
    Given a CodecConfig with residual_enabled true
    And another CodecConfig with residual_enabled false but otherwise identical
    And the same FP32 vector V
    When V is compressed and decompressed under both configs
    Then the reconstruction error with residuals is less than or equal to without
```

### Batch decompression

```gherkin
  Scenario: Decompress an entire corpus worth of vectors
    Given a Corpus containing 1000 CompressedVectors
    When the corpus is batch-decompressed
    Then 1000 FP32 vectors are returned
    And each has the expected dimension
```

---

## Rules encoded by these scenarios

1. **FP32 output:** decompression always returns standard FP32 vectors
2. **Determinism:** same inputs produce same outputs
3. **Config safety:** mismatched configs are caught before producing garbage output
4. **Residual benefit:** stage-2 correction measurably improves fidelity
5. **Batch correctness:** batch decompression produces the same results as single-vector decompression

## Automation level

Python API tests. Round-trip fidelity scenarios may use NumPy assertions with
configurable tolerances.

## See also

- [[behavior-layer/codec-compression|Codec Compression]]
- [[behavior-layer/score-fidelity|Score Fidelity]]
- [[domain-layer/aggregates-and-entities|Aggregates and Entities]]
- [[random-preconditioning-without-normalization-overhead]]
