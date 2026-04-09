---
title: Low Coupling
tags:
  - design
  - architecture
  - coupling
date-created: 2026-04-08
status: active
category: design
---

# Low Coupling

> [!info] Policy
> Dependencies in TinyQuant are explicit, narrow, stable, and acyclic.
> Changes to one module should not ripple through unrelated modules.

## Forms of coupling and how TinyQuant controls them

### Structural coupling (imports, calls, field access)

| Rule | Rationale |
|------|-----------|
| No import cycles between packages | Cycles make change order unpredictable |
| Domain modules never import infrastructure | DIP: domain depends on protocols, not concrete I/O |
| Cross-package imports use the public API (`__init__.py` exports) only | Internal modules are free to restructure without breaking consumers |
| Prefer value objects over mutable shared references at package boundaries | Immutable data cannot create action-at-a-distance |

**Enforcement:** `ruff` import rules + architecture tests that assert
`tinyquant_cpu.codec` does not import from `tinyquant_cpu.corpus` or
`tinyquant_cpu.backend`.

### Contract coupling (APIs, protocols, serialization formats)

| Rule | Rationale |
|------|-----------|
| `Protocol` classes define all cross-boundary contracts | Consumers depend on the protocol, not the implementation |
| Compressed vector format is versioned | Future codec changes must not silently break existing stored data |
| Backend adapters own their own wire-format translation | The corpus layer does not know how pgvector or brute-force formats work |

### Temporal coupling (ordering dependencies, synchronous chains)

| Rule | Rationale |
|------|-----------|
| Codebook must be trained before compression | Enforced by requiring a `Codebook` argument, not by hidden state |
| Corpus configuration is frozen at creation | No temporal dependency on "was config set before first insert?" |
| Decompression does not depend on the original insertion order | Any vector can be decompressed independently |

### Operational coupling (deployment, shared state)

TinyQuant is a library, not a service. Operational coupling is the
consumer's concern, not TinyQuant's. However:

| Rule | Rationale |
|------|-----------|
| No module-level mutable state | Library consumers should not encounter hidden singletons |
| No implicit file system or network access | All I/O goes through explicit parameters or protocol boundaries |
| Thread safety is documented per class | Consumers need to know what they can share across threads |

## Dependency direction

```text
tinyquant_cpu.codec    (leaf: no internal dependencies)
    ↑
tinyquant_cpu.corpus   (depends on codec value objects and service)
    ↑
tinyquant_cpu.backend  (depends on corpus for decompressed vector handoff)
```

This is a strict layered dependency. No upward or lateral imports.

## Coupling reduction techniques used

| Technique | Where applied |
|-----------|--------------|
| **Value objects at boundaries** | CodecConfig, CompressedVector, Codebook cross from codec to corpus as immutable data |
| **Protocol-based abstraction** | SearchBackend protocol decouples corpus from any specific search implementation |
| **Composition root** | Object wiring happens at the entry point, not inside domain classes |
| **Adapter pattern** | BackendAdapter translates between TinyQuant types and backend-specific wire formats |
| **No shared mutable state** | Codec is stateless; Corpus is the only stateful aggregate |

## Measurement signals

| Signal | Healthy | Investigate |
|--------|---------|-------------|
| Package cycle count | 0 | Any cycle |
| Cross-package import count | Low, through `__init__.py` only | Direct imports of internal modules |
| Fan-out per module | < 5 imports | > 8 imports suggests mixed responsibilities |
| Co-change frequency | Changes localized to one package | Changes spanning 3+ packages for one feature |

## See also

- [[architecture/high-coherence|High Coherence]]
- [[architecture/solid-principles|SOLID Principles]]
- [[architecture/namespace-and-module-structure|Namespace and Module Structure]]
- [[domain-layer/context-map|Context Map]]
