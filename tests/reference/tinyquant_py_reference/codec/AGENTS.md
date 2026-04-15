# AGENTS.md — Guide for AI Agents Working in `tests/reference/tinyquant_py_reference/codec`

`codec/` holds the frozen pure-Python codec implementation used as an oracle.

## Layout

```text
tests/reference/tinyquant_py_reference/codec/
├── codec.py
├── codec_config.py
├── codebook.py
├── compressed_vector.py
├── rotation_matrix.py
├── _quantize.py
├── _errors.py
├── __init__.py
└── README.md
```

## Common Workflows

### Touch the reference codec

1. Confirm the change is for parity preservation, not new feature development.
2. Keep file-level responsibilities stable when possible.
3. Re-run codec and parity tests after changing serialization or math behavior.

## Invariants — Do Not Violate

- This subtree preserves historical codec behavior.
- Serialization and quantization changes must be reflected in the relevant
  parity or packaging tests.
- Do not drift away from the documented oracle role.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../../AGENTS.md)
- [tests/parity/AGENTS.md](../../../parity/AGENTS.md)
