# tests

Python test suite for TinyQuant. Tests are organized by layer, with shared fixtures in
`conftest.py` and per-suite `conftest.py` files where needed.

## Suite layout

| Directory | Scope |
| --- | --- |
| `architecture/` | Dependency-direction and export-consistency checks (V-01) |
| `backend/` | Unit tests for `BruteForceBackend` and `SearchResult` |
| `calibration/` | Quality-gate tests for score fidelity, compression ratio, determinism, and research alignment (VAL-01–VAL-04). All `@pytest.mark.calibration` and slow by default. |
| `codec/` | Unit tests for `Codebook`, `Codec`, `CodecConfig`, `CompressedVector`, `RotationMatrix` |
| `corpus/` | Unit tests for `Corpus`, `CompressionPolicy`, `VectorEntry`, and domain events |
| `e2e/` | End-to-end pipeline tests covering compress → serialize → search |
| `integration/` | Integration tests for codec–corpus and corpus–backend handoffs; `test_pgvector.py` requires a live PostgreSQL + pgvector instance |
| `packaging/` | Fat-wheel assembler audits, selector detection, and shim-parity tests (Phase 24) |
| `parity/` | Cross-implementation parity suite (`pytest -m parity`). Phase 23: Python self-parity only. Phase 24 wires Rust. |
| `reference/` | Houses `tinyquant_py_reference`, the frozen pure-Python reference implementation used as a differential oracle |

## Running tests

```bash
# Fast unit tests only
pytest tests/ -m "not calibration and not parity and not integration"

# Parity suite
pytest -m parity

# Full suite (calibration requires fixtures from scripts/calibration/)
pytest tests/
```

## See also

- [Local AGENTS.md](./AGENTS.md)
- [Root README](../README.md)
