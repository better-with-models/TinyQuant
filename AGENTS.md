# AGENTS.md — TinyQuant

## About TinyQuant

> *CPU-only vector quantization codec for embedding storage compression.*

TinyQuant is a CPU-only vector quantization codec that compresses
high-dimensional embedding vectors to low-bit representations while
preserving cosine similarity rankings. It combines random orthogonal
preconditioning with two-stage scalar quantization and optional FP16
residual correction to hit 8× compression at 4-bit with Pearson ρ ≈ 0.998
and 95% top-5 recall on real OpenAI embeddings.

See [README.md](README.md) for the vanilla, PyPI-friendly landing page
and [.github/README.md](.github/README.md) for the rich GitHub-flavored
version (with logo hero, admonitions, and collapsible recipes) that
GitHub auto-prefers for the repo landing page.

## Repository overview

TinyQuant uses an LLM-maintained documentation vault under `docs/`. The
repository is documentation-first and implementation-plural: the shipping
engine is the Rust workspace under `rust/`, and a pure-Python reference
implementation lives under `tests/reference/tinyquant_py_reference/` as a
test-only differential oracle. The legacy `src/tinyquant_cpu/` tree is
gone as of Phase 23 — no source is shipped to PyPI from this branch; the
`tinyquant-cpu==0.1.1` wheel on PyPI remains as the last pure-Python
release. Phase 24 will reclaim the `tinyquant-cpu` name on PyPI with a
Rust-backed fat wheel.

The documentation system is explicitly based on the ideas in
`docs/research/llm-wiki.md`. Treat that file as the conceptual source for how
the repo's knowledge base should operate.

## Key directories

| Path | Purpose |
| ------ | ------- |
| `rust/` | Cargo workspace for the shipping Rust implementation (tinyquant-core, tinyquant-py, tinyquant-sys, tinyquant-cli). |
| `tests/reference/tinyquant_py_reference/` | Pure-Python reference implementation. Test-only oracle; never installed by end users. Frozen at `v0.1.1` behavior. |
| `tests/parity/` | Cross-implementation parity suite (`pytest -m parity`). Self-parity lives now; Rust-side parity is wired on in Phase 24. |
| `docs/` | LLM-maintained wiki (Obsidian vault). All markdown here uses Obsidian-flavored syntax unless explicitly exempted. |
| `docs/research/` | Raw source material. The LLM reads from these but never modifies them after placement. |
| `docs/entities/` | Wiki pages for concrete systems, datasets, services, libraries, or tools. |
| `docs/concepts/` | Wiki pages for abstract ideas, patterns, methods, and architectural principles. |
| `docs/sources/` | One-page summaries of ingested source documents. |
| `docs/comparisons/` | Side-by-side analyses, trade-off tables, and decision records. |
| `docs/behavior/` | Behavior specifications and acceptance criteria. |
| `docs/design/` | Design-level analysis and structured architecture notes. |
| `docs/specs/` | Specs, plans, and implementation-oriented documentation. |
| `docs/assets/` | Images and other binary attachments referenced by wiki pages. |

## Markdown policy

There are two markdown modes in this repository:

- **Inside `docs/`**: use rich Obsidian-flavored markdown
- **Outside `docs/`**: use ordinary markdown that follows strict
  markdownlint-compatible rules

This distinction is intentional. The `docs/` vault is optimized for Obsidian as
an LLM-maintained wiki; markdown elsewhere in the repo should stay portable,
plain, and lint-clean.

## docs/ is an Obsidian vault

The `docs/` directory is designed to be opened directly in Obsidian. Every
markdown file under `docs/` except files in `docs/research/` must use
Obsidian-flavored markdown:

- Use wikilinks: `[[Page Name]]` and `[[Page Name|display text]]`
- Add YAML frontmatter to every wiki page with at least `title`, `tags`, and
  `date-created`
- Use callouts with `> [!type]`
- Use `![[Page Name]]` and `![[image.png]]` for embeds
- Add Dataview-friendly fields such as `status`, `category`, or `source-count`
- Use fenced ` ```mermaid ` blocks for diagrams where useful

Files in `docs/research/` are raw sources. They may use any markdown flavor and
must not be modified after initial placement.

## LLM wiki pattern

The `docs/` directory follows the LLM Wiki pattern described in
`docs/research/llm-wiki.md`, adapted for TinyQuant:

1. Raw sources in `docs/research/` and the rest of the repository outside
   `docs/`, when those files are being analyzed as source material
2. The wiki in the rest of `docs/`
3. The schema in this file plus `docs/README.md`

### Raw data model

For TinyQuant, "raw" material includes both:

- **curated research inputs** placed in `docs/research/`
- **the live repository itself** outside `docs/` such as code, configs,
  scripts, tests, and operational files

The LLM should synthesize from both of those raw layers into the Obsidian wiki
under `docs/`. In other words, `docs/` is the compiled knowledge layer, while
`docs/research/` and the rest of the repo are the underlying evidence.

### Core operations

| Operation | What happens |
| --------- | ------------ |
| `ingest` | A new source is placed in `docs/research/`. The LLM creates a summary in `docs/sources/`, updates or creates wiki pages, and updates `docs/index.md` and `docs/log.md`. |
| `query` | The LLM reads `docs/index.md` to locate relevant pages, reads those pages, and synthesizes an answer. Valuable outputs can be filed back into the wiki. |
| `lint` | Health-check for orphan pages, missing cross-references, stale claims, contradictions, structural gaps, and code-doc drift between the wiki and the live repo. |

## Special files

| File | Role |
| ------ | ---- |
| `docs/index.md` | Catalog of wiki pages with summaries and metadata. Updated on each meaningful content addition. |
| `docs/log.md` | Append-only operational history for scaffolding, ingests, and maintenance work. |
| `docs/README.md` | Human-facing overview of the wiki structure and conventions. |

## Editing rules

- Never modify files in `docs/research/` after initial placement
- Always update `docs/index.md` and `docs/log.md` when adding or changing wiki
  pages
- Use wikilinks for internal references across wiki pages
- Add YAML frontmatter to every new wiki page
- Store images in `docs/assets/` and reference them with `![[filename.ext]]`
- Keep markdown outside `docs/` in ordinary markdown, not Obsidian-flavored
  markdown
- Treat non-`docs/` markdown as subject to strict markdownlint discipline

## Cross-file prose alignment

Four files describe TinyQuant's identity and must stay in sync:

- `README.md` — root-level, vanilla markdown, what ships to PyPI and
  what non-GitHub renderers see
- `.github/README.md` — rich GitHub-flavored landing page that GitHub
  auto-prefers for the repo page (with logo hero, admonitions,
  collapsibles)
- `AGENTS.md` — agent operating contract (this file)
- `CLAUDE.md` — Claude-specific redirect to `AGENTS.md`

**Rule:** if you edit the project tagline, elevator-pitch paragraph, or
headline benchmark numbers in any one of these files, propagate the same
change to the other three in the same commit. The canonical tagline
today is:

> *CPU-only vector quantization codec for embedding storage compression.*

The canonical elevator-pitch paragraph lives at the top of this file
under `## About TinyQuant`, in the lead of `README.md`, and in the
`[!NOTE]` TL;DR callout of `.github/README.md`. `CLAUDE.md` carries
the short tagline under its h1.

This rule exists because these four files are read by different
audiences (humans on GitHub, humans on PyPI, agents acting on the repo,
Claude sessions specifically) and drift erodes trust in the repo's
self-description.

## Documentation maturity standard

TinyQuant should use the `/well-documented` system at **full maturity** across
the entire repository, not just inside `docs/`.

That means:

- root-level documentation should remain accurate, specific, and current
- important subtrees should have documentation proportional to their
  responsibility and change risk
- code, configs, tests, and operational workflows should stay aligned with the
  docs that explain them
- documentation updates should accompany structural or behavioral code changes

Prefer evidence-first documentation work: reconcile docs to the code and repo
layout before adding new prose.

## Pre-commit verification

Documentation quality is part of pre-commit verification for this repository.

Before commit, verify at least the following:

- `/well-documented` expectations are satisfied at full maturity
- markdown outside `docs/` passes strict markdownlint rules
- links, paths, commands, and structural references in documentation still
  match the current repository
- code changes that affect behavior, architecture, layout, or workflows are
  reflected in the relevant docs

The Obsidian wiki inside `docs/` should still be checked for internal
consistency, but it is allowed to use Obsidian-specific constructs that would
not be valid under a strict ordinary-markdown markdownlint profile.

## Escalation cues

Pause and confirm before:

- Deleting or renaming files in `docs/research/`
- Restructuring the `docs/` directory layout
- Changing the frontmatter schema used across wiki pages
- Bulk-ingesting multiple sources in one pass without review
