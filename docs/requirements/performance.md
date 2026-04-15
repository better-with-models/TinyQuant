---
title: Performance Requirements — Latency and Throughput
tags:
  - requirements
  - planguage
  - performance
date-created: 2026-04-15
status: draft
category: requirements
---

# Performance Requirements

Latency and throughput requirements for the Rust port. All measurements
are taken on a single x86_64 core at a fixed clock speed against the
same fixture used for quality validation (OpenAI `text-embedding-3-small`,
dimension 1536, unless noted). "Past" values are the Python reference on
the same machine.

> [!note] Scope
> These requirements apply to the **Rust port** only. The Python
> reference is the `Past` baseline. Python has no `Must` or `Plan` for
> these numbers; its performance is documented as a calibration anchor.

---

### FR-PERF-001 — Single-vector compress latency

```
Gist:       Compressing a single FP32 vector (dim 1536, 4-bit + residual)
            completes in ≤ 18 µs at warm cache.
Type:       Performance
Actor:      Library consumer (single-vector path)
Function:   The p99 latency of compress(V, C, B) for dim 1536 shall not
            exceed the threshold below, measured at warm rotation cache.
Scale:      p99 wall-clock latency in µs per single compress call.
Meter:      Run 10 000 warm compress calls (rotation cache pre-filled);
            dimension 1536; bit_width 4; residual_enabled true.
            Record p99 using criterion or a tight Rust benchmark harness.
Must:       p99 ≤ 18 µs
Plan:       p99 ≤ 10 µs
Stretch:    p99 ≤ 6 µs
Past:       Python reference: ~180 µs (single thread, warm NumPy)
Qualify:    Rust Rust port; single thread; rotation cache warm; dim 1536;
            SIMD enabled (simd feature).
Rationale:  A 10× latency improvement enables TinyQuant to run inline in
            request-handling paths where Python's 180 µs would introduce
            visible tail latency.
Authority:  Perf lead
Ref:        [[design/rust/goals-and-non-goals]]
```

---

### FR-PERF-002 — Single-vector decompress latency

```
Gist:       Decompressing a single CompressedVector (dim 1536) completes
            in ≤ 9 µs at warm cache.
Type:       Performance
Actor:      Library consumer (single-vector read path)
Scale:      p99 wall-clock latency in µs per decompress call.
Meter:      10 000 warm decompress calls; same config as FR-PERF-001.
Must:       p99 ≤ 9 µs
Plan:       p99 ≤ 5 µs
Stretch:    p99 ≤ 3 µs
Past:       Python reference: ~95 µs
Qualify:    Same as FR-PERF-001.
Rationale:  Decompression is on the read path; it should not add more
            latency than a typical network round-trip (< 1 ms).
Ref:        FR-PERF-001
```

---

### FR-PERF-003 — Batch compress throughput

```
Gist:       Compressing 10 000 vectors (dim 1536) completes in ≤ 80 ms
            using Rayon parallelism.
Type:       Performance
Actor:      Library consumer (batch write path)
Scale:      Wall-clock elapsed time in ms for compress_batch(10 000 vectors).
Meter:      Median of 20 trials of compress_batch(10 000 × dim 1536,
            bit_width 4, residual true) using rayon_parallelism() on the
            host's physical core count.
Must:       Median ≤ 80 ms
Plan:       Median ≤ 40 ms
Stretch:    Median ≤ 25 ms
Past:       Python reference: ~1.9 s (single-threaded NumPy)
Qualify:    Rust port; Rayon thread pool sized to physical cores; SIMD
            enabled; dim 1536; rotation cache warm after first batch.
Rationale:  Batch throughput governs corpus ingestion rate. A 24× speedup
            over Python means a 1M-vector corpus index takes < 8 s instead
            of ~3 min.
Ref:        FR-PERF-001
```

---

### FR-PERF-004 — Batch decompress throughput

```
Gist:       Decompressing 10 000 vectors (dim 1536) completes in ≤ 40 ms.
Type:       Performance
Actor:      Library consumer (batch read path)
Scale:      Wall-clock elapsed time in ms for decompress_batch(10 000 vectors).
Meter:      Median of 20 trials; same setup as FR-PERF-003 but decompress.
Must:       Median ≤ 40 ms
Plan:       Median ≤ 20 ms
Stretch:    Median ≤ 12 ms
Past:       Python reference: ~0.95 s
Qualify:    Same as FR-PERF-003.
Ref:        FR-PERF-003
```

---

### FR-PERF-005 — `to_bytes` serialization latency

```
Gist:       Serializing a dim-1536 CompressedVector to bytes completes in
            ≤ 150 ns.
Type:       Performance
Actor:      Serialization layer, IO path
Scale:      Median wall-clock latency in ns per to_bytes call.
Meter:      10 000 to_bytes calls on a pre-constructed CompressedVector
            (dim 1536, 4-bit + residual); record median with criterion.
Must:       Median ≤ 150 ns
Plan:       Median ≤ 100 ns
Stretch:    Median ≤ 80 ns
Past:       Python reference: ~5 µs
Qualify:    Rust port; dim 1536; bit_width 4; residual on.
Rationale:  to_bytes is called on every vector written to disk or network.
            At ≤ 150 ns it is not the bottleneck in any realistic IO path.
```

---

### FR-PERF-006 — `from_bytes` deserialization latency

```
Gist:       Deserializing a dim-1536 CompressedVector from bytes completes
            in ≤ 200 ns.
Type:       Performance
Actor:      IO path, mmap read path
Scale:      Median wall-clock latency in ns per from_bytes call.
Meter:      10 000 from_bytes calls on pre-serialized byte buffers
            (dim 1536, 4-bit + residual); record median.
Must:       Median ≤ 200 ns
Plan:       Median ≤ 120 ns
Stretch:    Median ≤ 100 ns
Past:       Python reference: ~8 µs
Qualify:    Rust port; dim 1536; byte buffer already in L1/L2 cache.
Ref:        FR-PERF-005
```

---

### FR-PERF-007 — Codebook training latency

```
Gist:       Training a codebook from 100 000 scalar values completes in
            ≤ 5 ms.
Type:       Performance
Actor:      Codec consumer (training path)
Scale:      Wall-clock elapsed time in ms for Codebook::train(100k values,
            bit_width 4).
Meter:      Median of 10 trials of Codebook::train with 100 000 f32 values;
            bit_width 4.
Must:       Median ≤ 5 ms
Plan:       Median ≤ 2 ms
Stretch:    Median ≤ 1.5 ms
Past:       Python reference: ~45 ms
Qualify:    Rust port; 100 000 f32 values; bit_width 4 (16 codebook entries);
            single thread.
Rationale:  Codebook training is called once per corpus setup, not per
            vector, so the absolute threshold is more lenient than compress.
```

---

### FR-PERF-008 — Rotation matrix build (cold)

```
Gist:       Building a rotation matrix from scratch (dim 1536) completes
            in ≤ 35 ms.
Type:       Performance
Actor:      Codec (first compress call with an unseen seed+dim pair)
Scale:      Wall-clock elapsed time in ms for RotationMatrix::build(seed,
            dim=1536) on a cold cache.
Meter:      10 trials with distinct seeds; cache cleared between trials;
            record median.
Must:       Median ≤ 35 ms
Plan:       Median ≤ 20 ms
Stretch:    Median ≤ 15 ms
Past:       Python reference: ~110 ms (NumPy QR on dim=1536)
Qualify:    Rust port; dim 1536; cold rotation cache; faer QR.
Rationale:  The cold-build path is a one-time cost per (seed, dim) pair
            per process. At ≤ 35 ms it does not noticeably delay the
            first request in a server process.
```

---

### FR-PERF-009 — Rotation matrix cache hit latency

```
Gist:       Retrieving a previously built rotation matrix (warm cache) takes
            ≤ 40 ns.
Type:       Performance
Actor:      Codec (all compress calls after the first)
Scale:      Median latency in ns for a RotationCache hit (same seed + dim).
Meter:      After one cold build, run 100 000 cache lookups for the same
            (seed=42, dim=1536); record median using criterion.
Must:       Median ≤ 40 ns
Plan:       Median ≤ 25 ns
Stretch:    Median ≤ 20 ns
Past:       Python reference: ~400 ns (dict lookup + NumPy object overhead)
Qualify:    Rust port; ArcSwapOption hot-entry path; same (seed, dim) as
            the cached entry.
Rationale:  Every compress call hits the rotation cache; at ≤ 40 ns this
            path is not the bottleneck relative to the quantization kernel.
Ref:        [[design/rust/parallelism]] §Lock-free read paths
```

---

## See also

- [[requirements/quality|Score Fidelity Requirements]]
- [[requirements/gpu|GPU Performance Requirements]]
- [[design/rust/goals-and-non-goals|Goals and Non-Goals]]
- [[design/rust/benchmark-harness|Benchmark Harness]]
- [[design/rust/simd-strategy|SIMD Strategy]]
