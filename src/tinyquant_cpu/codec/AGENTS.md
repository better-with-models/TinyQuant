# AGENTS.md — Guide for AI Agents Working in `src/tinyquant_cpu/codec`

This sub-package re-exports the codec layer of the `tinyquant_cpu` Python
shim. It exposes `CodecConfig`, `Codebook`, `Codec`, `CompressedVector`, and
`RotationMatrix` to callers of `tinyquant_cpu.codec`. In Phase 24 these come
from the `tinyquant_rs._core` Rust extension; the pure-Python reference
implementations in the individual modules remain as fallbacks.

## What this area contains

- primary responsibility: expose the full codec API (`CodecConfig`, `Codebook`, `Codec`, `CompressedVector`, `RotationMatrix`) under `tinyquant_cpu.codec`
- main entrypoints: `__init__.py` (re-exports and Rust/Python dispatch), `codec.py`, `codebook.py`, `codec_config.py`, `compressed_vector.py`, `rotation_matrix.py`
- common changes: updating `__init__.py` to prefer the Rust-backed type when available; adding new public methods to the pure-Python fallbacks

## Layout

```text
codec/
├── __init__.py
├── _errors.py
├── _quantize.py
├── codebook.py
├── codec.py
├── codec_config.py
├── compressed_vector.py
├── README.md
└── rotation_matrix.py
```

## Common workflows

### Update existing behavior

1. Read `__init__.py` to understand the Rust/Python dispatch before touching any module.
2. Changes to the pure-Python classes must keep the interface compatible with the Rust-backed classes.
3. Run `pytest tests/codec/` and `pytest -m parity` after any change.

### Add a new public type

1. Implement it in a new `<type>.py` module following the existing pattern.
2. Re-export from `__init__.py`.
3. Add the matching PyO3 class in `rust/crates/tinyquant-py/src/` and keep the interfaces in sync.

## Invariants — Do Not Violate

- The public interface of every class here must be identical to the corresponding Rust-backed class.
- `_errors.py` and `_quantize.py` are internal helpers; do not import them from outside this package.
- `__init__.py` must guard Rust-extension imports with `try/except ImportError`.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../AGENTS.md)
