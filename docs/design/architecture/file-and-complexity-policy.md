---
title: File and Complexity Policy
tags:
  - design
  - architecture
  - complexity
  - policy
date-created: 2026-04-08
status: active
category: design
---

# File and Complexity Policy

> [!info] Policy
> One public class per file. Low cyclomatic complexity. These are hard limits
> enforced by linting, not soft preferences.

## One class per file

Every public class lives in its own file, named after the class in
`snake_case`:

```text
tinyquant/codec/codec_config.py     → class CodecConfig
tinyquant/codec/rotation_matrix.py  → class RotationMatrix
tinyquant/codec/codebook.py         → class Codebook
tinyquant/codec/compressed_vector.py → class CompressedVector
tinyquant/codec/codec.py            → class Codec
tinyquant/corpus/corpus.py          → class Corpus
tinyquant/corpus/vector_entry.py    → class VectorEntry
tinyquant/corpus/compression_policy.py → class CompressionPolicy
```

### Why

- **Discoverability:** the file name tells you exactly what it contains
- **Merge safety:** changes to unrelated classes never conflict in the same
  file
- **Blame clarity:** `git blame` on a file tracks one concept's history
- **Test correspondence:** `tests/codec/test_codec_config.py` maps directly
  to `tinyquant/codec/codec_config.py`

### Exceptions

- Small private helper classes (e.g. a `_QuantizationState` used only inside
  `codec.py`) may colocate with their consumer
- Module-level constants, type aliases, and re-exports in `__init__.py` are
  fine
- Test files may contain multiple test classes when they test one unit from
  different angles

## Cyclomatic complexity limits

| Scope | Maximum CC | Enforcement |
|-------|-----------|-------------|
| Function or method | **7** | `ruff` rule `C901` with `max-complexity = 7` |
| Class | **15** (sum of all methods) | Monitored; split class if sum exceeds limit |

### Why 7

A function with CC > 7 has too many independent paths to hold in working
memory. Research consistently shows that defect density rises sharply above
this threshold.

### How to stay under the limit

| Technique | When to use |
|-----------|------------|
| **Extract helper function** | A branch does meaningful work worth naming |
| **Replace conditional with polymorphism** | The same type-check appears in multiple places |
| **Use early returns / guard clauses** | Deeply nested `if`/`else` chains |
| **Use lookup tables or dictionaries** | Many branches that map input to output |
| **Move validation to value objects** | Complex precondition checks |

### What not to do

- Do not extract functions just to get under the limit if the result is harder
  to read
- Do not hide complexity behind lambdas or comprehensions that are equally
  hard to follow
- Do not use `# noqa: C901` without a documented reason in a code comment

## File length guidance

| Metric | Soft limit | Action if exceeded |
|--------|-----------|-------------------|
| Lines per file | **200** | Review for SRP violations; consider splitting |
| Lines per function | **30** | Extract named helpers or restructure |

These are monitoring thresholds, not hard gates. A 210-line file with one
cohesive class is fine. A 150-line file with three unrelated concerns is not.

## See also

- [[architecture/solid-principles|SOLID Principles]]
- [[architecture/high-coherence|High Coherence]]
- [[architecture/linting-and-tooling|Linting and Tooling]]
- [[architecture/namespace-and-module-structure|Namespace and Module Structure]]
