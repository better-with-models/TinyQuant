---
title: Quality Requirements — Score Fidelity and Compression Ratio
tags:
  - requirements
  - planguage
  - quality
  - fidelity
date-created: 2026-04-15
status: draft
category: requirements
---

# Quality Requirements

Score fidelity and compression ratio requirements for the TinyQuant codec.
These requirements bind both the Python reference implementation and the
Rust port. They are calibration-level gates: measured on the 10 000-vector
gold corpus of OpenAI `text-embedding-3-small` vectors (dimension 768)
unless a different fixture is specified.

---

### FR-QUAL-001 — Pearson ρ at 4-bit with residual

```
Gist:       4-bit compression with residual correction preserves cosine
            similarity scores at Pearson ρ ≥ 0.995.
Type:       Performance
Actor:      Quality validation process
Function:   For the 10 000-vector gold corpus compressed at bit_width 4
            with residual_enabled true, the Pearson correlation coefficient
            between original pairwise cosine similarities and decompressed
            pairwise cosine similarities shall meet the threshold below.
Scale:      Pearson ρ between original and decompressed cosine similarity
            vectors, computed on a sample of 1 000 randomly chosen pairs
            from the gold corpus.
Meter:      Run the calibration suite (cargo test -p tinyquant-bench
            --ignored -- calibration) against the 10k gold corpus;
            record the Pearson ρ output for 4-bit + residual.
Must:       ρ ≥ 0.995
Plan:       ρ ≥ 0.997
Stretch:    ρ ≥ 0.999
Past:       Python reference: ρ ≈ 0.997 (measured on gold corpus, 2026-04)
Qualify:    gold corpus = 10 000 OpenAI text-embedding-3-small vectors at
            dim 768; codebook trained on same corpus; 1 000 sampled pairs.
Rationale:  This is the primary fidelity gate for the production use case
            (4-bit + residual is the default codec configuration). Below
            0.995, approximate nearest-neighbor ranking degrades measurably.
Authority:  Codec lead
Ref:        [[design/behavior-layer/score-fidelity]]
```

---

### FR-QUAL-002 — Pearson ρ at 4-bit without residual

```
Gist:       4-bit compression without residual correction maintains Pearson
            ρ ≥ 0.98.
Type:       Performance
Actor:      Quality validation process
Function:   For the same gold corpus compressed at bit_width 4 with
            residual_enabled false, Pearson ρ shall meet the threshold below.
Scale:      Pearson ρ (same measurement procedure as FR-QUAL-001).
Meter:      Same calibration suite; record 4-bit no-residual result.
Must:       ρ ≥ 0.980
Plan:       ρ ≥ 0.985
Past:       Python reference: ρ ≈ 0.957 (bw=4, no residual, gold corpus)
Qualify:    Same gold corpus as FR-QUAL-001; residual_enabled false.
Rationale:  The no-residual path trades fidelity for storage density;
            0.98 preserves enough ranking quality for use cases that
            cannot afford the extra 2 bytes/element residual.
Ref:        FR-QUAL-001
```

---

### FR-QUAL-003 — Pearson ρ at 2-bit with residual

```
Gist:       2-bit compression with residual correction maintains Pearson
            ρ ≥ 0.95.
Type:       Performance
Actor:      Quality validation process
Function:   For the gold corpus compressed at bit_width 2 with
            residual_enabled true, Pearson ρ shall meet the threshold below.
Scale:      Pearson ρ (same procedure as FR-QUAL-001).
Meter:      Same calibration suite; record 2-bit + residual result.
Must:       ρ ≥ 0.950
Plan:       ρ ≥ 0.960
Past:       Python reference: ρ ≈ 0.95 (2-bit + residual, gold corpus)
Qualify:    Same gold corpus; bit_width 2; residual_enabled true.
Rationale:  2-bit is the maximum-compression setting; 0.95 is the floor
            below which similarity scores become unreliable for approximate
            retrieval.
Ref:        FR-QUAL-001
```

---

### FR-QUAL-004 — Top-10 neighbor overlap

```
Gist:       After compress–decompress, the average top-10 neighbor overlap
            with the original ranking is at least 80%.
Type:       Performance
Actor:      Quality validation process
Function:   For 100 query vectors sampled from the gold corpus, compute their
            true top-10 nearest neighbors in the original FP32 space and
            their approximate top-10 in the decompressed space. The mean
            Jaccard overlap across the 100 queries shall meet the threshold.
Scale:      Mean Jaccard overlap |true_top10 ∩ approx_top10| / 10, averaged
            over 100 query vectors.
Meter:      Run brute-force search on both original and decompressed corpora;
            compute overlap per query; report mean.
Must:       Mean overlap ≥ 80%
Plan:       Mean overlap ≥ 85%
Stretch:    Mean overlap ≥ 90%
Past:       Python reference: ~82% (measured on gold corpus, 2026-04)
Qualify:    4-bit + residual codec config; gold corpus; 100 query vectors
            chosen to cover diverse similarity ranges.
Rationale:  ANN quality is the ultimate downstream metric; a 4-bit codec
            that preserves 80% of top-10 neighbors is commercially useful
            for re-ranking pipelines.
Ref:        FR-QUAL-001
```

---

### FR-QUAL-005 — Residuals monotonically improve Pearson ρ

```
Gist:       For any given bit width, enabling residual correction produces
            higher Pearson ρ than disabling it.
Type:       Constraint
Actor:      Codec implementation
Function:   For any bit_width B in {2, 4, 8} and any representative corpus,
            Pearson_ρ(compress(residual=true)) shall be strictly greater than
            Pearson_ρ(compress(residual=false)).
Scale:      Sign of (ρ_residual_on - ρ_residual_off) per bit_width.
Meter:      Run calibration suite at each bit_width; report sign per width.
Must:       Positive sign (ρ_on > ρ_off) for all three bit widths.
Plan:       Same.
Qualify:    Same corpus and codebook used for both residual variants.
Rationale:  If residuals ever degrade fidelity, the Stage-2 implementation
            is incorrect. This requirement is a logical invariant, not a
            calibration threshold.
Ref:        FR-DECOMP-004
```

---

### FR-QUAL-006 — Passthrough fidelity is exact

```
Gist:       The "passthrough" compression policy preserves Pearson ρ = 1.000
            between original and retrieved similarity scores.
Type:       Constraint
Actor:      Quality validation process
Function:   For a Corpus with compression_policy "passthrough", pairwise
            cosine similarities computed on inserted vectors shall be
            identical to cosine similarities computed on retrieved vectors.
            Pearson ρ shall equal 1.000 within float64 arithmetic precision.
Scale:      Pearson ρ between original and retrieved similarity vectors.
Meter:      Insert 1 000 FP32 vectors into a passthrough corpus; compute 500
            pairwise cosine similarities before and after retrieval; measure ρ.
Must:       ρ = 1.000 (within 1e-12 floating-point tolerance)
Plan:       ρ = 1.000
Qualify:    compression_policy == "passthrough"; vectors are finite FP32.
Rationale:  The passthrough policy promises lossless storage; any fidelity
            loss is a bug.
Ref:        FR-CORP-005
```

---

### FR-QUAL-007 — Compression ratio at 4-bit

```
Gist:       4-bit compression without residual achieves at least 7× storage
            reduction versus raw FP32.
Type:       Performance
Actor:      Quality validation process
Function:   The size in bytes of a 4-bit no-residual CompressedVector,
            including all header and metadata, shall be no more than 1/7
            of the size of the original FP32 vector.
Scale:      Ratio: (FP32 size in bytes) / (CompressedVector.to_bytes() size
            in bytes).
Meter:      Measure to_bytes() size for 1 000 dim-768 vectors at bit_width 4,
            residual_enabled false. Compute ratio for each; report minimum.
Must:       Minimum ratio ≥ 7.0×
Plan:       Minimum ratio ≥ 7.5×
Past:       Python reference: ~7.5× for dim=768, bw=4, no residual.
Qualify:    bit_width 4; residual_enabled false; dimension 768.
Rationale:  The primary value proposition of TinyQuant is storage compression.
            Below 7× the codec is not competitive with simpler quantization
            schemes (e.g., int8 gives 4×).
Authority:  Codec lead
```

---

### FR-QUAL-008 — Compression ratio at 4-bit with residual

```
Gist:       4-bit compression with residual correction achieves at least
            5× storage reduction versus raw FP32.
Type:       Performance
Actor:      Quality validation process
Function:   The size in bytes of a 4-bit residual-on CompressedVector
            shall be no more than 1/5 of the original FP32 vector size.
Scale:      Same ratio as FR-QUAL-007, with residual_enabled true.
Meter:      Same procedure as FR-QUAL-007 with residual_enabled true.
Must:       Minimum ratio ≥ 5.0×
Plan:       Minimum ratio ≥ 5.5×
Past:       Python reference: ~5× for dim=768, bw=4, residual on (f16
            residual at 2 bytes/element costs ~2 bytes/element on top of
            the 0.5 bytes/element index storage).
Qualify:    bit_width 4; residual_enabled true; dimension 768.
Rationale:  The residual payload adds 2 bytes/element (FP16); at 5× we
            still achieve meaningful compression over raw FP32 (4 bytes/elem).
            Below 5× the quality gain from residuals may not justify the
            storage cost in latency-sensitive deployments.
Ref:        FR-QUAL-007
```

---

## See also

- [[requirements/codec|Codec Requirements]]
- [[requirements/performance|Performance Requirements]]
- [[design/behavior-layer/score-fidelity|Score Fidelity Behavior]]
- [[design/rust/goals-and-non-goals|Goals and Non-Goals]]
