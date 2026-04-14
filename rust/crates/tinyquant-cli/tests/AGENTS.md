# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-cli/tests`

This directory holds CLI smoke tests for the `tinyquant` binary. Tests shell out via `assert_cmd::Command::cargo_bin("tinyquant")` and assert exit codes, stdout content, and on-disk artefacts. They are the canonical gate for the subcommand contract described in `docs/plans/rust/phase-22-pyo3-cabi-release.md` §Step 12. CI and release workflows depend on these tests passing before publishing.

## What this area contains

- primary responsibility: end-to-end smoke tests for the `tinyquant` binary — `info`, `codec train/compress/decompress` round-trip, and exit-code assertions
- main entrypoints: `cli_smoke.rs` — contains `info_subcommand_prints_version`, `codec_train_writes_codebook_file`, and `codec_compress_decompress_round_trip_via_cli`
- common changes: adding coverage for new subcommands, updating assertions when flag names or output formats change

## Layout

```text
tests/
├── cli_smoke.rs
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
- [Root AGENTS.md](../../../../AGENTS.md)
