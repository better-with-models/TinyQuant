# tests/integration

This directory contains integration tests that verify the handoff contracts between
adjacent layers. There are four test files: two covering internal bounded-context
boundaries (IB-01, IB-02), one covering the `CompressedVector` serialization wire
format (EB-01), and one covering the live PostgreSQL + pgvector adapter (EB-02).
The pgvector tests require an external service and are gated by the `pgvector`
pytest marker; they are skipped automatically when neither `PGVECTOR_TEST_DSN`
nor Docker is available.

## What lives here

- `conftest.py` — session-scoped `pgvector_dsn` fixture. Resolves a PostgreSQL
  DSN by checking `PGVECTOR_TEST_DSN` env var first, then launching a
  `pgvector/pgvector:pg17` Docker container via testcontainers, or skipping if
  neither is available.
- `test_codec_corpus.py` (IB-01) — `TestCodecCorpusIntegration`: verifies that
  a `CompressedVector` produced by `Codec.compress` is identical to the one
  stored by `Corpus.insert` (same config hash, indices, and residual); that
  `Corpus.decompress` matches a direct `Codec.decompress` round-trip; and that
  batch corpus insert matches direct batch codec compress.
- `test_corpus_backend.py` (IB-02) — `TestCorpusBackendIntegration`: verifies
  that `Corpus.decompress_all()` output is accepted by `BruteForceBackend.ingest`
  without errors, produces FP32 vectors of the correct shape, and that a
  subsequent search returns ranked results with IDs matching the corpus.
- `test_serialization.py` (EB-01) — `TestSerializeDeserialize`: round-trip
  `from_bytes(to_bytes(cv))` produces an identical `CompressedVector`; verifies
  version byte is `0x01`; rejects unknown version byte (`0xFF`) with `ValueError`;
  rejects truncated data; detects tampered config hash; handles dim=3072; and
  confirms residual bytes survive serialization.
- `test_pgvector.py` (EB-02, `@pytest.mark.pgvector`) — `TestPgvectorAdapter`:
  ingest writes rows to the target table, search returns ranked `SearchResult`
  objects, dimension is preserved through the wire format, remove deletes rows,
  SQL injection attempts via parameterized queries are safe, and the connection
  factory callable is used rather than a hardcoded connection.
- `__init__.py` — empty package marker.

## How this area fits the system

Integration tests use the same shared fixtures as unit tests (`config_4bit`,
`trained_codebook`, `sample_vector`, `sample_vectors`, `corpus_compress`,
`sample_corpus`, `brute_force_backend`) from `tests/conftest.py`, supplemented
by local fixtures for the pgvector session connection and a dim=3072 stress-test
config. These tests sit between unit tests (single-layer) and E2E tests
(full pipeline) in the testing pyramid.

## Common edit paths

- `test_pgvector.py` — add tests when `PgvectorAdapter` gains new methods or
  when the SQL schema changes (e.g., adding an ANN index).
- `test_serialization.py` — update version byte assertions or add new test cases
  when the `CompressedVector` binary format version is bumped.
- `conftest.py` — update the Docker image tag (`pgvector/pgvector:pg17`) when
  the minimum supported PostgreSQL version changes.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
