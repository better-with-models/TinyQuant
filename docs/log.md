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
