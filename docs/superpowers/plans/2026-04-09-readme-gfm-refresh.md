---
title: "README GitHub-Flavored Refresh Implementation Plan"
tags:
  - plan
  - docs
  - readme
  - branding
date-created: 2026-04-09
status: draft
category: documentation
---

# README GitHub-Flavored Refresh — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Work in the dedicated worktree created in Task 0.

**Goal:** Replace the existing root `README.md` with a rich, GitHub-flavored Markdown version that uses the TinyQuant logo assets and a modern GitHub-native visual vocabulary (hero `<picture>`, admonitions, collapsible details, TOC, comparison matrices), and propagate a canonical project tagline into `AGENTS.md` and `CLAUDE.md` with a new "Cross-file prose alignment" rule in `AGENTS.md`'s editing guide.

**Architecture:** Full rewrite of `README.md` with all factual substance preserved (benchmark numbers, code examples, research citations, install/build/reproduce steps, license). New narrative arc optimized for the GitHub landing-page reader: hook → proof → install → quickstart → how → recipes → research → dev. The tagline becomes a canonical string defined in one place (the README lead) and duplicated verbatim to `AGENTS.md`'s new "About TinyQuant" section and to `CLAUDE.md`'s lead. A new `## Cross-file prose alignment` subsection in `AGENTS.md` enforces that these three files stay in sync whenever project identity changes.

**Tech Stack:** GitHub-flavored Markdown, `<picture>` with `prefers-color-scheme`, GitHub admonitions (`> [!NOTE]`, `> [!TIP]`, `> [!WARNING]`, `> [!IMPORTANT]`), `<details>`/`<summary>` HTML, `markdownlint-cli2` (enforces strict rules outside `docs/`), local git worktree.

---

## Context

The current root `README.md` (309 lines, commit `b5ef955`) has strong content but a GitHub-unfriendly visual vocabulary: wall-of-prose opening, no logo, no admonitions, no collapsibles, no TOC, and a "Basic Usage" section that dumps six subsections of code back-to-back. Meanwhile, the project has a complete logo family under `docs/assets/` (wordmark + icon variants, light + dark modes, transparent PNG variants, SVG versions) that is currently unused in any surface-level doc.

`AGENTS.md` (149 lines) is a formal operating contract that defines the docs vault policy and editing rules but currently has no project-identity prose — no tagline, no one-line description, no link to the root README. `CLAUDE.md` (11 lines) is a thin redirect to `AGENTS.md`. There's no existing rule that the three files should agree on project identity, and today they don't — only `README.md` contains the elevator pitch.

User's explicit answers (2026-04-09):

- **Scope:** Full rewrite of `README.md`.
- **Tagline:** Canonicalize across all three files. Add a short "About TinyQuant" blurb to `AGENTS.md` mirroring the README lead, and the same blurb to `CLAUDE.md`.
- **Alignment note:** New `## Cross-file prose alignment` subsection in `AGENTS.md` directly under the existing `## Editing rules` section (currently at `AGENTS.md:96`).
- **Logo scope:** Logo lives in `README.md` only. `AGENTS.md` and `CLAUDE.md` stay text-only.
- **Base branch:** `origin/main` at `b5ef955` (confirmed explicitly: "you can just base off of main in your worktree").

---

## Canonical tagline

The plan establishes one canonical short tagline and one canonical one-paragraph description. These strings are used verbatim in all three files; any future change must touch all three (enforced by the new alignment rule).

**Short tagline** (one sentence, matches `pyproject.toml` `[project].description`):

> CPU-only vector quantization codec for embedding storage compression.

**Elevator-pitch paragraph** (one paragraph, ~60 words, appears as README lead prose and as AGENTS.md "About TinyQuant" blurb):

> TinyQuant is a CPU-only vector quantization codec that compresses
> high-dimensional embedding vectors to low-bit representations while
> preserving cosine similarity rankings. It combines random orthogonal
> preconditioning with two-stage scalar quantization and optional FP16
> residual correction to hit 8× compression at 4-bit with Pearson ρ ≈ 0.998
> and 95% top-5 recall on real OpenAI embeddings.

The pitch preserves the hard numbers from today's README opening, the method-level description ("random orthogonal preconditioning with two-stage scalar quantization and optional FP16 residual correction"), and the benchmark attribution. Any change to any of these facts must propagate to all three files.

---

## File Structure

Files this plan will **create**:

- `.worktrees/readme-refresh/` — new git worktree, branch `docs/readme-gfm-refresh`, based on `origin/main` (Task 0). Cleaned up at the end of Task 8 after the PR merges.

Files this plan will **modify** (all within the worktree):

- `README.md` — **full rewrite**. See the "New README.md structure" section below for the section-by-section spec.
- `AGENTS.md` — two targeted inserts: a new "About TinyQuant" blurb at the top (between the current h1 and `## Repository overview`), and a new `## Cross-file prose alignment` subsection immediately after `## Editing rules` (after line 106 in the current file).
- `CLAUDE.md` — add the canonical short tagline as a single-line blockquote directly under the current h1, keeping the rest of the existing redirect text intact.

Files this plan will **read but not modify**:

- `pyproject.toml` — source of truth for the short tagline (`[project].description`). Already says "CPU-only vector quantization codec for embedding storage compression." — the canonical short tagline matches verbatim.
- `docs/assets/tinyquant-logo-dark-transparent.png` — hero image for GitHub dark mode. Confirmed to exist; inspect in Task 2 to verify mark colors render correctly on a dark page background.
- `docs/assets/tinyquant-logo-light-transparent.png` — hero image for GitHub light mode. Same inspection in Task 2.
- `docs/assets/tinyquant-logo-dark.svg` / `docs/assets/tinyquant-logo-light.svg` — fallback SVG sources (opaque backgrounds; not the primary hero choice but documented as backup).
- `experiments/quantization-benchmark/results/plots/compression_vs_fidelity.png` — existing benchmark plot, already referenced in today's README. Preserved verbatim in the new README.
- `experiments/quantization-benchmark/REPORT.md` — linked from the new README "Benchmarks" section.
- `CHANGELOG.md`, `LICENSE` — referenced by the new README but not modified.

**Assets NOT used** in this plan (documented for future reference):

- `tinyquant-logo-bug-*.png` / `.svg` — icon-only "bug" variants. The user explicitly said the logo lives in `README.md` only, so the inline-heading icon use case is not needed. These assets stay unused.
- `tinyquant-logo-source.png` — 917 KB master source. Not used directly.

---

## New README.md structure

The new README is designed around four principles:

1. **Hook in 15 seconds.** Above-the-fold content answers "what is this?", "why should I care?", and "what are the numbers?" before the reader scrolls.
2. **Copy-paste install and quickstart.** The path from "landed on the page" to "I have it installed and running" is three code blocks, no paragraphs.
3. **Collapsibles for the long tail.** Detailed recipes, advanced usage, and research citations live inside `<details>` blocks so the linear scroll stays short.
4. **GitHub-native primitives over custom HTML.** Use `> [!NOTE]` admonitions, `<details>`, `<picture>`, and plain tables — no custom CSS, no JavaScript, no external dependencies.

Section order (new → old mapping):

| # | New section | Replaces (old) | Notes |
| --- | --- | --- | --- |
| 1 | Hero + badges + tagline | `# TinyQuant` + 4 badges + intro paragraphs | Adds `<picture>` logo with dark/light mode switching; adds a 5th badge for PyPI version; tagline becomes a single one-liner |
| 2 | `> [!NOTE]` TL;DR callout | _(new)_ | One sentence + 3 bullets (what it is, who it's for, the 8× number) |
| 3 | "The pitch" (benchmark table + plot) | `## Why TinyQuant` | Same numbers, tighter prose; benchmark plot preserved |
| 4 | `<details>` Table of Contents | _(new)_ | Auto-generated, 12 anchor links |
| 5 | Installation | `## Installation` | Replaced with a 3-row "Which install do I want?" table + code blocks per row, plus a `> [!TIP]` on the pgvector optional dep |
| 6 | Quickstart | `## Quickstart` | Same 20-line example; adds a `<details>` "Walk-through" that explains each step |
| 7 | How it works | _(new, synthesized from research lineage)_ | 4-paragraph explainer: rotation → two-stage scalar → residual → backend-agnostic. No new math, just unlocks the "why" for readers who bounced off research papers |
| 8 | Recipes | `## Basic Usage` (6 subsections) | Each subsection becomes a `<details>` block. Adds a rate-distortion comparison matrix at the top |
| 9 | Key properties | `## Key Properties` | Bulleted; tightened |
| 10 | Benchmarks | _(partially new; merges old "Reproducing the Benchmark")_ | One section for both the benchmark numbers and how to reproduce. Link to full REPORT.md |
| 11 | Research lineage | `## Research Lineage` | Moved later in the reading order; adds a compact comparison table |
| 12 | Development | `## Development` | Preserved; adds `> [!TIP]` about strict mypy + ruff + markdownlint gates |
| 13 | Repository layout | `## Repository Layout` | Preserved; table paths already point at `src/tinyquant_cpu/` |
| 14 | Contributing | _(new, brief)_ | 4 bullets: file issues, run the test suite, follow the SDLC in `docs/`, submit PRs |
| 15 | License | `## License` | Preserved |
| 16 | Related docs | `## Related Documentation` | Preserved |

Substance preservation (non-negotiable — every one of these must appear verbatim or near-verbatim in the new README):

- `8x compression` at 4-bit with `ρ = 0.998`, `95%` top-5 recall
- `16x compression` at 2-bit with `ρ = 0.964`, `85%` top-5 recall
- Benchmark row for `TinyQuant 4-bit + residual` at `ρ = 1.000`, `100%` recall
- `335 real embeddings` from OpenAI `text-embedding-3-small` (1536 dim)
- `5.7 GB → 732 MB` for 1M vectors
- Citations: TurboQuant (Google Research 2025), PolarQuant (2025), QJL (2024) with working links
- All current working code examples (single-vector, batch, policies, pgvector adapter)
- `208 tests, 90.95% coverage` (from the current Development section)
- Apache-2.0 license reference

---

## Task 0: Create the worktree and verify clean baseline

**Files:**
- Create: `.worktrees/readme-refresh/` (git worktree dir)
- New branch: `docs/readme-gfm-refresh` based on `origin/main`

**Why:** The user is currently on `chore/gh-actions-node24` doing parallel Node 24 upgrade work (4 unmerged commits) and should not be interrupted. A worktree lets this README work proceed in isolation. `.worktrees/` is already in `.gitignore` (confirmed via `grep -n "worktree" .gitignore` → line 66: `.worktrees/`).

- [ ] **Step 1: Confirm worktree prerequisites**

  ```bash
  cd C:/Users/aaqui/better-with-models/TinyQuant
  git fetch origin
  grep -n ".worktrees" .gitignore   # expect: 66:.worktrees/
  git worktree list                 # expect: primary on chore/gh-actions-node24
  git log --oneline origin/main -3  # expect: b5ef955 at the top
  ```

- [ ] **Step 2: Create the worktree**

  ```bash
  git worktree add -b docs/readme-gfm-refresh .worktrees/readme-refresh origin/main
  cd .worktrees/readme-refresh
  git log --oneline -3              # expect: b5ef955 at the top
  git status                        # expect: clean
  ```

- [ ] **Step 3: Install the project into a throwaway venv inside the worktree for verification later**

  ```bash
  python -m venv .venv
  source .venv/Scripts/activate     # Windows bash
  pip install --quiet --upgrade pip
  pip install --quiet -e ".[dev]"
  python -c "import tinyquant_cpu; print(tinyquant_cpu.__version__)"
  deactivate
  ```

  Expected: prints `0.1.0`. This is the baseline sanity check; it proves the worktree is a working checkout before we start editing prose.

- [ ] **Step 4: Run markdownlint on the current README as a baseline**

  ```bash
  npx --yes markdownlint-cli2 README.md AGENTS.md CLAUDE.md
  ```

  Expected: PASS (or else the pre-existing files already have a lint error, in which case note which rule and carry on — this is a baseline, not a gate).

---

## Task 1: Verify the hero logo assets render correctly

**Files:**
- Read: `docs/assets/tinyquant-logo-dark-transparent.png`
- Read: `docs/assets/tinyquant-logo-light-transparent.png`

**Why:** The SVG naming convention was verified earlier: `-dark` = designed FOR dark-mode display (white/light-colored marks); `-light` = designed FOR light-mode display (navy/dark-colored marks). The opaque SVG files have background rects (`#1f1d22` and `#f5f4f5`) which would clash slightly with GitHub's actual theme colors (`#0d1117` / `#ffffff`). The transparent PNGs should give a cleaner blend. But the Explore agent didn't verify the transparent PNG mark colors, so confirm them before committing the `<picture>` tag.

- [ ] **Step 1: Open both transparent PNGs in the Read tool and visually verify mark color**

  Use the Read tool (it supports PNG images) on both files. Expected:
  - `tinyquant-logo-dark-transparent.png` → WHITE / light-colored TinyQuant wordmark + icon, no background. Should be visible against a dark page.
  - `tinyquant-logo-light-transparent.png` → NAVY (`#22325f`-ish) wordmark + icon, no background. Should be visible against a white page.

  If either file doesn't match that expectation, stop and investigate — the naming convention may not carry over to the transparent PNGs. Fallback plan: use the opaque SVGs (`tinyquant-logo-dark.svg` + `tinyquant-logo-light.svg`) in the `<picture>` tag instead. Document which choice was made in the next step's commit message.

- [ ] **Step 2: Decide hero asset pair**

  Default choice (pending verification): `tinyquant-logo-dark-transparent.png` for dark mode, `tinyquant-logo-light-transparent.png` for light mode.

  Fallback: `tinyquant-logo-dark.svg` + `tinyquant-logo-light.svg`.

  Record the choice in a one-line comment inside the new README's `<picture>` block for future maintainers.

---

## Task 2: Draft and write the new README.md

**Files:**
- Modify: `README.md` (full rewrite; ~400–500 lines expected, up from 309)

**Why:** User chose "Full rewrite". This is the biggest task in the plan and should be done as a single atomic commit — partial rewrites of a README are hard to review. The content spec is in the "New README.md structure" section above; this task is the execution of that spec.

- [ ] **Step 1: Replace `README.md` with the new content in full**

  Use the Write tool (not Edit) to overwrite `README.md` with the new content. The file must include, in order:

  1. **Hero block** — `<div align="center">` wrapping a `<picture>` element with `prefers-color-scheme` switching. Below the logo: h1 `# TinyQuant`, the canonical short tagline as italic text, and the badge row. Close `</div>`.

     ```markdown
     <div align="center">

     <picture>
       <source media="(prefers-color-scheme: dark)" srcset="docs/assets/tinyquant-logo-dark-transparent.png">
       <img src="docs/assets/tinyquant-logo-light-transparent.png" alt="TinyQuant" width="420">
     </picture>

     # TinyQuant

     *CPU-only vector quantization codec for embedding storage compression.*

     [![PyPI](https://img.shields.io/pypi/v/tinyquant-cpu.svg)](https://pypi.org/project/tinyquant-cpu/)
     [![CI](https://github.com/better-with-models/TinyQuant/actions/workflows/ci.yml/badge.svg)](https://github.com/better-with-models/TinyQuant/actions/workflows/ci.yml)
     [![Python 3.12+](https://img.shields.io/badge/python-3.12+-blue.svg)](https://www.python.org/downloads/)
     [![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

     </div>
     ```

     Notes:
     - The `PyPI` badge is NEW — replaces the old `Version` badge because the package is actually published now.
     - `width="420"` keeps the hero visually bounded.
     - The `<picture>` alt text goes on the `<img>` fallback, not the `<source>` children.

  2. **TL;DR callout** — a `> [!NOTE]` admonition with the canonical elevator-pitch paragraph (see "Canonical tagline" above) and three sub-bullets:
     - **What it is:** a Python library that squeezes embedding vectors into 4-bit (or 2-bit) representations without losing retrieval quality.
     - **Who it's for:** teams running cosine-similarity search on embeddings and paying for RAM or disk by the gigabyte.
     - **Headline number:** 8× compression at 95% top-5 recall on real OpenAI embeddings.

  3. **The pitch** — `## The pitch` heading (or `## At a glance`). Preserves the existing benchmark table verbatim. Preserves the `5.7 GB → 732 MB` line. Preserves the `![Compression vs. Fidelity](experiments/quantization-benchmark/results/plots/compression_vs_fidelity.png)` image.

  4. **Table of contents** — `<details><summary>Contents</summary>` with a bulleted nested TOC that links to every h2 below.

  5. **Installation** — Replace the three bare `pip install` blocks with a decision table:

     ```markdown
     | I want to... | Install command |
     | --- | --- |
     | Try it out | `pip install tinyquant-cpu` |
     | Use it with PostgreSQL + pgvector | `pip install "tinyquant-cpu[pgvector]"` |
     | Contribute or run the test suite | `pip install "tinyquant-cpu[dev]"` |
     ```

     Follow with a `> [!TIP]` explaining that `[pgvector]` pulls `psycopg[binary]` and Python 3.12+ is required.

  6. **Quickstart** — The existing ~20-line end-to-end example, unchanged. Below it, a `<details><summary>What just happened?</summary>` with a line-by-line walk-through of the 6 phases (config → train → corpus → insert → decompress → search).

  7. **How it works** — NEW section. Four short paragraphs:
     1. *The problem* — naive scalar quantization destroys inner products because coordinate distributions are skewed.
     2. *The trick* — pre-multiplying by a random orthogonal matrix (QR-derived) uniformizes the coordinate distribution, so a shared scalar quantizer works across all dimensions.
     3. *Two-stage refinement* — a coarse codebook plus an optional FP16 residual gives 8×/16× compression at different points on the rate-distortion curve.
     4. *Backend-agnostic* — the codec just produces `CompressedVector` bytes; search lives in a separate layer (`BruteForceBackend` or the `PgvectorAdapter`), so you can plug it into any retrieval store.

     Reference links: TurboQuant, PolarQuant, QJL at the bottom of the section (reuse the existing URLs).

  8. **Recipes** — Each of the current 6 "Basic Usage" subsections becomes a `<details>` block. Order:
     1. Single-vector compression
     2. Batch compression
     3. Rate-distortion tuning (add a comparison table: bit width × residual × compression × recall)
     4. Compression policies (COMPRESS, PASSTHROUGH, FP16)
     5. Binary serialization
     6. PostgreSQL + pgvector backend

     Preface the recipes with the rate-distortion comparison matrix:

     ```markdown
     | Config | Bytes/vec | Compression | ρ | Top-5 Recall | When to use |
     | --- | ---: | ---: | ---: | ---: | --- |
     | `CodecConfig(bit_width=4)` | 768 | 8× | 0.998 | 95% | **Default** — best balance of size and quality |
     | `CodecConfig(bit_width=2)` | 384 | 16× | 0.964 | 85% | Aggressive compression, degraded recall |
     | `CodecConfig(bit_width=4, residual_enabled=True)` | 3840 | 1.6× | 1.000 | 100% | Reranking or high-precision search |
     ```

  9. **Key properties** — Preserve the existing bullet list, tighten phrasing. Keep the Apache-2.0 bullet.

  10. **Benchmarks** — merge the old "Why TinyQuant" benchmark details with the old "Reproducing the Benchmark" steps into one section. The table stays in "The pitch" section; this one is about methodology and reproduction.

  11. **Research lineage** — Preserve the 3 citations. Add a compact comparison table:

      ```markdown
      | Method | Year | Key contribution |
      | --- | :-: | --- |
      | [TurboQuant] | 2025 | Random rotation + scalar quantization, no block norms |
      | [PolarQuant] | 2025 | QR-derived orthogonal preconditioning |
      | [QJL] | 2024 | Theoretical inner-product preservation bounds |
      ```

  12. **Development** — Preserve. Add a `> [!TIP]` about the three strict gates (ruff, mypy, markdownlint).

  13. **Repository layout** — Preserve the existing table, paths already point at `src/tinyquant_cpu/`.

  14. **Contributing** — NEW brief section. 4 bullets. Reference `AGENTS.md` and `CLAUDE.md` for agent-assisted contributions. Reference `docs/` for design and SDLC conventions.

  15. **License** — One line. Link to `LICENSE`.

  16. **Related documentation** — Preserve the existing links.

- [ ] **Step 2: Run markdownlint on the new README**

  ```bash
  npx --yes markdownlint-cli2 README.md
  ```

  Expected: PASS. If it fails, read the rule name in the output, fix the offending markdown, and re-run until clean. Common issues to watch for:
  - `MD013` (line length) — the pre-commit hook config in this repo disables MD013 for `docs/**` but NOT for root-level files. Wrap long lines at 80 cols or use `<!-- markdownlint-disable MD013 -->` for tables.
  - `MD033` (inline HTML) — `<picture>`, `<source>`, `<img>`, `<details>`, `<summary>`, `<div align>` are all inline HTML. Add an explicit `<!-- markdownlint-disable MD033 -->` at the top of the file if needed.
  - `MD041` (first line h1) — the `<div align>` wrapper means the h1 is NOT on line 1. May need to either move the h1 up or add a `<!-- markdownlint-disable MD041 -->` comment.

- [ ] **Step 3: Commit the README rewrite**

  ```bash
  git add README.md
  git commit -m "docs(readme): full GitHub-flavored rewrite with logo hero and admonitions"
  ```

  The commit message body should summarize the structural changes (hero, TL;DR callout, TOC, recipes in `<details>`, comparison tables, canonical tagline) and explicitly note that all benchmark numbers and working code examples are preserved verbatim.

---

## Task 3: Update `AGENTS.md` with "About TinyQuant" blurb and alignment rule

**Files:**
- Modify: `AGENTS.md` — add a new "About TinyQuant" section at the top, add a new `## Cross-file prose alignment` section after `## Editing rules`.

**Why:** The user chose to canonicalize the tagline across all three files, which requires AGENTS.md to actually have prose to align. And the new alignment rule needs a home in the editing guide — the user specifically chose "New subsection in AGENTS.md under Editing rules".

- [ ] **Step 1: Add the "About TinyQuant" blurb at the top**

  Insert between the existing h1 `# AGENTS.md` (or whatever the current h1 is) and the first `## Repository overview`:

  ```markdown
  ## About TinyQuant

  > *CPU-only vector quantization codec for embedding storage compression.*

  TinyQuant is a CPU-only vector quantization codec that compresses
  high-dimensional embedding vectors to low-bit representations while
  preserving cosine similarity rankings. It combines random orthogonal
  preconditioning with two-stage scalar quantization and optional FP16
  residual correction to hit 8× compression at 4-bit with Pearson ρ ≈ 0.998
  and 95% top-5 recall on real OpenAI embeddings.

  See [README.md](README.md) for installation, quickstart, and recipes.
  ```

  This paragraph matches the canonical elevator-pitch paragraph from the "Canonical tagline" section above **verbatim**. Any future edit to this text must also edit `README.md` and `CLAUDE.md`.

- [ ] **Step 2: Add the `## Cross-file prose alignment` subsection**

  Insert immediately after the existing `## Editing rules` section (which ends around line 106 in the current file, just before `## Documentation maturity standard`):

  ```markdown
  ## Cross-file prose alignment

  Three files describe TinyQuant's identity and must stay in sync:

  - `README.md` — public-facing landing page
  - `AGENTS.md` — agent operating contract (this file)
  - `CLAUDE.md` — Claude-specific redirect to `AGENTS.md`

  **Rule:** if you edit the project tagline, elevator-pitch paragraph, or
  headline benchmark numbers in any one of these files, propagate the same
  change to the other two in the same commit. The canonical tagline today
  is:

  > *CPU-only vector quantization codec for embedding storage compression.*

  The canonical elevator-pitch paragraph lives at the top of this file
  under `## About TinyQuant` and in the lead of `README.md`. `CLAUDE.md`
  carries the short tagline under its h1.

  This rule exists because these three files are read by different
  audiences (humans on GitHub, agents acting on the repo, Claude sessions
  specifically) and drift erodes trust in the repo's self-description.
  ```

- [ ] **Step 3: Run markdownlint on AGENTS.md**

  ```bash
  npx --yes markdownlint-cli2 AGENTS.md
  ```

  Expected: PASS. AGENTS.md today is already clean per the Explore agent's report; the additions are standard h2 sections with no inline HTML.

- [ ] **Step 4: Commit the AGENTS.md changes**

  ```bash
  git add AGENTS.md
  git commit -m "docs(agents): add About TinyQuant blurb and cross-file prose alignment rule"
  ```

---

## Task 4: Update `CLAUDE.md` with the canonical tagline

**Files:**
- Modify: `CLAUDE.md` (11 lines → ~14 lines)

**Why:** User chose to canonicalize the tagline across all three files. CLAUDE.md currently has no tagline. Add a one-line blockquote under the h1, leave the existing redirect text intact.

- [ ] **Step 1: Insert the canonical short tagline**

  Replace the current top of `CLAUDE.md`:

  ```markdown
  # CLAUDE.md

  This repository uses [AGENTS.md](AGENTS.md) as the primary operating contract.
  ```

  With:

  ```markdown
  # CLAUDE.md

  > *CPU-only vector quantization codec for embedding storage compression.*

  This repository uses [AGENTS.md](AGENTS.md) as the primary operating contract.
  ```

  Keep the rest of the file (the 4-bullet "read this for" list) exactly as-is.

- [ ] **Step 2: Run markdownlint on CLAUDE.md**

  ```bash
  npx --yes markdownlint-cli2 CLAUDE.md
  ```

  Expected: PASS.

- [ ] **Step 3: Commit the CLAUDE.md change**

  ```bash
  git add CLAUDE.md
  git commit -m "docs(claude): add canonical tagline under h1"
  ```

---

## Task 5: Final validation — markdownlint, pre-commit hook, tagline consistency check

**Files:** none modified.

**Why:** Three files were touched with a canonical tagline that must match verbatim. A tiny validation script catches drift before it ships. Also re-run the pre-commit hook since the earlier commits triggered it individually; running it at the end catches cross-file issues.

- [ ] **Step 1: Run the repo's pre-commit hook manually**

  ```bash
  bash .githooks/pre-commit || echo "pre-commit hook exited $?"
  ```

  Expected: PASS. Watch for the `markdownlint passed for markdown outside docs/` message — this is the gate for the files we edited.

- [ ] **Step 2: Verify the canonical short tagline appears in all three files, verbatim**

  ```bash
  TAG="CPU-only vector quantization codec for embedding storage compression."
  echo "--- README.md ---"; grep -c -F "$TAG" README.md
  echo "--- AGENTS.md ---"; grep -c -F "$TAG" AGENTS.md
  echo "--- CLAUDE.md ---"; grep -c -F "$TAG" CLAUDE.md
  ```

  Expected: each prints a count `>= 1`. If any print `0`, the tagline drifted during drafting — fix the offending file and amend the relevant commit.

- [ ] **Step 3: Verify the elevator-pitch paragraph's key phrase ("random orthogonal preconditioning with two-stage scalar quantization") appears in README.md and AGENTS.md (not CLAUDE.md; CLAUDE.md only carries the short tagline)**

  ```bash
  PHRASE="random orthogonal preconditioning with two-stage scalar quantization"
  grep -c -F "$PHRASE" README.md AGENTS.md
  ```

  Expected: both files return count `>= 1`. `CLAUDE.md` is excluded intentionally.

- [ ] **Step 4: Visual spot-check of the new README**

  ```bash
  wc -l README.md               # expect ~400-500 lines
  grep -c "^## " README.md      # count of h2 sections — expect 12-16
  grep -c "^### " README.md     # count of h3 sections (collapsibles may use headings) — expect 0-6
  grep -c "<details>" README.md # expect 6-8 (recipes + TOC + walk-through)
  grep -c "> \[!" README.md     # count of admonitions — expect 2-4
  grep -c "<picture>" README.md # expect exactly 1 (the hero)
  ```

  Expected ranges above. If any are wildly off, re-read the structure spec above and adjust.

---

## Task 6: Push branch, open PR, visual review on GitHub

**Files:** none modified.

**Why:** The markdown-rendering validation that matters most is what actually shows up on the GitHub PR page. That's the only place `<picture>` + `prefers-color-scheme`, admonitions, and `<details>` all render correctly in one view.

- [ ] **Step 1: Push the branch to origin**

  ```bash
  git push -u origin docs/readme-gfm-refresh
  ```

- [ ] **Step 2: Open the PR with a descriptive body**

  ```bash
  gh pr create --base main --head docs/readme-gfm-refresh \
    --title "docs: GitHub-flavored README rewrite with logo hero and canonical tagline" \
    --body "$(cat <<'EOF'
  ## Summary

  Full rewrite of the root README.md into a GitHub-native landing page:
  `<picture>` hero with light/dark logo switching, TL;DR admonition,
  collapsible TOC, recipes-as-`<details>`, a new "How it works"
  explainer, and a rate-distortion comparison matrix.

  All benchmark numbers, working code examples, research citations, and
  reproduction steps are preserved verbatim from the previous version.
  The package name (`tinyquant-cpu`) and import name (`tinyquant_cpu`)
  are consistent with PyPI.

  ## Cross-file prose alignment

  Canonicalized the project tagline across README.md, AGENTS.md, and
  CLAUDE.md. A new `## Cross-file prose alignment` rule in AGENTS.md
  (under `## Editing rules`) enforces that future edits to the tagline
  or elevator-pitch paragraph propagate to all three files in the same
  commit.

  - **Short tagline** (in all three files): *CPU-only vector
    quantization codec for embedding storage compression.*
  - **Elevator-pitch paragraph** (in README.md lead and AGENTS.md
    `## About TinyQuant`): full 60-word description including rotation,
    two-stage quantization, residual, and the 8×/ρ=0.998/95% recall
    headline numbers.

  ## Logo assets used

  - \`docs/assets/tinyquant-logo-dark-transparent.png\` → GitHub dark mode
  - \`docs/assets/tinyquant-logo-light-transparent.png\` → GitHub light mode

  (See the commit for the exact \`<picture>\` block. SVGs are available as
  a fallback but were not chosen because the transparent PNGs blend more
  cleanly with GitHub's actual theme backgrounds.)

  ## Verification

  - [x] \`markdownlint-cli2 README.md AGENTS.md CLAUDE.md\` passes
  - [x] \`.githooks/pre-commit\` passes locally
  - [x] Canonical short tagline present in all three files (verified via grep)
  - [x] Elevator-pitch key phrase present in README.md and AGENTS.md
  - [ ] Hero \`<picture>\` renders correctly in GitHub light mode (spot-check on this PR)
  - [ ] Hero \`<picture>\` renders correctly in GitHub dark mode (toggle and spot-check)
  - [ ] All \`<details>\` collapsibles render and expand correctly
  - [ ] Admonitions render with the correct icon and color

  ## Test plan

  - [ ] Open this PR in browser, toggle light/dark mode, verify the logo flips
  - [ ] Click each \`<details>\` and confirm content expands
  - [ ] Verify every link in the new README resolves (404 check)
  - [ ] Confirm the benchmark plot image still loads

  🤖 Generated with [Claude Code](https://claude.com/claude-code)
  EOF
  )"
  ```

- [ ] **Step 3: Open the PR in the browser and visually verify**

  ```bash
  gh pr view --web
  ```

  Visual checklist (interactive, not automatable):
  1. Hero logo present, correct size, correct orientation
  2. Toggle GitHub dark mode (via profile menu) — logo flips to the dark-transparent variant
  3. TL;DR admonition has a blue `[!NOTE]` icon and badge
  4. Benchmark table renders with right-aligned numeric columns
  5. Benchmark plot PNG loads
  6. TOC `<details>` expands and each link jumps to the right anchor
  7. All 6 recipe `<details>` expand and show syntax-highlighted Python
  8. Rate-distortion comparison table renders correctly (right-aligned numerics)
  9. Research lineage comparison table renders
  10. No raw HTML escaping visible anywhere (if you see literal `<picture>` or `<details>` text, the HTML didn't get recognized — check markdownlint disable comments)

---

## Task 7: Merge and clean up the worktree

**Files:** none modified on this branch. Primary worktree (`chore/gh-actions-node24`) is untouched throughout.

**Why:** Worktrees aren't free — they occupy disk and git has to track them. Clean them up as soon as the PR merges. Per the `superpowers:finishing-a-development-branch` skill pattern.

- [ ] **Step 1: Wait for CI to pass on the PR**

  ```bash
  gh pr checks --watch --interval 20
  ```

  Expected: lint, markdown-lint, typecheck, tests (matrix 3.12/3.13), pgvector integration, build-artifact — all green. The test jobs should be unaffected by doc-only changes.

- [ ] **Step 2: (Operator decision) merge the PR**

  ```bash
  gh pr merge --squash --delete-branch
  ```

  Or leave it open for human review, depending on how the user wants to sign off. This step is operator-gated.

- [ ] **Step 3: Remove the worktree**

  ```bash
  cd C:/Users/aaqui/better-with-models/TinyQuant    # back to primary
  git worktree remove .worktrees/readme-refresh
  git worktree list                                  # expect: only the primary
  ```

  If `git worktree remove` complains about uncommitted changes or the `.venv/` inside the worktree, use `git worktree remove --force` — the branch is merged and anything left behind is disposable.

- [ ] **Step 4: Clean up local branch references**

  ```bash
  git branch -d docs/readme-gfm-refresh 2>/dev/null || echo "already gone"
  git remote prune origin
  ```

---

## Verification

End-to-end correctness of this plan is proven when **all** of the following hold:

1. **Markdownlint:** `npx markdownlint-cli2 README.md AGENTS.md CLAUDE.md` returns `0 error(s)` (Task 2/3/4/5).
2. **Pre-commit hook:** `.githooks/pre-commit` exits 0 on the final tree (Task 5).
3. **Tagline consistency:** `grep -c -F "CPU-only vector quantization codec for embedding storage compression."` returns `>= 1` for each of `README.md`, `AGENTS.md`, `CLAUDE.md` (Task 5).
4. **Elevator-pitch consistency:** the phrase "random orthogonal preconditioning with two-stage scalar quantization" appears in both `README.md` and `AGENTS.md` (Task 5).
5. **Numeric substance preserved:** all headline numbers from today's README — `8x`, `16x`, `ρ = 0.998`, `ρ = 0.964`, `95%`, `85%`, `335`, `1536`, `5.7 GB`, `732 MB`, `208 tests`, `90.95%` — appear verbatim in the new README.
6. **Logo rendering:** the `<picture>` block resolves to a visible hero image in both GitHub light mode and GitHub dark mode on the PR page (Task 6).
7. **Collapsibles render:** all `<details>` blocks in the new README expand/collapse correctly when clicked on the PR page (Task 6).
8. **No stale references:** no occurrence of the old package name `tinyquant` (lowercase, without `-cpu`/`_cpu` suffix) in the new README.

If any of (1)–(5) fail, fix locally and amend the relevant commit. If (6)–(7) fail, that usually indicates a markdown escaping issue or a missing `markdownlint-disable` pragma; add the pragma and re-push.

---

## Critical files (quick reference)

| Path | Role |
|---|---|
| `README.md` | Primary target of Task 2 — full rewrite |
| `AGENTS.md` | Target of Task 3 — add "About TinyQuant" blurb + "Cross-file prose alignment" section |
| `CLAUDE.md` | Target of Task 4 — add canonical tagline under h1 |
| `pyproject.toml` | Source of truth for the short tagline (unchanged) |
| `docs/assets/tinyquant-logo-dark-transparent.png` | Hero image (dark mode) |
| `docs/assets/tinyquant-logo-light-transparent.png` | Hero image (light mode) |
| `experiments/quantization-benchmark/results/plots/compression_vs_fidelity.png` | Benchmark plot (referenced, not modified) |
| `.worktrees/readme-refresh/` | Isolated worktree for this branch |
| `.githooks/pre-commit` | Pre-commit hook invoked in Task 5 for final validation |
