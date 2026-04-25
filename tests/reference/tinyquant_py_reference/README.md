# tinyquant_py_reference

Frozen pure-Python TinyQuant implementation used as a differential oracle.

## Layout

| Path | Purpose |
| --- | --- |
| `backend/` | Reference brute-force backend and protocols |
| `codec/` | Reference codec, codebook, rotation, and serialization types |
| `corpus/` | Reference corpus aggregate and related value objects |
| `tools/` | Helper tooling for serialization and fixture inspection |
| `_types.py` | Shared typed aliases |

## Notes

This package is pinned to the last pure-Python behavior. It is kept under
`tests/` to make the non-shipping status obvious.

## See Also

- [Local AGENTS.md](./AGENTS.md)
- [Parent README](../README.md)
- [src/README.md](../../../src/README.md)
