# Concepts

## Compiled knowledge layer

The compiled knowledge layer is the `docs/` directory. It holds the maintained
Obsidian wiki where durable summaries, concepts, entities, specs, and design
notes live.

**See also:** [docs/README.md](docs/README.md), [AGENTS.md](AGENTS.md)

## Raw evidence layer

The raw evidence layer includes `docs/research/` plus the live repository
outside `docs/` when code, configs, tests, or scripts are being analyzed as
source material for the wiki.

**See also:** [docs/research/llm-wiki.md](docs/research/llm-wiki.md),
[AGENTS.md](AGENTS.md)

## Authoritative Rust core

The authoritative Rust core is the `rust/` workspace. Shipped TinyQuant
behavior is defined there first, then surfaced through Python and JavaScript
bindings.

**See also:** [rust/README.md](rust/README.md), [rust/AGENTS.md](rust/AGENTS.md)

## Python shim

The Python shim is `src/tinyquant_cpu/`. It re-exports the Rust extension under
the `tinyquant_cpu` import name so editor tooling and fat-wheel packaging share
one public API surface.

**See also:** [src/README.md](src/README.md), [src/AGENTS.md](src/AGENTS.md)

## Frozen reference implementation

The frozen reference implementation is
`tests/reference/tinyquant_py_reference/`. It is a test-only differential
oracle pinned to the last pure-Python behavior, not a shipped product surface.

**See also:** [tests/reference/README.md](tests/reference/README.md),
[tests/reference/AGENTS.md](tests/reference/AGENTS.md)

## Parity gate

The parity gate is the collection of tests and fixtures that prove the Rust
implementation, Python shim, and frozen oracle stay behaviorally aligned where
alignment is promised.

**See also:** [tests/README.md](tests/README.md),
[tests/parity/README.md](tests/parity/README.md)

## Obsidian markdown boundary

Obsidian-specific markdown such as wikilinks, embeds, and callouts is allowed
inside `docs/` and should not appear in markdown files outside that vault.

**See also:** [README.md](README.md), [AGENTS.md](AGENTS.md)
