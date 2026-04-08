---
title: Test-Driven Development
tags:
  - design
  - architecture
  - tdd
  - testing
date-created: 2026-04-08
status: active
category: design
---

# Test-Driven Development

> [!info] Policy
> All TinyQuant production code enters through a failing test. No
> implementation work proceeds without a red test that names the next
> capability.

## The loop

1. **Red** — write the smallest failing test that describes one observable
   behavior
2. **Green** — make that test pass with the simplest implementation that
   preserves intent
3. **Refactor** — improve structure, names, and duplication while the suite
   stays green

Each cycle should take minutes, not hours.

## Test levels

| Level | Use for | Speed |
|-------|---------|-------|
| **Unit** | Pure domain logic: codec math, value object invariants, policy rules | Milliseconds |
| **Component** | Codec + Codebook + RotationMatrix composed together; Corpus aggregate behavior | Fast (no I/O) |
| **Integration** | Serialization round trips, persistence helpers, backend adapter wiring | Seconds |
| **Calibration** | Score fidelity, Pearson rho, rank preservation against gold corpus | Minutes (CI gate, not inner loop) |

> [!tip] Default to unit
> If you are unsure where a test belongs, start at the unit level. Promote to
> component or integration only when the behavior genuinely spans multiple
> collaborators.

## Test structure

```python
# tests/codec/test_compress.py

def test_compress_produces_indices_in_valid_range(
    codec_config_4bit: CodecConfig,
    trained_codebook: Codebook,
    sample_vector: NDArray[np.float32],
) -> None:
    """Stage-1 indices must fall within [0, 2^bit_width)."""
    result = compress(sample_vector, codec_config_4bit, trained_codebook)
    assert all(0 <= idx < 16 for idx in result.indices)
```

- Test names describe the behavior, not the method under test
- One assertion focus per test (multiple `assert` lines are fine if they check
  one logical outcome)
- Fixtures supply domain objects, not raw dicts or magic numbers
- No mocks for domain logic; mocks only at true I/O boundaries

## Design signals TDD exposes

| Signal | Likely cause |
|--------|-------------|
| Too many constructor arguments | Missing value object or config aggregate |
| Hard-to-create test data | Model coupled to persistence or framework shape |
| Excessive mocking | Interaction-heavy design; consider restructuring around return values |
| Brittle assertions | Test asserts implementation sequence, not outcome |

## What TDD means for TinyQuant specifically

- Codec primitives (rotate, quantize, residual correct) get unit tests first
- Codebook training gets component tests against small synthetic data
- Corpus invariants (config freezing, policy enforcement) get unit tests on
  the aggregate
- Score fidelity gets calibration tests that run in CI but are not part of the
  inner red-green-refactor loop
- Backend adapters get integration tests behind the search protocol interface

## Anti-patterns to avoid

- **Test-last drift:** writing large implementation blocks before creating a
  failing test
- **Assertion-free tests:** relying on "no exception" as the only contract
- **Mock-driven design:** creating interfaces solely to satisfy a mocking
  framework
- **Overly broad first tests:** starting at system scope before the design
  vocabulary exists
- **Refactor debt:** adding behavior after behavior without pausing to clean up

## Tooling

| Tool | Role |
|------|------|
| `pytest` | Test runner with `--import-mode=importlib` |
| `pytest-cov` | Coverage reporting; target 90%+ on codec and corpus layers |
| `hypothesis` | Property-based testing for codec determinism and round-trip invariants |

## See also

- [[architecture/solid-principles|SOLID Principles]]
- [[architecture/linting-and-tooling|Linting and Tooling]]
- [[behavior-layer/README|Behavior Layer]]
- [[domain-layer/aggregates-and-entities|Aggregates and Entities]]
