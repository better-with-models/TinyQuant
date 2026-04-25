# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-cli/src`

This directory contains all Rust source for the `tinyquant` binary. `main.rs` is the single source of truth for the clap derive layout; per-subcommand logic lives in `commands/` modules; `io.rs` handles format-aware matrix reading and writing; `progress.rs` provides `indicatif` helper wrappers. The CLI integration tests in `../tests/` and any downstream consumer that shells out to the binary depend on the exit-code contract and flag surface defined here.

## What this area contains

- primary responsibility: `main.rs` (clap struct definitions, `dispatch`, `report`, `init_tracing`), `io.rs` (matrix I/O for `f32`/`npy`/`csv`/`jsonl` formats), `progress.rs` (`indicatif` progress-bar helpers), `commands/` (dispatch modules for each subcommand)
- main entrypoints: `main.rs` for the top-level `Cli` / `Command` / `CodecCmd` / `CorpusCmd` structures; `commands/mod.rs` for `CliErrorKind` and the `generate_completion` / `generate_man` helpers
- common changes: adding or renaming subcommand variants, adjusting exit codes (`CliErrorKind`), extending `VectorFormat`/`SearchOutputFormat`/`IngestPolicy` enums, modifying `--threads` defaults

## Layout

```text
src/
├── commands/
│   ├── codec.rs
│   ├── codec_compress.rs
│   ├── codec_decompress.rs
│   ├── codec_train.rs
│   ├── codebook_io.rs
│   ├── corpus.rs
│   ├── corpus_ingest.rs
│   ├── corpus_search.rs
│   ├── info.rs
│   ├── mod.rs
│   └── verify.rs
├── io.rs
├── main.rs
├── progress.rs
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
