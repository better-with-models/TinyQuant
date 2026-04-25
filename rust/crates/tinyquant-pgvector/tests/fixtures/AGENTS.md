# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-pgvector/tests/fixtures`

Committed test fixtures for the `tinyquant-pgvector` integration suite. These
files are golden data — do not regenerate or overwrite them automatically.

## What this area contains

- primary responsibility: store committed SQL schema migrations and JSON wire-format samples used by the integration tests
- main entrypoints: `0001_create_vectors_table.sql` (schema baseline), `pgvector_wire_100.json` (100-vector wire-format golden file)
- common changes: updating the SQL file when the schema evolves; replacing the JSON file when the wire format changes

## Layout

```text
fixtures/
├── 0001_create_vectors_table.sql
├── pgvector_wire_100.json
└── README.md
```

## Common workflows

### Update a fixture

1. Make the schema or wire-format change in the source crate first.
2. Regenerate the affected fixture by running the relevant test helper with `--bless` or by re-running `gen_corpus_fixture`.
3. Commit the updated fixture alongside the source change in the same PR.

### Add a new fixture

1. Confirm the fixture belongs here (SQL schema or wire-format golden data).
2. Add the file and update the layout section above.
3. Reference the new fixture from the appropriate test in `tests/adapter.rs` or `tests/smoke.rs`.

## Invariants — Do Not Violate

- Never auto-generate or silently overwrite fixture files; they are the ground-truth contract.
- SQL files must be valid PostgreSQL and must run against a pgvector-enabled instance.
- JSON fixtures must encode vectors at the dimension documented in their filename.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../../../AGENTS.md)
