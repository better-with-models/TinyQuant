---
title: High Coherence
tags:
  - design
  - architecture
  - coherence
date-created: 2026-04-08
status: active
category: design
---

# High Coherence

> [!info] Policy
> Every module, package, and namespace in TinyQuant tells one consistent design
> story. Related behavior lives together. Unrelated behavior lives apart.

## Four levels of coherence

### Conceptual coherence

One term means one thing everywhere. The
[[domain-layer/ubiquitous-language|Ubiquitous Language]] is the authoritative
vocabulary. If a concept drifts (e.g. "codebook" starts meaning different
things in different modules), that is a design defect, not a naming
preference.

**Enforcement:** code review and grep. If the same concept has two names or
two concepts share one name, fix it before merge.

### Module coherence

Each module has one focused responsibility and one primary reason to change.
Signs of poor module coherence:

| Smell | Example | Fix |
|-------|---------|-----|
| **God module** | `utils.py` with 15 unrelated functions | Split by concept |
| **Feature envy** | Codec module reaching into Corpus internals | Move the behavior to the owner |
| **Shotgun surgery** | Adding a new bit width touches 8 files across 3 packages | Consolidate the variation point |
| **Unrelated imports** | A domain module importing `json` and `socket` | Push I/O behind a protocol |

**Enforcement:** the [[architecture/file-and-complexity-policy|one-class-per-file rule]] and the
[[architecture/namespace-and-module-structure|namespace structure]] make
module coherence structurally visible.

### Architectural coherence

Dependency direction is explicit and acyclic:

```text
tinyquant.codec  →  (no dependencies on corpus or backend)
tinyquant.corpus →  depends on codec (value objects and service)
tinyquant.backend → depends on corpus (decompressed vector handoff)
```

No cycles. No shared mutable state. No hidden side channels.

**Enforcement:** import linting rules in `ruff` or a dedicated architecture
test that asserts allowed dependency directions.

### Runtime coherence

The actual call graph matches the intended architecture. If the codec is
supposed to be stateless, it must not accumulate hidden state at runtime. If
the corpus is the consistency boundary, no code path should bypass it to
modify vectors directly.

**Enforcement:** tests that exercise real composition, not just mocked
interfaces.

## Coherence and namespaces

TinyQuant uses Python packages as coherence boundaries. The
[[architecture/namespace-and-module-structure|namespace structure]] is designed
so that:

- Each package corresponds to one bounded context
- Each module within a package corresponds to one domain concept
- Cross-package imports follow the dependency direction above
- `__init__.py` files define the package's public API surface

## What coherence is not

- It is not stylistic uniformity (consistent formatting is a linting concern,
  not a coherence concern)
- It is not maximal splitting (a cohesive 200-line module is more coherent
  than five 40-line fragments with hidden coupling)
- It is not code deduplication at all costs (two modules that look similar but
  change for different reasons should stay separate)

## See also

- [[architecture/low-coupling|Low Coupling]]
- [[architecture/namespace-and-module-structure|Namespace and Module Structure]]
- [[architecture/solid-principles|SOLID Principles]]
- [[domain-layer/context-map|Context Map]]
