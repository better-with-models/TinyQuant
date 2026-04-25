# tests/corpus

This directory contains unit tests for the `corpus` bounded context of the Python
reference implementation. The tests cover `Corpus` (the aggregate root), the
`CompressionPolicy` enum, domain events, and the `VectorEntry` entity. They are
separated from the codec suite because the corpus layer depends on the codec layer
but not vice versa, and from integration tests because no cross-layer handoff is
exercised here.

## What lives here

- `test_compression_policy.py` — tests for `CompressionPolicy`: `requires_codec()`
  returns `True` for `COMPRESS` and `False` for `PASSTHROUGH`/`FP16`;
  `storage_dtype()` maps to `uint8`, `float32`, and `float16` respectively.
- `test_corpus.py` — tests for the `Corpus` aggregate: creation (zero count,
  frozen config and codebook, `CorpusCreated` event, event draining); single
  insert under `COMPRESS` policy (count, `VectorEntry` type, compressed
  representation, config hash, dimension mismatch, duplicate ID, UTC timestamp,
  per-vector metadata, `VectorsInserted` event); insert under `PASSTHROUGH` and
  `FP16` policies; batch insert (atomic on duplicate, correct event listing all
  IDs, empty no-op); `get`/`contains`; `decompress`/`decompress_all` (FP32
  output, KeyError, all IDs present, `CorpusDecompressed` event); `remove`
  (count decrement, KeyError, ID unavailable after removal); and property
  invariants (`vector_count`, `is_empty`, `vector_ids`).
- `test_events.py` — tests for domain events: `CorpusCreated`, `VectorsInserted`,
  `CorpusDecompressed`, and `CompressionPolicyViolationDetected` — each verified
  for field population, frozen immutability, and UTC timezone-awareness.
- `test_vector_entry.py` — tests for `VectorEntry`: construction, metadata
  defaulting to `None`; property delegation to `CompressedVector` (`config_hash`,
  `dimension`, `has_residual`); identity equality and hashing by `vector_id`;
  and mutability of the metadata dict.
- `__init__.py` — empty package marker.

## How this area fits the system

These tests use fixtures from `tests/conftest.py`: `config_4bit`,
`trained_codebook`, `sample_vector`, `sample_vectors`, `corpus_compress`,
`corpus_passthrough`, and `corpus_fp16`. The corpus layer sits above the codec
layer in the dependency DAG, so these tests implicitly exercise the codec
pipeline end-to-end. The architecture tests in `tests/architecture/` verify
that the corpus layer does not import the backend layer.

## Common edit paths

- `test_corpus.py` — add tests for new `Corpus` methods or new
  `CompressionPolicy` variants.
- `test_events.py` — add tests for any new domain event added to
  `tinyquant_py_reference.corpus.events`.
- `test_vector_entry.py` — update when `VectorEntry` gains new delegated
  properties or changes its equality semantics.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
