---
title: Backend Protocol Requirements
tags:
  - requirements
  - planguage
  - backend
date-created: 2026-04-15
status: draft
category: requirements
---

# Backend Protocol Requirements

Functional requirements for the contract between TinyQuant's corpus layer
and external search backends. These requirements govern what crosses the
corpus-to-backend boundary and how errors are isolated.

---

### FR-BACK-001 — Only FP32 vectors cross the backend boundary

```
Gist:       Compressed representations are never passed to a SearchBackend;
            only decompressed FP32 vectors are.
Type:       Constraint
Actor:      Backend adapter implementation
Function:   The corpus layer shall decompress all CompressedVectors to FP32
            before passing them to any SearchBackend. No method on the
            SearchBackend trait shall accept a CompressedVector as a
            parameter.
Scale:      Fraction of corpus-to-backend data transfers in the test suite
            where every value passed to the backend is an FP32 vector (not
            a CompressedVector or raw byte buffer).
Meter:      Instrument 5 000 backend ingest calls (brute-force and pgvector
            adapters); assert that no call delivers a non-FP32 representation.
Must:       100%
Plan:       100%
Rationale:  The backend protocol boundary is an anti-corruption layer. Search
            backends must not need to understand the codec; coupling them to
            the compressed format would make the codec a leaky abstraction.
```

---

### FR-BACK-002 — Brute-force backend ranks results by descending cosine similarity

```
Gist:       The brute-force SearchBackend returns top-k results ordered by
            descending cosine similarity to the query.
Type:       Function
Actor:      Library consumer, integration test harness
Function:   Given a BruteForceBackend populated with N decompressed FP32
            vectors and a query vector Q, search(Q, top_k=K) shall return
            exactly K SearchResult objects (or fewer if N < K), ordered so
            that result[0].score ≥ result[1].score ≥ … ≥ result[K-1].score,
            where each score is the cosine similarity of the result vector
            to Q.
Scale:      Fraction of search calls where the result list is correctly
            ordered by descending score AND has the correct length.
Meter:      Run 1 000 search queries against corpora of sizes
            {100, 1 000, 10 000} with K in {1, 5, 10, 50}; verify ordering
            and length for each.
Must:       100%
Plan:       100%
Qualify:    All vectors in the backend are finite FP32 and the query vector
            is finite FP32 with the same dimension.
Rationale:  A search result list that is not sorted defeats the purpose of
            top-k retrieval; consumers rely on result[0] being the best match.
```

---

### FR-BACK-003 — Backend adapter preserves vector dimensionality

```
Gist:       Format-translating adapters (e.g., pgvector) preserve the
            dimension of every FP32 vector.
Type:       Function
Actor:      Backend adapter implementation
Function:   After a SearchBackend adapter (pgvector or equivalent) formats
            an FP32 vector for ingestion, the stored representation shall
            round-trip back to an FP32 vector of the same dimension, with
            every element equal within float32 precision.
Scale:      Fraction of round-tripped vectors that have the same dimension
            and element-wise equality to the original.
Meter:      Pass 1 000 FP32 vectors of dimension D through the pgvector
            adapter; retrieve and compare to originals.
Must:       100%
Plan:       100%
Qualify:    Adapter is pgvector; vector elements are within the representable
            range of the wire format (pgvector TEXT: full float32 precision).
Rationale:  Dimensionality corruption turns every similarity score wrong;
            adapters must be lossless format converters.
```

---

### FR-BACK-004 — Error isolation: one corrupt vector does not block others

```
Gist:       If one vector in a batch decompression fails, the others are
            still delivered to the backend and the error is reported per
            affected vector.
Type:       Function
Actor:      Library consumer, backend adapter
Function:   When decompress_batch is called on a corpus of N vectors where
            M ≤ N vectors are corrupt or undecompressable, the backend shall
            receive (N - M) successfully decompressed FP32 vectors, and
            the caller shall receive an error report identifying each of
            the M affected vector_ids.
Scale:      (a) Fraction of successful vectors still delivered when M < N
            vectors fail.
            (b) Fraction of corrupt vector_ids correctly identified in the
            error report.
Meter:      Inject 1 corrupt vector into corpora of sizes {10, 100, 1 000};
            invoke decompress_batch; verify (N-1) FP32 vectors delivered and
            the corrupt vector_id appears in the error report.
Must:       (a) 100% of successful vectors delivered.
            (b) 100% of corrupt vector_ids identified.
Plan:       Same.
Rationale:  Poisoning an entire batch because of one bad entry would make
            TinyQuant unusable in append-heavy production corpora where
            occasional corruption is expected.
```

---

### FR-BACK-005 — Query vectors are not codec-transformed

```
Gist:       FP32 query vectors are passed to the backend without compression,
            rotation, or any codec transformation.
Type:       Constraint
Actor:      Library consumer
Function:   When a consumer passes an FP32 query vector Q to a search
            operation backed by a SearchBackend, Q shall be forwarded to the
            backend in the same FP32 representation it was supplied in. No
            rotation, quantization, or residual computation shall be applied
            to Q.
Scale:      Fraction of search calls where the query vector received by the
            backend is element-wise identical to the query vector supplied
            by the consumer.
Meter:      Intercept 1 000 backend search calls; compare the query vector
            at the backend interface to the query vector at the consumer
            interface. Assert element-wise equality.
Must:       100%
Plan:       100%
Rationale:  Query vectors and corpus vectors are compared in the same space.
            Applying the codec rotation to the query but not re-compressing
            it introduces a space mismatch that corrupts similarity scores.
```

---

## See also

- [[requirements/corpus|Corpus Requirements]]
- [[requirements/codec|Codec Requirements]]
- [[design/behavior-layer/backend-protocol|Backend Protocol Behavior]]
- [[design/domain-layer/context-map|Context Map]]
