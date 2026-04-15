# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-py/tests/python`

`tests/python/` verifies the Python-facing behavior of the Rust extension crate.

## Layout

```text
rust/crates/tinyquant-py/tests/python/
├── test_parity.py
└── README.md
```

## Common Workflows

### Update a Python binding test

1. Keep assertions focused on Python-visible behavior.
2. Prefer parity-oriented checks over Rust-internal details.
3. Re-run the Python binding tests after editing this directory.

## Invariants — Do Not Violate

- These tests protect the Python-facing extension contract.
- Keep the test aligned with the frozen oracle and outer shim expectations.
- Do not make this directory the home for generic crate tests.

## See Also

- [Parent AGENTS.md](../../AGENTS.md)
- [Root AGENTS.md](../../../../../AGENTS.md)
- [tests/parity/AGENTS.md](../../../../../tests/parity/AGENTS.md)
