# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-cli/src/commands`

`src/commands/` contains the command handlers behind the `tinyquant` CLI subcommands.

## Layout

```text
rust/crates/tinyquant-cli/src/commands/
├── codebook_io.rs
├── codec.rs
├── codec_compress.rs
├── codec_decompress.rs
├── codec_train.rs
├── corpus.rs
├── corpus_ingest.rs
├── corpus_search.rs
├── info.rs
├── verify.rs
├── mod.rs
└── README.md
```

## Common Workflows

### Update a command

1. Keep clap-facing behavior and help text aligned with the parent CLI docs.
2. Change the narrowest handler file possible.
3. Re-run the CLI tests or smoke scripts after changing flags or output.

## Invariants — Do Not Violate

- `mod.rs` is the command registry.
- Command handlers here own CLI orchestration, not core codec semantics.
- Exit behavior and output shape changes must be reflected in tests or docs.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../../../AGENTS.md)
- [rust/crates/tinyquant-cli/tests/AGENTS.md](../../tests/AGENTS.md)
