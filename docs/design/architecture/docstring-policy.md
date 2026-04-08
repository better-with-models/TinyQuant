---
title: Docstring Policy
tags:
  - design
  - architecture
  - documentation
  - docstrings
date-created: 2026-04-08
status: active
category: design
---

# Docstring Policy

> [!info] Policy
> Every public symbol has a rich PEP 257 docstring using Google-style
> sections. Docstrings are enforced by `ruff` rule `D` and verified by
> Sphinx extraction.

## What must be documented

| Symbol type | Required | Format |
|-------------|----------|--------|
| Public module | Yes | One-line summary + purpose description |
| Public class | Yes | Summary + class-level docstring describing responsibility and invariants |
| Public method / function | Yes | Summary + `Args`, `Returns`, `Raises` sections |
| Public property | Yes | One-line summary of what it returns |
| Protocol class | Yes | Summary + contract description that implementations must honor |
| Private helper (`_name`) | Recommended if non-obvious | One-line summary minimum |
| Test function | Yes | One-line summary of the behavior being verified |

## Google-style section format

```python
def compress(
    vector: NDArray[np.float32],
    config: CodecConfig,
    codebook: Codebook,
) -> CompressedVector:
    """Compress an FP32 vector into a low-bit representation.

    Applies random preconditioning via the rotation matrix derived from
    the config's seed, then stage-1 scalar quantization against the
    codebook, and optionally stage-2 residual correction.

    Args:
        vector: Input embedding vector. Must match ``config.dimension``.
        config: Immutable codec configuration controlling bit width,
            seed, dimension, and residual mode.
        codebook: Trained codebook with ``2^bit_width`` entries.

    Returns:
        Compressed vector with quantized indices and optional residual
        data. Carries a ``config_hash`` linking it to the originating
        config.

    Raises:
        DimensionMismatchError: If ``len(vector) != config.dimension``.
        CodebookIncompatibleError: If the codebook was trained under a
            different config.
    """
```

## Docstring quality rules

1. **First line is a truthful imperative summary.** "Compress an FP32
   vector..." not "This method compresses..."
2. **Args match the signature exactly.** If the parameter is renamed, the
   docstring must update in the same commit.
3. **Returns describes the value, not the type.** The type annotation already
   shows the type.
4. **Raises lists only exceptions callers should handle.** Do not document
   `TypeError` from argument validation — that is a programming error, not a
   contract.
5. **No invented guarantees.** Do not claim thread safety, performance bounds,
   or error behavior the code does not actually implement.
6. **Examples when non-obvious.** Value objects and protocol classes benefit
   from a short usage example.

## Anti-patterns

| Anti-pattern | Fix |
|-------------|-----|
| Restating the signature in prose | Delete the redundant prose; let the signature speak |
| Documenting private internals that callers cannot observe | Remove or mark as implementation note |
| Stale examples that no longer match the API | Update or delete; stale examples are worse than none |
| Mixing Google and NumPy section styles in one file | Standardize on Google style |
| Empty docstrings (`"""."""`) to silence the linter | Write a real summary or mark the symbol private |

## Extraction and verification

```bash
# Verify docstrings with ruff
ruff check --select D .

# Generate API docs with Sphinx
sphinx-apidoc -o docs/api src/tinyquant/

# Quick runtime check
python -m pydoc tinyquant.codec.codec
```

## See also

- [[architecture/linting-and-tooling|Linting and Tooling]]
- [[architecture/type-safety|Type Safety]]
- [[domain-layer/ubiquitous-language|Ubiquitous Language]]
