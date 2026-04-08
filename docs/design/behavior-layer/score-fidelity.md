---
title: Score Fidelity Behavior
tags:
  - design
  - behavior
  - bdd
  - quality
  - fidelity
date-created: 2026-04-08
status: active
category: design
source-count: 6
---

# Score Fidelity Behavior

> [!info] Capability
> Quantify how well TinyQuant preserves similarity scores through the
> compress-decompress round trip, giving consumers confidence that retrieval
> quality is acceptable.

## Actor

Quality validation process (calibration suite, CI pipeline, or integration
test).

## Preconditions

- A representative corpus of FP32 vectors (gold corpus)
- A trained CodecConfig and Codebook at the target bit width
- Original pairwise similarity scores computed on uncompressed vectors

---

## Scenarios

### Pearson correlation meets baseline

```gherkin
Feature: Score fidelity

  Scenario: 4-bit compression preserves Pearson rho above baseline
    Given a gold corpus of 10000 FP32 vectors at dimension 768
    And a CodecConfig with bit_width 4, residual_enabled true
    And original pairwise cosine similarities for a sample of 1000 pairs
    When all vectors are compressed and decompressed
    And pairwise cosine similarities are recomputed on decompressed vectors
    Then the Pearson correlation between original and decompressed scores
         is at least 0.995
```

> [!note] Threshold
> The research baseline targets ~0.997 Pearson rho. The scenario uses 0.995
> as a conservative lower bound. Exact thresholds should be calibrated against
> production embedding distributions.

### Rank preservation on top-k neighbors

```gherkin
  Scenario: Top-10 neighbor overlap is high for moderate corpus sizes
    Given a gold corpus of 10000 vectors
    And the original top-10 neighbors for 100 query vectors
    When all vectors are compressed and decompressed
    And top-10 neighbors are recomputed on decompressed vectors
    Then the average overlap with original top-10 is at least 80%
```

### Fidelity degrades gracefully with lower bit widths

```gherkin
  Scenario: 2-bit compression has lower fidelity than 4-bit
    Given the same gold corpus and query vectors
    When compressed at 2-bit and 4-bit respectively
    Then the 4-bit Pearson rho is higher than the 2-bit Pearson rho
    And the 2-bit Pearson rho is still above 0.95
```

### Residual correction improves score fidelity

```gherkin
  Scenario: Enabling residuals improves Pearson rho
    Given a gold corpus compressed at 4-bit with residuals disabled
    And the same corpus compressed at 4-bit with residuals enabled
    When Pearson rho is computed for both
    Then the residual-enabled rho is higher than the residual-disabled rho
```

### Passthrough policy preserves perfect fidelity

```gherkin
  Scenario: Passthrough vectors have perfect score fidelity
    Given a corpus with compression_policy "passthrough"
    When vectors are stored and retrieved
    Then Pearson rho between original and retrieved scores is 1.0
```

### Score fidelity is independent of corpus size

```gherkin
  Scenario: Fidelity does not degrade as corpus grows
    Given corpora of 1000, 10000, and 50000 vectors
    And the same CodecConfig and Codebook
    When Pearson rho is measured for each
    Then rho values are within 0.005 of each other
```

> [!warning] Top-k retrieval is different
> Score fidelity (pairwise correlation) is stable with corpus size, but
> **exact top-1 retrieval accuracy** degrades as corpus size grows. This is
> why TinyQuant does not claim compressed-domain search accuracy.

---

## Rules encoded by these scenarios

1. **Pearson baseline:** 4-bit + residuals meets a minimum correlation threshold
2. **Rank preservation:** neighbor overlap is high enough for approximate tasks
3. **Monotonic bit-width relationship:** more bits means better fidelity
4. **Residual benefit:** stage-2 correction measurably improves correlation
5. **Passthrough invariant:** uncompressed vectors preserve scores exactly
6. **Size independence:** per-pair fidelity does not depend on corpus size

## Automation level

These are calibration-level tests, likely run as part of CI or release
validation rather than on every commit. They require representative embedding
data and are sensitive to the training corpus used for codebook generation.

## See also

- [[behavior-layer/codec-decompression|Codec Decompression]]
- [[behavior-layer/corpus-management|Corpus Management]]
- [[two-stage-quantization-and-residual-correction]]
- [[TinyQuant]]
