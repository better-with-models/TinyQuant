---
title: Wiki Log
tags:
  - meta
  - log
date-created: 2026-04-08
status: active
---

# Log

> [!info] Operation history
> Append-only record of documentation-system changes. Use the format
> `## [YYYY-MM-DD] operation | description`.

## [2026-04-08] init | Documentation system scaffolding created

Initialized the TinyQuant documentation system using the same structural model
as TurboSwede, without carrying over TurboSwede-specific content:

- Added root `AGENTS.md` to define the wiki schema and operating rules
- Standardized `docs/` as an Obsidian vault with shared settings in
  `.obsidian/app.json`
- Created top-level wiki control pages: [[README]], [[index]], and [[log]]
- Added documentation directories for `entities/`, `concepts/`, `sources/`,
  `comparisons/`, `behavior/`, `design/`, `specs/`, and `assets/`
- Added placeholder structure for `design/domain-layer/` and `specs/plans/`
- Preserved `research/` as the immutable raw-source area

## [2026-04-08] ingest | TinyQuant research seed

Ingested the first TinyQuant-specific research materials from
`docs/research/tinyquant-research/` into the wiki:

- Created [[tinyquant-library-research]] as a source summary for the library
  research seed
- Created [[tinyquant-better-router-integration]] as a source summary for the
  better-router and TurboSwede integration plan
- Created [[TinyQuant]] as the first project entity page
- Created [[storage-codec-vs-search-backend-separation]] and
  [[per-collection-compression-policy]] as core concept pages
- Created [[storage-codec-architecture]] as the first project design page
- Updated [[index]] to register all new source, entity, concept, and design
  pages

The research establishes TinyQuant as a CPU-only, clean-room embedding storage
codec with decompression-first search integration and per-collection rollout
policies for downstream systems.

## [2026-04-08] ingest | TurboQuant research pack

Ingested the broader TurboQuant-related research set from `docs/research/`:

- Created source summaries for [[vector-quantization-paper-synthesis]],
  [[turbo-quant-deep-research-report]],
  [[turbo-quant-code-availability]], and [[turboquant-integration]]
- Added upstream method entity pages for [[TurboQuant]], [[PolarQuant]], and
  [[QJL]]
- Added concept pages for
  [[random-preconditioning-without-normalization-overhead]] and
  [[two-stage-quantization-and-residual-correction]]
- Updated [[TinyQuant]] and [[storage-codec-architecture]] so the project is
  anchored in the full upstream research lineage rather than only the local
  TinyQuant research seed
- Updated [[index]] to register the additional sources, entities, and concepts

The wiki now captures both the local TinyQuant adaptation story and the broader
research lineage that motivates its architecture, provenance discipline, and
deployment boundaries.
