# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-py/python/tinyquant_rs`

`tinyquant_rs/` is the Python package stub that exposes the compiled Rust extension.

## Layout

```text
rust/crates/tinyquant-py/python/tinyquant_rs/
├── __init__.py
├── py.typed
└── README.md
```

## Common Workflows

### Update the package stub

1. Keep the package surface minimal and consistent with the built extension.
2. Avoid adding Python-side logic here that belongs in Rust or the outer shim.
3. Re-run Python packaging or parity tests after touching this directory.

## Invariants — Do Not Violate

- This is a package stub, not the user-facing Python shim.
- Keep import behavior aligned with the built extension layout.
- `py.typed` remains present so the package advertises typing support.

## See Also

- [Parent AGENTS.md](../../AGENTS.md)
- [Root AGENTS.md](../../../../../AGENTS.md)
- [src/AGENTS.md](../../../../../src/AGENTS.md)
