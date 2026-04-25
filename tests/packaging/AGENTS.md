# AGENTS.md — Guide for AI Agents Working in `tests/packaging`

`tests/packaging/` verifies packaging-time behavior for the Rust-backed Python wheel.

## Layout

```text
tests/packaging/
├── test_assemble_fat_wheel.py
├── test_python_fatwheel_workflow.py
├── test_selector_detection.py
├── test_shim_parity.py
└── README.md
```

## Common Workflows

### Add or update a packaging regression

1. Put the assertion in the narrowest existing test module.
2. Keep the test name aligned with the packaging script it protects.
3. Re-run `pytest tests/packaging -q` after the script or template change.

## Invariants — Do Not Violate

- These tests gate packaging layout and selector behavior; do not loosen them
  just to accept drift.
- Shim-parity assertions must stay aligned with `src/tinyquant_cpu/` and the
  assembled wheel templates.
- Packaging expectations change only with corresponding script or template
  changes in the same commit.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../AGENTS.md)
- [scripts/packaging/AGENTS.md](../../scripts/packaging/AGENTS.md)
