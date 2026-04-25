# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-bruteforce/tests`

This directory contains the integration tests for `BruteForceBackend`. Tests use real fixture data from `fixtures/` to exercise the full ingest-and-search path. `backend.rs` contains behavioural integration tests (ingest, search, error conditions), and `smoke.rs` provides a minimal always-run check that the backend constructs and returns results. Changes here most often involve adding test cases for new error conditions, extending fixture coverage, or verifying correctness of updated similarity logic.

## What this area contains

- primary responsibility: integration tests for `BruteForceBackend` — `backend.rs` (full behavioural tests covering ingest, search, and error paths), `smoke.rs` (fast sanity check), `fixtures/` (test vector data)
- main entrypoints: `backend.rs` (primary test coverage), `smoke.rs` (always-run sanity check)
- common changes: adding test cases for new error variants or edge cases in search, updating fixture data when vector dimensions change

## Layout

```text
tests/
├── fixtures/
├── backend.rs
├── README.md
└── smoke.rs
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
