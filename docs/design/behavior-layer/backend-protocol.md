---
title: Backend Protocol Behavior
tags:
  - design
  - behavior
  - bdd
  - backend
  - integration
date-created: 2026-04-08
status: active
category: design
source-count: 6
---

# Backend Protocol Behavior

> [!info] Capability
> Define the contract between TinyQuant's corpus layer and external search
> backends, ensuring backends only ever receive FP32 vectors.

## Actor

Backend adapter implementation or integration test harness.

## Preconditions

- A populated Corpus with compressed vectors
- A SearchBackend implementation (brute-force reference or external adapter)

---

## Scenarios

### Backend receives FP32 vectors only

```gherkin
Feature: Backend protocol

  Scenario: Search backend receives decompressed FP32 vectors
    Given a Corpus containing compressed vectors
    And a SearchBackend adapter
    When the corpus is decompressed for backend ingestion
    Then the backend receives only FP32 vectors
    And no compressed representations cross the backend boundary
```

### Brute-force backend returns correct results

```gherkin
  Scenario: Brute-force backend produces ranked results from decompressed vectors
    Given a Corpus of 1000 vectors, decompressed to FP32
    And a brute-force SearchBackend
    And a query vector Q
    When Q is searched against the backend with top_k 10
    Then 10 results are returned
    And results are ranked by descending cosine similarity
```

### Backend adapter translates vector format

```gherkin
  Scenario: pgvector adapter converts FP32 arrays to pgvector format
    Given a set of decompressed FP32 vectors
    And a pgvector BackendAdapter
    When the adapter prepares vectors for ingestion
    Then each vector is formatted as a pgvector-compatible array literal
    And vector dimensionality is preserved
```

### Backend never receives partial decompression

```gherkin
  Scenario: Partial decompression failure does not send incomplete data
    Given a Corpus of 100 vectors where 1 vector has corrupted data
    When batch decompression is attempted
    Then the backend does not receive the corrupted vector
    And the error is reported with the affected vector_id
    And successfully decompressed vectors are still available
```

### Query vectors pass through without compression

```gherkin
  Scenario: Query vectors are not compressed before search
    Given an FP32 query vector Q
    And a SearchBackend populated with decompressed corpus vectors
    When Q is used to search the backend
    Then Q is passed directly to the backend in FP32
    And no codec transformation is applied to Q
```

---

## Rules encoded by these scenarios

1. **FP32 boundary:** compressed representations never cross into backend territory
2. **Adapter responsibility:** format translation is the adapter's job, not the backend's
3. **Error isolation:** one bad vector does not poison the entire batch
4. **Query passthrough:** query vectors are never codec-transformed
5. **Backend agnosticism:** the protocol works with any backend that accepts FP32 vectors

## Automation level

Brute-force backend scenarios are component tests. Adapter scenarios (pgvector
etc.) are integration tests that may require external infrastructure.

## See also

- [[behavior-layer/corpus-management|Corpus Management]]
- [[behavior-layer/codec-decompression|Codec Decompression]]
- [[domain-layer/context-map|Context Map]]
- [[storage-codec-vs-search-backend-separation]]
