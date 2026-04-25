# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-cli`

This crate produces the `tinyquant` CLI binary — the primary operator-facing tool for the TinyQuant codec. It owns argument parsing (via clap derive), subcommand dispatch, progress-bar wiring, and format-aware I/O. The Python reference implementation, `tinyquant-sys` C ABI consumers, and CI smoke tests all depend on the binary produced here. Changes most often happen when adding or modifying subcommands, adjusting exit-code behaviour, or extending the supported input/output formats.

## What this area contains

- primary responsibility: `tinyquant` binary — `codec train/compress/decompress`, `corpus ingest/decompress/search`, `verify`, and `info` subcommands
- main entrypoints: `src/main.rs` (clap derive layout and dispatch), `src/commands/` (per-subcommand logic), `src/io.rs` (format-aware matrix I/O)
- common changes: adding subcommand flags, adjusting exit codes, extending `VectorFormat` / `SearchOutputFormat` variants, updating progress-bar integration

## Layout

```text
tinyquant-cli/
├── src/
├── tests/
├── Cargo.toml
└── README.md
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
- [Root AGENTS.md](../../../AGENTS.md)
