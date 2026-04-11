---
title: "Phase 14: Codebook and Quantize Kernels — Implementation Notes"
tags:
  - design
  - rust
  - phase-14
  - codebook
  - quantize
  - implementation-notes
  - retrospective
date-created: 2026-04-10
date-completed: 2026-04-10
status: stable
category: design
---

# Phase 14 — Implementation Notes

> [!info] Purpose
> Captures the design decisions and deviations from the original
> [[plans/rust/phase-14-codebook-quantize|Phase 14 Plan]] that arose
> during execution on 2026-04-10. Future Rust phases should read this
> alongside [[design/rust/numerical-semantics|Numerical Semantics]]
> §Quantization and §Codebook training for the parity invariants that
> are now locked-in by code and fixtures rather than by prose alone.

> [!note] Relationship to the plan doc
> The plan doc describes the *intended* TDD sequence. This page
> records the *actual* outcome: what landed, what was reshaped
> mid-execution, and why. The plan doc itself has been flipped to
> `status: complete` — treat it as the intent-of-record and this page
> as the execution log.

## What landed

Phase 14 shipped on the dedicated `phase-14-codebook-quantize` branch
and merges back to `main` via an explicit merge commit (per the
user's branch-workflow decision captured in
`C:\Users\aaqui\.claude\plans\tidy-singing-whistle.md`). The branch
contents split into logically-reviewable commits:

1. `chore(workspace)` — add `bytemuck` as a `tinyquant-core`
   dev-dependency via the workspace table, extend `.gitattributes`
   with LFS globs for the two new fixture dirs.
2. `feat(tinyquant-core)` — `Codebook` value object with
   construction invariants, `train`, `quantize_into`, `dequantize_into`,
   and the `PartialEq`-by-bits / custom `Debug` impls.
3. `feat(tinyquant-core)` — private `scalar_quantize` /
   `scalar_dequantize` kernels in `codec::quantize` that the public
   `Codebook` methods delegate to.
4. `feat(xtask)` — `fixtures refresh-codebook` +
   `fixtures refresh-quantize` + extended `refresh-all`.
5. `feat(scripts)` — `generate_rust_fixtures.py codebook` and
   `quantize` subcommands.
6. `test(tinyquant-core)` — freeze the 10 000 × 64 training corpus
   plus the `bw={2, 4, 8}` codebook + quantize fixtures in Git LFS.
7. `docs(rust)` — mark Phase 14 complete; append `log.md`; land this
   implementation-notes page.

New public surface in `tinyquant-core`:

- `tinyquant_core::codec::Codebook` — value object with `new`,
  `train`, `num_entries`, `bit_width`, `entries`, `quantize`,
  `quantize_into`, `dequantize`, `dequantize_into`. Inner storage is
  `Arc<[f32]>` so `Clone` is `O(1)`; equality compares the f32 bits.
- `tinyquant_core::codec::quantize::{scalar_quantize,
  scalar_dequantize}` — `pub(crate)` inside a `pub(crate)` module
  (clippy `redundant_pub_crate` rewrote these to `pub`). These are
  the panic-free reference kernels that the future SIMD path will
  diff against.

Prelude re-exports `Codebook` alongside the Phase 13 surface.

## Deviations from the original plan

### Bit-width sweep instead of a single fixture

The draft plan captured a single training + quantize fixture at
`(seed=42, bit_width=4, n=10000, d=64)`. On review the user requested
the full `bw ∈ {2, 4, 8}` sweep. The training corpus is shared across
all three, so the fixture footprint only grew by 16 + 64 + 1024 = 1104
bytes of codebook entries plus 3 × 10 000 bytes of expected indices —
negligible compared to the 2.5 MiB training corpus. Worth it: we now
have byte parity on 4 + 16 + 256 codebook entries and 30 000 quantize
indices rather than just 16 + 10 000.

### No `proptest`, yes `rand_chacha` loop

The original plan called for a `proptest` proptest to show that
`quantize_into` always produces indices in `[0, 2^bit_width)`. Adding
`proptest = "1"` to the workspace dev-deps pulled a transitive
dependency (`getrandom 0.4.2`, ultimately via modern `tempfile` and
`rustix`) that requires Cargo's `edition2024` feature, which is only
stable from Rust 1.85 onward. The workspace MSRV is 1.81, pinned by
`rust-toolchain.toml` and the `rust-version` field in
`rust/Cargo.toml`, and Phase 12 already bumped from 1.78 once — we
did not want to move it again for a testing convenience.

Instead, the property is expressed as a deterministic ChaCha-seeded
loop (256 batches × up to 512 finite `f32` values drawn from
`ChaCha20Rng::seed_from_u64(1337)`). `rand_chacha` is already a
`tinyquant-core` runtime dependency, so this adds nothing to the
dependency graph. The test covers the same invariant as the original
proptest would have and is trivially reproducible.

### Runtime `fs::read` fixture loading, not `include_bytes!`

The draft plan showed the training-parity test using
`include_bytes!`. That works for small constants, but the Phase 14
training corpus is 2.5 MiB and would have bloated every integration
test binary that linked to `tinyquant-core`. Instead, both new
integration test files follow the exact pattern already in use for
`rotation_fixture_parity.rs`: compute the path with
`env!("CARGO_MANIFEST_DIR")` and read at runtime via `std::fs::read`.
Fixture data therefore only materializes when a test asking for it
runs.

### Tie-break confirmation test had the wrong expected literals

While writing the `quantize_then_dequantize_returns_nearest_entries`
round-trip test I pre-computed expected entries using
`(0..16).map(|i| i as f32 * 0.1)`, which is *not* the same as
`{0.0, 0.1, 0.2, …, 1.5}` — `9.0_f32 * 0.1_f32` is
`0.90000004`, not `0.9`. The first run surfaced the mismatch cleanly
("`[…, 0.90000004, …]` vs `[…, 0.9, …]`"), so the test now compares
`out[k]` to `entries[expected_idx]` directly rather than to a
hand-typed literal. No change to the production code was needed —
this was a test-author gotcha, noted here so future tests in the
codec module use the same pattern.

### Lint collisions and their fixes

The crate denies `clippy::pedantic + nursery + unwrap_used +
expect_used + panic + indexing_slicing + cognitive_complexity`, which
caught several first-draft mistakes before they landed:

- `bool_to_int_using_if` — rewrote
  `if entries.is_empty() { 0 } else { 1 }` as
  `u32::from(!entries.is_empty())`.
- `explicit_iter_method` — replaced `for &v in entries.iter()` with
  `for &v in &*entries` (deref coercion on `Box<[f32]>`).
- `cast_precision_loss` + `cast_possible_truncation` +
  `cast_sign_loss` — the quantile arithmetic legitimately needs
  `k as f64`, `last_idx as f64`, `h.floor() as usize`, and
  `value_f64 as f32`. Rather than fighting the casts, `Codebook::train`
  carries a local `#[allow(...)]` attribute naming exactly those three
  lints. No other function needs the allow.
- `missing_fields_in_debug` — the hand-written `Debug` impl now
  includes the `entries` slice, not just `bit_width` and
  `num_entries`.
- `redundant_pub_crate` — because `codec::quantize` is already
  `pub(crate)`, inner items must be `pub` rather than `pub(crate)`.
- `trivially_copy_pass_by_ref` — the `f32_cmp` helper takes
  `f32, f32` by value.

All of these are documented here so the next phase doesn't reinvent
the workaround.

### No changes to the Python reference

Phase 14 deliberately does not touch `src/tinyquant_cpu/codec/`. The
Python codebook is the parity source of truth; the Rust port mirrors
it. If Python-side drift ever surfaces through the fixture diff, the
right move is to investigate the Python change first, not to relax
the Rust comparison.

## Test evidence

After the final green run:

- **12 codebook tests** — four construction invariants, bw=2/4/8
  training parity, two quantize/dequantize round-trips, bw=2+bw=8
  bounds smoke, dequantize out-of-range rejection, and the
  ChaCha-seeded in-range scan.
- **4 quantize tests** — bw=2, bw=4, bw=8 Python-fixture byte parity
  (10 000 indices each, 30 000 total) and a bw=4 dequantize round
  trip that every output f32 belongs to the codebook entry set.
- **No regressions** — all Phase 12 and Phase 13 tests continue to
  pass (`codec_config` 13, `rotation_matrix` 8,
  `rotation_fixture_parity` 3, `rotation_cache` 8, plus the pre-
  existing workspace baseline).

`cargo xtask fmt`, `cargo xtask lint`, `cargo xtask test`,
`cargo build -p tinyquant-core --no-default-features`, and
`cargo build -p tinyquant-core --target thumbv7em-none-eabihf
--no-default-features` were all green on the final sweep. Running
`cargo xtask fixtures refresh-all` after the code landed left
`git status` empty against the committed fixture blobs, proving the
Python + Rust fixture round trip is deterministic.

## What Phase 15 inherits

Phase 15 (Codec service + residual) can assume:

- `Codebook::train` produces byte-identical entries for the same
  `(seed, rows, cols, bit_width)` triple Python does — the test
  suite freezes the `seed=42, rows=10000, cols=64, bw ∈ {2, 4, 8}`
  snapshot.
- `Codebook::quantize_into` is byte-for-byte compatible with
  `tinyquant_cpu.codec.codebook.Codebook.quantize` on f32 inputs
  drawn from `numpy.random.default_rng(7).standard_normal`.
- `scalar_quantize` and `scalar_dequantize` are stable reference
  kernels: Phase 20's SIMD path can diff against them crate-
  internally without going through `Codebook`.
- `Codebook` is `no_std`-safe, `Clone + Debug + PartialEq`, and
  panic-free under the codec crate's full lint profile.

## See also

- [[plans/rust/phase-14-codebook-quantize|Phase 14 Plan]]
- [[design/rust/numerical-semantics|Numerical Semantics]]
- [[design/rust/phase-13-implementation-notes|Phase 13 Implementation
  Notes]]
- [[plans/rust/phase-15-codec-service-residual|Phase 15 Plan]]
