# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-core/src`

**BOOTSTRAP NOTE:** replace this opening paragraph with what this area is responsible for, who depends on it, and the kinds of changes that most often happen here.

## What this area contains

- primary responsibility: replace with the main job of this directory
- main entrypoints: replace with the files or subdirectories an agent should open first
- common changes: replace with the edits that usually happen here

## Layout

```text
src/
├── backend/
├── codec/
├── corpus/
├── errors.rs
├── lib.rs
├── prelude.rs
├── README.md
└── types.rs
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
