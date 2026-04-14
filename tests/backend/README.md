# tests/backend

This directory contains unit tests for the `backend` bounded context of the
Python reference implementation. There is one test file,
`test_brute_force.py`, which exercises `BruteForceBackend` and `SearchResult`
from `tinyquant_py_reference.backend`. The tests are separated from codec and
corpus suites because the backend layer has no dependency on compression logic —
it operates on plain FP32 vectors already decompressed by the corpus layer.

## What lives here

- `test_brute_force.py` — five test classes:
  - `TestSearchResult` — construction, descending-score ordering via `sorted()`,
    and immutability of `SearchResult`.
  - `TestIngest` — `BruteForceBackend.ingest()` stores vectors, overwrites on
    duplicate ID, and accepts an empty dict without error.
  - `TestSearch` — returns exactly `top_k` results, ranks by descending cosine
    similarity, handles fewer vectors than `top_k`, returns `SearchResult`
    objects, produces score ~1.0 for a self-query, ~0.0 for orthogonal vectors,
    and returns an empty list for an empty backend.
  - `TestRemove` — `remove()` decrements count and excluded IDs do not appear
    in subsequent search results.
  - `TestClear` — `clear()` resets count to zero.
- `__init__.py` — empty package marker.

## How this area fits the system

`BruteForceBackend` is the in-memory search backend used by the Python reference
implementation and by most E2E and integration tests. It implements cosine
similarity over ingested FP32 vectors. The shared `sample_vectors` and `sample_vector`
fixtures from `tests/conftest.py` supply the test data; no corpus or codec
fixtures are needed here.

## Common edit paths

- `test_brute_force.py` when adding new methods to `BruteForceBackend`
  (e.g., `update`, `clear_by_prefix`) or changing `SearchResult` fields.
- Add a test class for any new backend method that is introduced into the
  `SearchBackend` protocol in `tinyquant_py_reference.backend.protocol`.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
