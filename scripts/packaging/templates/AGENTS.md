# AGENTS.md — Guide for AI Agents Working in `scripts/packaging/templates`

`templates/` holds file templates copied into assembled wheel artifacts.

## Layout

```text
scripts/packaging/templates/
├── __init__.py
├── _selector.py
├── backend__init__.py
├── codec__init__.py
├── corpus__init__.py
├── METADATA
├── WHEEL
└── README.md
```

## Common Workflows

### Update a packaging template

1. Change the narrowest template file possible.
2. Keep template content aligned with the assembler script and packaging tests.
3. Re-run packaging verification after touching this directory.

## Invariants — Do Not Violate

- These files are copied into built artifacts, so file names and content shape
  are part of the packaging contract.
- Do not hand-edit generated artifacts instead of fixing the template source.
- Template changes must stay aligned with `tests/packaging/`.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../AGENTS.md)
- [tests/packaging/AGENTS.md](../../../tests/packaging/AGENTS.md)
