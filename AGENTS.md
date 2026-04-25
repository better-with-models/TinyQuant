# AGENTS.md — TinyQuant

TinyQuant is a Rust-native vector quantization codec for embedding compression — CPU SIMD, optional GPU acceleration, and Python/TypeScript bindings.

## Layout

```text
TinyQuant/
├── README.md                portable root README for PyPI and general renderers
├── .github/README.md        rich GitHub landing page
├── AGENTS.md                repo-wide agent contract
├── CLAUDE.md                short redirect to AGENTS.md
├── CHANGELOG.md             release history
├── COMPATIBILITY.md         cross-surface version alignment ledger
├── CONCEPTS.md              root glossary for repo-wide vocabulary
├── docs/                    Obsidian wiki plus immutable research sources
├── rust/                    authoritative shipping implementation
├── src/                     Python import shim for the Rust extension
├── tests/                   Python suites, parity tests, and frozen oracle
├── scripts/                 verification, packaging, and fixture automation
├── javascript/              npm package surfaces
├── experiments/             benchmark and research playgrounds
├── .github/                 CI, release automation, and GitHub-facing assets
└── .githooks/               versioned Git hooks
```

Use the closest local `AGENTS.md` before editing inside a subtree. Root guidance
stays high-level on purpose; directory-local contracts own the detail.

## Common Workflows

### Change product behavior

1. Treat [`rust/AGENTS.md`](rust/AGENTS.md) as the primary implementation
   contract when codec, corpus, backend, CLI, or file-format semantics change.
2. Keep [`src/AGENTS.md`](src/AGENTS.md) aligned only as a Python shim and
   editor-facing import surface, not as the system of record.
3. Keep the frozen oracle under
   [`tests/reference/AGENTS.md`](tests/reference/AGENTS.md) aligned only when a
   documented rollout plan explicitly requires it.
4. Run the narrowest relevant test gate first, then parity and broader suite
   checks before finishing.

### Change documentation

1. Markdown under `docs/` is an Obsidian vault; read
   [`docs/README.md`](docs/README.md) before editing wiki pages.
2. Raw sources under `docs/research/` are read-only after placement.
3. Update `docs/index.md` and `docs/log.md` when wiki pages change in a durable
   way.
4. Markdown outside `docs/` stays portable, ordinary, and compatible with
   `.markdownlint-cli2.jsonc`.

### Change release or automation surfaces

1. Keep [`scripts/AGENTS.md`](scripts/AGENTS.md),
   [`.github/AGENTS.md`](.github/AGENTS.md), and
   [`tests/packaging/AGENTS.md`](tests/packaging/AGENTS.md) aligned when
   packaging or CI behavior changes.
2. Update `CHANGELOG.md`, `COMPATIBILITY.md`, and any affected user-facing
   README files in the same change when release behavior moves.
3. Keep `CLAUDE.md` files as short redirects to sibling `AGENTS.md` files.

## Invariants — Do Not Violate

- `rust/` is the authoritative implementation of shipped TinyQuant behavior.
  `src/tinyquant_cpu/` is a developer shim, and
  `tests/reference/tinyquant_py_reference/` is a frozen differential oracle.
- If the project tagline, elevator paragraph, or headline benchmark numbers
  change in `README.md`, `.github/README.md`, `AGENTS.md`, or `CLAUDE.md`,
  update all four in the same commit.
- `docs/research/` is immutable after source placement.
- Markdown inside `docs/` may use Obsidian features; markdown outside `docs/`
  must not.
- Non-`docs/` markdown lint runs through `scripts/verify_pre_commit.py` and the
  CI `markdown-lint` job in `.github/workflows/ci.yml`.
- `docs/` vault lint runs through `markdownlint-obsidian` in
  `.pre-commit-config.yaml` and `.github/workflows/docs-lint.yml`.
- Every `.py` file opens with a module docstring. Every `.rs` file opens with a
  `//!` module docstring. Public symbols carry their own docstrings or doc
  comments.
- When a directory's structure, workflows, or safe-editing constraints change,
  update that directory's local `README.md` and `AGENTS.md` with the code.

## See Also

- [README.md](README.md)
- [docs/README.md](docs/README.md)
- [rust/AGENTS.md](rust/AGENTS.md)
- [src/AGENTS.md](src/AGENTS.md)
- [tests/AGENTS.md](tests/AGENTS.md)
- [scripts/AGENTS.md](scripts/AGENTS.md)
- [experiments/AGENTS.md](experiments/AGENTS.md)
- [.github/AGENTS.md](.github/AGENTS.md)
- [javascript/@tinyquant/core/AGENTS.md](javascript/@tinyquant/core/AGENTS.md)
