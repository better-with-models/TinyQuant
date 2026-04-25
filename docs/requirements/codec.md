---
title: Codec Requirements — Compression, Decompression, Serialization
tags:
  - requirements
  - planguage
  - codec
date-created: 2026-04-15
status: draft
category: requirements
---

# Codec Requirements

Functional requirements for the TinyQuant codec write path (compression),
read path (decompression), and binary serialization. These requirements
bind both the Python reference implementation and the Rust port.

---

## Compression

---

### FR-COMP-001 — Single-vector compression

```
Gist:       Compressing a valid FP32 vector produces a CompressedVector.
Type:       Function
Actor:      Library consumer
Function:   Given a CodecConfig C, a Codebook B trained for C, and an FP32
            vector V of dimension C.dim, the codec shall produce a
            CompressedVector CV without raising an error.
Scale:      Fraction of valid (C, B, V) triples that complete successfully.
Meter:      Call compress(V, C, B) for 10 000 randomly sampled valid triples.
            Count the fraction returning a CompressedVector (not an error).
Must:       100%
Plan:       100%
Qualify:    V.dim == C.dim AND B is trained with C AND all elements of V are
            finite (not NaN, not Inf).
Rationale:  Every valid input must compress; silent failures are unacceptable
            in a storage codec.
Tests:      [Python] tests/codec/test_codec.py::TestCompress::test_compress_returns_compressed_vector
            [Rust]   rust/crates/tinyquant-core/tests/codec_service.rs::compress_single_vector
Gap:        None.
```

---

### FR-COMP-002 — Index range

```
Gist:       Every quantized index in the CompressedVector is in [0, 2^bit_width).
Type:       Function
Actor:      Library consumer
Function:   After compression, every element of CV.indices shall be in the
            half-open interval [0, 2^C.bit_width).
Scale:      Fraction of indices across all CompressedVectors in the test set
            that are within the valid range.
Meter:      Compress 10 000 vectors with bit_width in {2, 4, 8}; for each
            CompressedVector inspect every index value.
            Report fraction in [0, 2^bit_width).
Must:       100%
Plan:       100%
Qualify:    FR-COMP-001 applies.
Rationale:  Out-of-range indices produce undefined behavior during codebook
            lookup at decompression time.
Tests:      [Python] tests/codec/test_codec.py::TestCompress::test_compress_indices_in_valid_range
            [Rust]   rust/crates/tinyquant-core/tests/codebook.rs::quantize_indices_always_in_codebook_across_random_inputs
            [Rust]   rust/crates/tinyquant-core/tests/quantize.rs::test_quantize_returns_valid_indices
Gap:        None.
```

---

### FR-COMP-003 — Deterministic compression

```
Gist:       The same inputs always produce byte-identical CompressedVectors.
Type:       Constraint
Actor:      Library consumer
Function:   Given identical (V, C, B) triples, two separate calls to compress
            shall return CompressedVectors whose serialized bytes are identical.
Scale:      Fraction of repeated-input pairs that produce byte-identical output.
Meter:      For 1 000 (V, C, B) triples, call compress twice each and compare
            CV1.to_bytes() == CV2.to_bytes().
Must:       100%
Plan:       100%
Qualify:    Both calls run in the same process on the same hardware with the
            same rotation cache state.
Rationale:  Determinism enables reproducible storage, content-addressable
            caching, and parity testing against the Python reference.
Tests:      [Python] tests/codec/test_codec.py::TestCompress::test_compress_deterministic
            [Python] tests/calibration/test_determinism.py::test_compress_deterministic_across_calls
            [Rust]   rust/crates/tinyquant-core/tests/batch_determinism.rs::full_pipeline_deterministic_across_repeat_runs
            [Rust]   rust/crates/tinyquant-core/tests/batch_determinism.rs::full_pipeline_deterministic_across_thread_counts
Gap:        None.
```

---

### FR-COMP-004 — Residual field present when enabled

```
Gist:       When residual_enabled is true, the CompressedVector carries a
            non-empty residual payload.
Type:       Function
Actor:      Library consumer
Function:   If C.residual_enabled is true, CV.residual shall be non-null and
            its length shall equal C.dim * 2 (bytes — FP16 per element).
Scale:      Fraction of compress calls with residual_enabled true that produce
            a non-null residual of the correct length.
Meter:      Compress 1 000 vectors with residual_enabled true; inspect
            CV.residual.
Must:       100%
Plan:       100%
Qualify:    FR-COMP-001 applies AND C.residual_enabled is true.
Rationale:  The residual payload is the mechanism for Stage-2 correction;
            its absence silently degrades reconstruction quality.
Tests:      [Python] tests/codec/test_codec.py::TestCompress::test_compress_with_residual
            [Rust]   rust/crates/tinyquant-core/tests/compressed_vector.rs::new_with_residual_payload
Gap:        GAP-COMP-004 — No Rust integration test verifies CV.residual.len() == dim * 2
            after a full compress() call. The Rust unit test constructs the type directly,
            bypassing the compress path. See testing-gaps.md §GAP-COMP-004.
```

---

### FR-COMP-005 — Residual field absent when disabled

```
Gist:       When residual_enabled is false, the CompressedVector has no
            residual payload.
Type:       Function
Actor:      Library consumer
Function:   If C.residual_enabled is false, CV.residual shall be null
            (absent/None).
Scale:      Fraction of compress calls with residual_enabled false that
            produce a null residual field.
Meter:      Compress 1 000 vectors with residual_enabled false; inspect
            CV.residual.
Must:       100%
Plan:       100%
Qualify:    FR-COMP-001 applies AND C.residual_enabled is false.
Rationale:  Unexpected residual data wastes storage and could mislead
            consumers that inspect the field.
Tests:      [Python] tests/codec/test_codec.py::TestCompress::test_compress_without_residual
            [Rust]   rust/crates/tinyquant-core/tests/compressed_vector.rs::has_residual_flag_accuracy
Gap:        None.
```

---

### FR-COMP-006 — Dimension mismatch rejection

```
Gist:       Compressing a vector whose dimension does not match the config
            raises a dimension mismatch error and produces no output.
Type:       Function
Actor:      Library consumer
Function:   If V.dim ≠ C.dim, compress(V, C, B) shall raise
            CodecError::DimensionMismatch (or equivalent) and shall not
            return a CompressedVector.
Scale:      Fraction of mismatched (V, C) pairs that raise the correct error
            without producing a partial result.
Meter:      Call compress with V.dim in {C.dim - 1, C.dim + 1, 0, 2×C.dim}
            for 100 randomly chosen C; confirm error type and no CV produced.
Must:       100%
Plan:       100%
Qualify:    V.dim ≠ C.dim.
Rationale:  Silent truncation or padding would corrupt the rotation matrix
            application and produce garbage output with no observable failure.
Tests:      [Python] tests/codec/test_codec.py::TestCompress::test_compress_dimension_mismatch_raises
            [Rust]   rust/crates/tinyquant-core/tests/errors.rs::dimension_mismatch_error_format
Gap:        GAP-COMP-006 — The Rust test validates error display format only. No Rust
            integration test calls compress() with a mismatched dimension and verifies
            both CodecError::DimensionMismatch is raised and no CompressedVector is
            returned. See testing-gaps.md §GAP-COMP-006.
```

---

### FR-COMP-007 — Config hash embedded

```
Gist:       Every CompressedVector carries a config_hash linking it to the
            CodecConfig used to create it.
Type:       Function
Actor:      Library consumer
Function:   CV.config_hash shall equal CodecConfig.hash() for the config C
            used during compression.
Scale:      Fraction of CompressedVectors whose config_hash matches the
            config hash of the CodecConfig used to create them.
Meter:      Compress 1 000 vectors; for each CV assert
            CV.config_hash == C.hash().
Must:       100%
Plan:       100%
Qualify:    FR-COMP-001 applies.
Rationale:  Config traceability is the guard against decompressing a
            CompressedVector with the wrong codebook or rotation matrix.
Tests:      [Python] tests/codec/test_codec.py::TestCompress::test_compress_config_hash_matches
            [Rust]   rust/crates/tinyquant-core/tests/codec_config.rs::test_config_hash_is_deterministic
Gap:        GAP-COMP-007 — No Rust integration test asserts CV.config_hash == C.hash()
            after a full compress() call. The codec_config test verifies hash stability
            in isolation, not the end-to-end embedding in a CompressedVector.
            See testing-gaps.md §GAP-COMP-007.
```

---

### FR-COMP-008 — Batch compression completeness

```
Gist:       Compressing a batch of N valid vectors produces exactly N
            CompressedVectors.
Type:       Function
Actor:      Library consumer
Function:   compress_batch(vectors, rows=N, cols=D, C, B) shall return a
            collection of exactly N CompressedVectors when all N input rows
            are valid.
Scale:      Fraction of batch calls where len(result) == N.
Meter:      Run compress_batch with batch sizes in {1, 10, 100, 1000, 10 000};
            assert len(result) == batch_size for each.
Must:       100%
Plan:       100%
Qualify:    All N input rows satisfy FR-COMP-001's qualify conditions.
Rationale:  A partial result would silently lose data; the caller cannot
            distinguish which vectors succeeded.
Tests:      [Python] tests/codec/test_codec.py::TestBatch::test_compress_batch_count_matches
            [Rust]   rust/crates/tinyquant-core/tests/codec_service.rs::batch_compress_matches_individual
            [Rust]   rust/crates/tinyquant-core/tests/batch_parallel.rs::compress_batch_serial_matches_codec_compress
Gap:        None.
```

---

## Decompression

---

### FR-DECOMP-001 — Decompression produces FP32 vector of original dimension

```
Gist:       Decompressing a CompressedVector returns an FP32 vector with the
            same dimension as the original input.
Type:       Function
Actor:      Library consumer
Function:   decompress(CV, C, B) shall return an FP32 vector R such that
            len(R) == C.dim and every element of R is a finite float32.
Scale:      Fraction of decompress calls that return a vector of the correct
            dimension with all-finite values.
Meter:      Compress and then decompress 10 000 vectors; inspect len(R) and
            finiteness of each element.
Must:       100%
Plan:       100%
Qualify:    CV was produced by FR-COMP-001 with the same C and B that are
            supplied to decompress.
Rationale:  A consumer that receives a vector of the wrong dimension or
            containing NaN cannot safely use it for similarity search.
Tests:      [Python] tests/codec/test_codec.py::TestDecompress::test_decompress_returns_fp32_array
            [Python] tests/codec/test_codec.py::TestDecompress::test_decompress_correct_dimension
            [Rust]   rust/crates/tinyquant-core/tests/codec_service.rs::decompress_compressed_vector
            [Rust]   rust/crates/tinyquant-core/tests/codec_service.rs::round_trip_preserves_shape
Gap:        None.
```

---

### FR-DECOMP-002 — Deterministic decompression

```
Gist:       The same CompressedVector, config, and codebook always yield
            byte-identical FP32 output.
Type:       Constraint
Actor:      Library consumer
Function:   Two calls to decompress(CV, C, B) with identical arguments shall
            return FP32 vectors that are bit-for-bit equal.
Scale:      Fraction of repeated-input pairs that produce identical FP32
            output.
Meter:      For 1 000 CompressedVectors, call decompress twice each;
            assert R1 == R2 element-wise using exact float comparison.
Must:       100%
Plan:       100%
Qualify:    Same CV, C, B in both calls; same process, same hardware.
Rationale:  Non-deterministic decompression breaks caching, testing, and
            any downstream system that expects reproducible vectors.
Tests:      [Python] tests/codec/test_codec.py::TestDecompress::test_decompress_deterministic
            [Rust]   rust/crates/tinyquant-core/tests/batch_determinism.rs::full_pipeline_deterministic_across_repeat_runs
Gap:        None.
```

---

### FR-DECOMP-003 — Config mismatch rejection

```
Gist:       Decompressing with a config whose hash does not match the
            CompressedVector raises a config mismatch error.
Type:       Function
Actor:      Library consumer
Function:   If CV.config_hash ≠ C.hash(), decompress(CV, C, B) shall raise
            CodecError::ConfigMismatch (or equivalent) and shall not return
            a partial FP32 vector.
Scale:      Fraction of mismatched (CV, C) pairs that raise the correct error.
Meter:      Produce 100 CompressedVectors under config C1; attempt to
            decompress each with a different config C2 (distinct seed or
            bit_width). Confirm error type and no output.
Must:       100%
Plan:       100%
Qualify:    CV.config_hash ≠ C.hash().
Rationale:  Applying the wrong rotation matrix or codebook produces
            plausible-looking garbage; the mismatch must be caught.
Tests:      [Python] tests/codec/test_codec.py::TestDecompress::test_decompress_config_mismatch_raises
            [Rust]   rust/crates/tinyquant-core/tests/errors.rs::config_mismatch_error
Gap:        GAP-DECOMP-003 — The Rust test validates error display format only. No Rust
            integration test supplies a CV with hash H1 and attempts decompress with config
            hash H2, verifying CodecError::ConfigMismatch and no output produced.
            See testing-gaps.md §GAP-DECOMP-003.
```

---

### FR-DECOMP-004 — Residual correction is applied

```
Gist:       When the CompressedVector contains a residual payload,
            decompression applies it to reduce reconstruction error.
Type:       Function
Actor:      Library consumer
Function:   For a vector V compressed with residual_enabled true, the
            mean-squared reconstruction error of decompress(CV_residual)
            shall be strictly less than that of decompress(CV_no_residual)
            where CV_no_residual is the same vector compressed without
            residuals.
Scale:      Fraction of tested vectors where residual-on MSE < residual-off MSE.
Meter:      Compress 1 000 vectors twice (residual on / off); compute MSE for
            each pair; report fraction where MSE_on < MSE_off.
Must:       95%
Plan:       99%
Rationale:  The residual stage exists to improve fidelity; if it does not
            improve the result in the majority of cases the implementation
            is incorrect.
Tests:      [Python] tests/codec/test_codec.py::TestRoundTrip::test_round_trip_with_residual_has_lower_error
            [Python] tests/calibration/test_score_fidelity.py::test_residual_improves_rho
            [Rust]   rust/crates/tinyquant-core/tests/residual.rs::residual_round_trip
            [Rust]   rust/crates/tinyquant-bench/tests/calibration.rs::* (#[ignore])
Gap:        GAP-DECOMP-004 — No non-ignored Rust test computes MSE for residual-on vs
            residual-off on the same inputs. The only Rust quality coverage is #[ignore].
            See testing-gaps.md §GAP-DECOMP-004.
Ref:        FR-QUAL-002, FR-QUAL-003
```

---

### FR-DECOMP-005 — Batch decompression completeness

```
Gist:       Decompressing N CompressedVectors returns exactly N FP32 vectors.
Type:       Function
Actor:      Library consumer
Function:   decompress_batch_into(compressed, C, B, out) shall write exactly
            N FP32 vectors into `out` when given N valid CompressedVectors.
Scale:      Fraction of batch decompress calls where len(out) == N.
Meter:      Run decompress_batch with sizes {1, 10, 100, 1000, 10 000};
            assert output length equals input length.
Must:       100%
Plan:       100%
Qualify:    All N CompressedVectors satisfy FR-DECOMP-003's qualify condition
            (config hashes match C).
Rationale:  Symmetry with FR-COMP-008; partial output silently loses data.
Tests:      [Python] tests/codec/test_codec.py::TestBatch::test_decompress_batch_count_matches
            [Rust]   rust/crates/tinyquant-core/tests/batch_parallel.rs::decompress_batch_serial_matches_individual
Gap:        None.
```

---

## Serialization

---

### FR-SER-001 — Round-trip byte identity

```
Gist:       Serializing and then deserializing a CompressedVector recovers
            the original exactly.
Type:       Function
Actor:      Library consumer, persistence layer
Function:   from_bytes(to_bytes(CV)) shall produce a CV2 such that
            CV2.indices == CV.indices, CV2.residual == CV.residual,
            CV2.config_hash == CV.config_hash, CV2.dimension == CV.dimension,
            and CV2.bit_width == CV.bit_width.
Scale:      Fraction of CompressedVectors that survive the round trip with
            all fields byte-identical.
Meter:      Generate 10 000 CompressedVectors across bit_width {2, 4, 8} and
            residual {on, off}; serialize then deserialize each; assert field
            equality.
Must:       100%
Plan:       100%
Rationale:  The serialization format is the storage contract; lossy
            round-trips corrupt stored embeddings.
Tests:      [Rust]   rust/crates/tinyquant-io/tests/roundtrip.rs (11 parametrized cases:
            bw2/bw4/bw8 × dim768/dim1/dim17/dim1536 × residual on/off)
            [Python] tests/codec/test_compressed_vector.py (serialization round-trip)
Gap:        None.
```

---

### FR-SER-002 — Python reference parity

```
Gist:       The Rust to_bytes output is byte-identical to the Python reference
            for any supported (CodecConfig, Codebook, vector) triple.
Type:       Constraint
Actor:      CI parity gate
Function:   For any triple (C, B, V) supported by both implementations,
            Rust::compress(V, C, B).to_bytes()
            == Python::compress(V, C, B).to_bytes().
Scale:      Fraction of canonical parity triples where byte output matches.
Meter:      Run the 120 canonical parity triples defined in
            rust/crates/tinyquant-io/tests/fixtures/ through both
            implementations; compare bytes.
Must:       100%
Plan:       100%
Past:       N/A — constraint applied at Rust port inception.
Qualify:    The triple (C, B, V) is in the canonical parity fixture set.
Rationale:  Byte-level parity is required for the Rust port to be a drop-in
            replacement for the Python reference.
Tests:      [Rust]   rust/crates/tinyquant-io/tests/python_parity.rs::case_01 through case_10
            (10 byte-for-byte cross-language parity cases covering bw2/bw4/bw8,
            dim768/dim1/dim17/dim1536, residual on/off)
Gap:        None.
Authority:  Codec lead + IO lead
```

---

### FR-SER-003 — Zero-copy corpus reads

```
Gist:       A memory-mapped corpus file can be iterated without allocating
            per-vector.
Type:       Function
Actor:      Library consumer (batch read path)
Function:   A CompressedVectorView over an mmap'd region shall expose the
            same fields as a CompressedVector without performing heap
            allocations per vector during iteration.
Scale:      Peak heap allocation (bytes) during iteration of a 10 000-vector
            mmap'd corpus file.
Meter:      Iterate 10 000 CompressedVectorViews over a memory-mapped file
            while recording allocations with dhat or valgrind massif; report
            peak heap delta from baseline.
Must:       ≤ 4 KiB allocation growth per 1 000 vectors iterated (i.e., no
            per-vector heap allocation).
Plan:       0 bytes growth (stack-only per-vector access).
Past:       Python from_bytes allocates one NumPy array per vector (~768 × 4
            bytes per call).
Qualify:    Rust only (tinyquant-io mmap feature). Python is exempt.
Rationale:  Reading a gigabyte of compressed vectors must be SSD-limited,
            not allocator-limited.
Tests:      [Rust]   rust/crates/tinyquant-io/tests/zero_copy.rs::zero_copy_slice_indices_no_allocation
            [Rust]   rust/crates/tinyquant-io/tests/mmap_corpus.rs::mmap_corpus_memory_mapped_not_copied
            [Rust]   rust/crates/tinyquant-io/tests/mmap_corpus.rs::mmap_corpus_zero_copy_slicing
            [CI]     rust-ci.yml::test-mmap-dhat (cargo test -p tinyquant-io --features "mmap dhat-heap")
Gap:        GAP-SER-003 — The tests assert structural no-allocation but do not enforce
            the ≤ 4 KiB/1 000 vectors Must threshold numerically. The dhat-heap CI job
            exists but does not assert the bound from the heap output file.
            See testing-gaps.md §GAP-SER-003.
```

---

## See also

- [[requirements/quality|Score Fidelity Requirements]]
- [[requirements/performance|Performance Requirements]]
- [[design/behavior-layer/codec-compression|Codec Compression Behavior]]
- [[design/behavior-layer/codec-decompression|Codec Decompression Behavior]]
- [[design/rust/numerical-semantics|Numerical Semantics]]
