# AGENTS.md — Guide for AI Agents Working in `scripts/packaging`

`scripts/packaging/` holds assembly and fixture-generation tooling for release artifacts.

## Layout

```text
scripts/packaging/
├── assemble_fat_wheel.py
├── fabricate_dummy_wheels.py
├── generate_js_parity_fixtures.py
├── templates/
└── README.md
```

## Common Workflows

### Update fat-wheel assembly

1. Keep `assemble_fat_wheel.py`, the wheel templates, and
   `tests/packaging/` expectations aligned.
2. Re-run the narrowest packaging test that covers the changed behavior.
3. Update user-facing docs if the assembled wheel layout changes.

### Update parity fixture generation

1. Regenerate fixtures in a controlled test run rather than hand-editing them.
2. Keep JavaScript test expectations aligned with the generator output.

## Invariants — Do Not Violate

- Packaging scripts define artifact layout; tests under `tests/packaging/`
  are the gate for that layout.
- Template files under `templates/` are copied into artifacts and must stay in
  sync with the assembled wheel contract.
- Do not change artifact names, selector logic, or shim file names without
  updating the corresponding tests and docs.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../AGENTS.md)
- [tests/packaging/AGENTS.md](../../tests/packaging/AGENTS.md)
