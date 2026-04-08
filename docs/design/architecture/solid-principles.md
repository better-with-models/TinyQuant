---
title: SOLID Principles
tags:
  - design
  - architecture
  - solid
date-created: 2026-04-08
status: active
category: design
---

# SOLID Principles

> [!info] Policy
> TinyQuant uses SOLID as change-driven heuristics, not doctrinal
> compliance. Each principle is applied where it reduces the cost of the
> changes TinyQuant actually faces.

## How each principle applies to TinyQuant

### Single Responsibility Principle

**Question:** what independent change requests hit this unit?

| Unit | Responsibility | Violation signal |
|------|---------------|-----------------|
| `Codec` service | Compression and decompression math | Would violate SRP if it also managed persistence or batch orchestration |
| `Corpus` aggregate | Configuration consistency and vector lifecycle | Would violate SRP if it owned codebook training or search logic |
| `BackendAdapter` | Wire-format translation for one backend | Would violate SRP if it also handled ranking or caching |

**Rule for TinyQuant:** each class or module has one reason to change. If a
module mixes domain math with serialization, persistence, or orchestration,
split it.

### Open/Closed Principle

**Question:** when behavior changes, where do edits accumulate?

TinyQuant's primary extension points:

- **Compression policies** (`compress`, `passthrough`, `fp16`) — new policies
  should not require editing the Corpus aggregate's core logic
- **Backend adapters** — new backends should plug in through the
  `SearchBackend` protocol, not by modifying existing adapter code
- **Bit widths** — supporting a new bit width should be a configuration
  change, not a code change in the codec pipeline

**Rule for TinyQuant:** use `Protocol` classes and strategy objects for known
variation points. Do not extract extension seams speculatively.

### Liskov Substitution Principle

**Question:** can callers use the abstraction without knowing the concrete
subtype?

- Every `SearchBackend` implementation must accept FP32 vectors and return
  ranked results without requiring callers to special-case subtypes
- Every compression policy must produce a valid stored representation that the
  corpus can later decompress without policy-specific branching

**Rule for TinyQuant:** if a protocol implementation needs a special case at
the call site, the protocol is too wide. Narrow it.

### Interface Segregation Principle

**Question:** which consumers are forced to depend on members they never use?

- The codec layer should not expose corpus management methods
- The corpus layer should not expose backend search methods
- Read-only consumers (decompression path) should not depend on write-path
  interfaces (codebook training, batch insertion)

**Rule for TinyQuant:** keep interfaces role-focused. Prefer multiple narrow
protocols over one wide class interface.

### Dependency Inversion Principle

**Question:** which direction do the volatile dependencies point?

```text
Domain logic (codec math, corpus invariants)
    ↑ depends on abstractions (protocols)
Infrastructure (serialization, backend adapters, file I/O)
    ↑ implements those abstractions
```

- Domain code never imports infrastructure modules directly
- Infrastructure plugs into domain through `Protocol` interfaces
- Object construction happens at the composition root (entry point or factory),
  not inside domain classes

**Rule for TinyQuant:** if a domain module imports `json`, `pathlib`, `struct`,
`psycopg`, or any I/O library, that is a DIP violation. Move the I/O behind a
protocol.

## SOLID anti-patterns to avoid

- Extracting one-interface-per-class with no real alternate implementation
- Replacing a clear conditional with a hierarchy that obscures the business rule
- Splitting units so aggressively that collaboration becomes harder to follow
- Mistaking constructor injection for DIP — injection alone does not guarantee
  good dependency direction

## See also

- [[architecture/test-driven-development|Test-Driven Development]]
- [[architecture/low-coupling|Low Coupling]]
- [[architecture/high-coherence|High Coherence]]
- [[domain-layer/context-map|Context Map]]
