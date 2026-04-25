# packaging

Packaging and fixture-generation scripts for the Python fat wheel and JavaScript parity surface.

## Files

| File | Purpose |
| --- | --- |
| `assemble_fat_wheel.py` | Assemble the Rust-backed `tinyquant-cpu` fat wheel layout |
| `fabricate_dummy_wheels.py` | Build dummy wheel inputs for packaging tests |
| `generate_js_parity_fixtures.py` | Emit parity fixtures consumed by the npm package tests |
| `templates/` | File templates copied into assembled wheel artifacts |

## See Also

- [Local AGENTS.md](./AGENTS.md)
- [Parent README](../README.md)
- [tests/packaging/README.md](../../tests/packaging/README.md)
