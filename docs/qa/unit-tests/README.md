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

> [!note] Reference location (Phase 23+)
> The pure-Python implementation under test lives at
> `tests/reference/tinyquant_py_reference/`, not `src/tinyquant_cpu/`.
> The `tests/` suite imports it as `tinyquant_py_reference` via the
> `pythonpath` extension in `pyproject.toml`. See
> [[entities/python-reference-implementation|Python Reference Implementation]]
> and [[plans/rust/phase-23-python-reference-demotion|Phase 23]] for
> the rename rationale. A sibling `tests/parity/` suite (run via
> `pytest -m parity`) covers cross-implementation parity and is
> documented in [[design/rust/testing-strategy|Testing Strategy]].

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

Floors apply to the reference implementation at
`tests/reference/tinyquant_py_reference/`:

| Package | Floor |
|---------|-------|
| `tinyquant_py_reference/codec/` | 95% |
| `tinyquant_py_reference/corpus/` | 90% |
| `tinyquant_py_reference/backend/` | 80% |
| Overall | 90% |

## See also

- [[qa/README|Quality Assurance]]
- [[classes/README|Class Specifications]]
- [[design/architecture/test-driven-development|Test-Driven Development]]
