---
title: Corpus Requirements — Management and Compression Policy
tags:
  - requirements
  - planguage
  - corpus
date-created: 2026-04-15
status: draft
category: requirements
---

# Corpus Requirements

Functional requirements for corpus creation, vector insertion, policy
enforcement, and lifecycle events.

---

### FR-CORP-001 — Corpus creation assigns unique identifier

```
Gist:       Every corpus created receives a unique corpus_id.
Type:       Function
Actor:      Library consumer
Function:   When the consumer creates a Corpus with a valid CodecConfig,
            Codebook, and CompressionPolicy, the resulting Corpus shall be
            assigned a corpus_id that is unique across all Corpora created
            in the same process.
Scale:      Fraction of distinct Corpus instances in a test run that have
            distinct corpus_id values.
Meter:      Create 1 000 Corpus instances in sequence; assert all corpus_id
            values are unique (no duplicates in the set).
Must:       100%
Plan:       100%
Rationale:  Identity is a precondition for cross-corpus operations and
            config-hash enforcement at insertion time.
Tests:      [Python] tests/corpus/test_corpus.py::TestCreation::test_create_empty_corpus
            [Rust]   rust/crates/tinyquant-core/tests/corpus_aggregate.rs::create_corpus_begins_empty
Gap:        GAP-CORP-001 — Neither test asserts uniqueness across multiple Corpus instances.
            Both verify that a corpus has an id, not that all ids are distinct.
            See testing-gaps.md §GAP-CORP-001.
```

---

### FR-CORP-002 — CodecConfig is frozen at corpus creation

```
Gist:       A corpus's CodecConfig cannot be changed after it is created.
Type:       Constraint
Actor:      Library consumer
Function:   Any operation that attempts to replace or mutate the CodecConfig
            of an existing Corpus shall be rejected with an immutability error.
            The corpus shall remain in its pre-attempt state.
Scale:      Fraction of mutation attempts that are rejected without
            side-effects.
Meter:      Create 100 Corpus instances; attempt to replace each corpus's
            CodecConfig via any available API path; confirm error raised and
            corpus.config unchanged.
Must:       100%
Plan:       100%
Rationale:  Allowing config mutation after insertion would make existing
            CompressedVectors undecompressable.
Tests:      [Python] tests/corpus/test_corpus.py::TestCreation::test_create_corpus_freezes_config
            [Python] tests/corpus/test_corpus.py::TestCreation::test_create_corpus_freezes_codebook
            [Rust]   rust/crates/tinyquant-core/tests/vector_entry.rs::vector_entry_immutable_after_construction
Gap:        GAP-CORP-002 — The Rust immutability test covers VectorEntry, not Corpus-level
            CodecConfig. No Rust test attempts to replace the config on an existing Corpus.
            See testing-gaps.md §GAP-CORP-002.
```

---

### FR-CORP-003 — Cross-config insertion rejected

```
Gist:       Inserting a CompressedVector whose config_hash differs from the
            corpus's config_hash fails with a config mismatch error.
Type:       Function
Actor:      Library consumer
Function:   insert(CV, corpus) shall raise CorpusError::ConfigMismatch (or
            equivalent) and leave the corpus unchanged when
            CV.config_hash ≠ corpus.config.hash().
Scale:      Fraction of mismatched insertions that raise the correct error
            without modifying the corpus.
Meter:      Create a Corpus with seed 42; attempt to insert 100
            CompressedVectors produced with seed 99; verify error type and
            that corpus.size() is unchanged.
Must:       100%
Plan:       100%
Rationale:  A corpus must hold only vectors that can be decompressed with
            the same config; mixed-config corpora would produce silent
            garbage during batch decompression.
Tests:      [Rust]   rust/crates/tinyquant-io/tests/rejection.rs::rejection_config_hash_mismatch
            [Python] tests/corpus/test_corpus.py (cross-config insertion scenario)
Gap:        None.
```

---

### FR-CORP-004 — Compress policy compresses on write

```
Gist:       Under the "compress" policy every inserted FP32 vector is codec-
            compressed before storage.
Type:       Function
Actor:      Library consumer
Function:   When a Corpus has compression_policy "compress" and the consumer
            inserts an FP32 vector V, the corpus shall store V as a
            CompressedVector (not as raw FP32).
Scale:      Fraction of insert calls under "compress" policy where the
            stored representation is a CompressedVector (not raw FP32).
Meter:      Create a "compress" corpus; insert 500 FP32 vectors; retrieve
            each VectorEntry and confirm its stored form is CompressedVector.
Must:       100%
Plan:       100%
Qualify:    compression_policy == "compress".
Rationale:  The primary purpose of TinyQuant is codec compression; the
            "compress" policy is the default production path.
Tests:      [Rust]   rust/crates/tinyquant-core/tests/compression_policy.rs::test_compress_policy_builds_codebook
            [Python] tests/corpus/test_corpus.py::TestInsert (insertion tests)
Gap:        None.
```

---

### FR-CORP-005 — Passthrough policy stores FP32 unchanged

```
Gist:       Under the "passthrough" policy FP32 vectors are stored as-is
            without codec transformation.
Type:       Function
Actor:      Library consumer
Function:   When a Corpus has compression_policy "passthrough" and the
            consumer inserts an FP32 vector V, the corpus shall store V
            in its original FP32 representation; no quantization, rotation,
            or residual computation shall be applied.
Scale:      Fraction of insert calls under "passthrough" policy where the
            retrieved vector is element-wise equal to the original input.
Meter:      Insert 500 FP32 vectors into a "passthrough" corpus; retrieve
            each; assert exact element-wise equality.
Must:       100%
Plan:       100%
Qualify:    compression_policy == "passthrough".
Rationale:  The passthrough policy is the escape hatch for vectors that must
            be stored losslessly (e.g., centroids, reference anchors).
Tests:      [Rust]   rust/crates/tinyquant-core/tests/compression_policy.rs::test_no_operation_policy_stores_raw
            [Python] tests/corpus/test_corpus.py (passthrough insertion scenario)
Gap:        None.
```

---

### FR-CORP-006 — FP16 policy downcasts precision only

```
Gist:       Under the "fp16" policy FP32 vectors are stored as FP16; no
            codebook quantization is applied.
Type:       Function
Actor:      Library consumer
Function:   When a Corpus has compression_policy "fp16" and the consumer
            inserts an FP32 vector V, the corpus shall store V as an FP16
            array. The round-trip reconstruction error shall be bounded by
            the precision loss of FP32→FP16 conversion only.
Scale:      Max element-wise absolute difference between the original FP32
            vector and the FP16-stored-then-FP32-cast vector.
Meter:      Insert 1 000 FP32 vectors into an "fp16" corpus; retrieve and
            cast back to FP32; report max |original[i] - retrieved[i]|.
Must:       ≤ 2^-13 × |original[i]| per element (machine epsilon for FP16
            is ~2^-10; factor of 8 safety margin).
Plan:       ≤ ULP(FP16(original[i])) per element.
Qualify:    compression_policy == "fp16" AND |original[i]| < 65 504
            (FP16 max representable value).
Rationale:  The "fp16" policy exists as a mid-point between "passthrough"
            (lossless) and "compress" (maximum compression); it must not
            apply codec quantization.
Tests:      [Rust]   rust/crates/tinyquant-core/tests/residual.rs::residual_encodes_fp16
Gap:        GAP-CORP-006 — The Rust test confirms FP16 encoding exists but does not
            measure the element-wise precision bound (≤ 2^-13 × |original[i]|).
            See testing-gaps.md §GAP-CORP-006.
```

---

### FR-CORP-007 — Compression policy is immutable once vectors exist

```
Gist:       A corpus's compression policy cannot change after the first
            vector has been inserted.
Type:       Constraint
Actor:      Library consumer
Function:   Any attempt to change compression_policy on a Corpus that
            contains one or more VectorEntries shall raise
            CorpusError::PolicyImmutable (or equivalent) and leave the
            corpus in its pre-attempt state.
Scale:      Fraction of policy-change attempts on populated corpora that
            raise the correct error without side-effects.
Meter:      Insert one vector into each of 100 corpora (one per policy
            variant); attempt to change each corpus's policy; confirm error
            type and that the original policy is unchanged.
Must:       100%
Plan:       100%
Rationale:  Mixed-policy corpora would make it impossible to guarantee a
            uniform decompression path for batch operations.
Tests:      [Python] tests/corpus/test_corpus.py::TestInsert (policy variant tests)
Gap:        GAP-CORP-007 — No Rust test attempts a policy change on a populated corpus.
            Python coverage is partial (policy immutability not isolated in its own test).
            See testing-gaps.md §GAP-CORP-007.
```

---

### FR-CORP-008 — Batch decompression raises domain event

```
Gist:       Successful batch decompression raises a CorpusDecompressed event.
Type:       Function
Actor:      Library consumer, event subscriber
Function:   After decompress_batch completes successfully on a Corpus, the
            Corpus shall emit a CorpusDecompressed domain event carrying the
            corpus_id and the count of vectors decompressed.
Scale:      Fraction of successful decompress_batch calls that are followed
            by a CorpusDecompressed event carrying the correct corpus_id and
            count.
Meter:      Subscribe to Corpus events; call decompress_batch on 50 corpora
            of varying sizes; verify one CorpusDecompressed event per call
            with correct fields.
Must:       100%
Plan:       100%
Qualify:    The decompress_batch call succeeds (no unhandled errors). If the
            operation fails mid-batch, a CorpusDecompressionFailed event is
            emitted instead (not in scope of this requirement).
Rationale:  Domain events are the observable signal that allows downstream
            systems (search backends, audit logs) to react to corpus state
            changes.
Tests:      [Rust]   rust/crates/tinyquant-core/tests/corpus_events.rs::corpus_decompressed_event_emitted
            [Python] tests/corpus/test_corpus.py::TestEvents::test_corpus_raised_event
Gap:        None.
```

---

### FR-CORP-009 — Empty corpus accepts insertions

```
Gist:       A newly created corpus starts empty and accepts its first
            insertion without preconditions beyond policy and config validity.
Type:       Function
Actor:      Library consumer
Function:   A Corpus created with a valid CodecConfig, Codebook, and
            CompressionPolicy shall begin with size() == 0 and shall accept
            the first valid insert call successfully.
Scale:      Fraction of newly created corpora that have size() == 0 and
            successfully accept a first insertion.
Meter:      Create 100 corpora; assert size() == 0; insert one valid vector
            into each; assert size() == 1.
Must:       100%
Plan:       100%
Rationale:  An empty corpus that cannot accept its first insertion is not
            usable; this requirement rules out any initialization race or
            policy bootstrap error.
Tests:      [Rust]   rust/crates/tinyquant-core/tests/corpus_aggregate.rs::create_corpus_begins_empty
            [Python] tests/corpus/test_corpus.py::TestCreation::test_create_empty_corpus
Gap:        None.
```

---

## See also

- [[requirements/codec|Codec Requirements]]
- [[requirements/backend|Backend Protocol Requirements]]
- [[design/behavior-layer/corpus-management|Corpus Management Behavior]]
- [[design/domain-layer/aggregates-and-entities|Aggregates and Entities]]
