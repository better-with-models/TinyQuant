---
title: Unit Tests
tags:
  - qa
  - testing
  - unit-tests
  - meta
date-created: 2026-04-08
status: active
category: qa
---

# Unit Tests

> [!info] Purpose
> Test specifications for every planned class and module. Each page maps
> one-to-one with a class in [[classes/README|Class Specifications]] and a
> test file in the `tests/` directory.

## Test file index

### Codec package — `tests/codec/`

| Test file | Class under test | Page | Est. tests |
|-----------|-----------------|------|-----------|
| `test_codec_config.py` | [[classes/codec-config\|CodecConfig]] | [[qa/unit-tests/test-codec-config\|spec]] | ~15 |
| `test_rotation_matrix.py` | [[classes/rotation-matrix\|RotationMatrix]] | [[qa/unit-tests/test-rotation-matrix\|spec]] | ~12 |
| `test_codebook.py` | [[classes/codebook\|Codebook]] | [[qa/unit-tests/test-codebook\|spec]] | ~14 |
| `test_compressed_vector.py` | [[classes/compressed-vector\|CompressedVector]] | [[qa/unit-tests/test-compressed-vector\|spec]] | ~12 |
| `test_codec.py` | [[classes/codec\|Codec]] | [[qa/unit-tests/test-codec\|spec]] | ~25 |

### Corpus package — `tests/corpus/`

| Test file | Class under test | Page | Est. tests |
|-----------|-----------------|------|-----------|
| `test_corpus.py` | [[classes/corpus\|Corpus]] | [[qa/unit-tests/test-corpus\|spec]] | ~30 |
| `test_vector_entry.py` | [[classes/vector-entry\|VectorEntry]] | [[qa/unit-tests/test-vector-entry\|spec]] | ~8 |
| `test_compression_policy.py` | [[classes/compression-policy\|CompressionPolicy]] | [[qa/unit-tests/test-compression-policy\|spec]] | ~6 |
| `test_events.py` | [[classes/corpus-events\|Corpus Events]] | [[qa/unit-tests/test-events\|spec]] | ~10 |

### Backend package — `tests/backend/`

| Test file | Class under test | Page | Est. tests |
|-----------|-----------------|------|-----------|
| `test_brute_force.py` | [[classes/brute-force-backend\|BruteForceBackend]] | [[qa/unit-tests/test-brute-force\|spec]] | ~12 |

## Conventions

- **Naming:** test functions use `test_<behavior_description>` not
  `test_<method_name>`
- **Fixtures:** shared in `tests/conftest.py`; package-specific fixtures in
  `tests/<package>/conftest.py`
- **Assertions:** one logical assertion per test; multiple `assert` lines are
  fine when checking one outcome
- **No mocks for domain logic:** mocks only at true I/O boundaries
- **Property-based tests:** use `hypothesis` for invariant verification
  (determinism, round-trip, range constraints)

## Coverage targets

| Package | Floor |
|---------|-------|
| `tinyquant/codec/` | 95% |
| `tinyquant/corpus/` | 90% |
| `tinyquant/backend/` | 80% |
| Overall | 90% |

## See also

- [[qa/README|Quality Assurance]]
- [[classes/README|Class Specifications]]
- [[design/architecture/test-driven-development|Test-Driven Development]]
