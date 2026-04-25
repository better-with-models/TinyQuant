# AGENTS.md — Guide for AI Agents Working in `tests/codec`

This directory holds unit tests for the codec layer: codebook training, quantization, compressed-vector serialization, and rotation matrix construction. Tests exercise each component in isolation and assert byte-level parity against Python-generated fixtures where applicable. Changes here happen when codec internals (bit widths, serialization format, `CodecConfig` fields) are modified or new codec features are added.

## What this area contains

- primary responsibility: `test_codebook.py` (codebook training and lookup), `test_codec.py` (end-to-end compress/decompress), `test_codec_config.py` (`CodecConfig` serialization and round-trip), `test_compressed_vector.py` (byte-parity of the compressed vector format), `test_rotation_matrix.py` (determinism and orthogonality of the rotation matrix)
- main entrypoints: `test_codec.py` for the end-to-end path; `test_compressed_vector.py` for byte-format parity
- common changes: updating fixture paths after `cargo xtask fixtures refresh-codec`, adding tests for new bit-width variants, adjusting assertions when the `CompressedVector` binary format changes

## Layout

```text
codec/
├── __init__.py
├── README.md
├── test_codebook.py
├── test_codec.py
├── test_codec_config.py
├── test_compressed_vector.py
└── test_rotation_matrix.py
```

## Common workflows

### Update existing behavior

1. Read the local README and the files you will touch before editing.
2. Follow the local invariants before introducing new files or abstractions.
3. Update nearby docs when the change affects layout, commands, or invariants.
4. Run the narrowest useful verification first, then the broader project gate.

### Add a new file or module

1. Confirm the new file belongs in this directory rather than a sibling.
2. Update the layout section if the structure changes in a way another agent must notice.
3. Add or refine local docs when the new file introduces a new boundary or invariant.

## Invariants — Do Not Violate

- keep this directory focused on its stated responsibility
- do not invent APIs, workflows, or invariants that the code does not support
- update this file when structure or safe-editing rules change

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../AGENTS.md)
