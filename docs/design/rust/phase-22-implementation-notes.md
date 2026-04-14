---
title: "Phase 22 Implementation Notes"
tags:
  - design
  - rust
  - phase-22
  - pyo3
  - cabi
  - release
date-created: 2026-04-14
category: design
---

# Phase 22 Implementation Notes

## Summary

Phase 22 splits into four parts (A/B/C/D). At the time of writing:

- **Part A — `tinyquant-py` PyO3 wheel (LANDED).** Commits `978ebf1`
  (initial wheel) and `158a132` (code-review fixes) ship the
  `tinyquant_cpu` parity surface: `CodecConfig`, `Codebook`,
  `CompressedVector`, `Codec.compress` / `.decompress` /
  `.compress_batch`, the frozen exception hierarchy
  (`TinyQuantError` → `DimensionMismatchError`, `ConfigMismatchError`,
  `CodebookIncompatibleError`, `DuplicateVectorError`), Python
  `__reduce__` round-trip, byte-equality parity tests against the
  Python reference, and GIL-release paths on every expensive call.
  cibuildwheel targets are wired into CI.
- **Part B — `tinyquant-sys` C ABI (LANDED at `f1eae7c`, fixed on
  this branch).** Opaque handle types (`CodecConfigHandle`,
  `CodebookHandle`, `CompressedVectorHandle`, `CorpusHandle`,
  `ByteBufferHandle`), `TinyQuantErrorKind` / `TinyQuantError`
  out-pointer, `catch_unwind`-wrapped `extern "C" fn` entry points for
  single-vector compress / decompress, compressed-vector
  (de)serialisation, and corpus CRUD. `build.rs` regenerates
  `include/tinyquant.h` via cbindgen on every crate build; CI gates
  the committed header with `git diff --exit-code`. Test surface:
  `abi_smoke`, `abi_handle_lifetime`, `abi_header_compile`,
  `abi_c_smoke`, `abi_cxx_smoke`, `abi_panic_crossing` (behind
  `panic-probe` feature).
- **Part C — `tinyquant-cli` standalone binary (PENDING).**
- **Part D — release workflow + tag-driven publish (PENDING).**

This note documents Part B + the follow-up fixes from the spec
reviewer's ISSUES FOUND round. Part A is already covered by its own
commit messages and the Phase 22 plan doc.

## Deviations from spec

### 1. Error-kind discriminants `Panic = 98` and `Unknown = 99`

`docs/plans/rust/phase-22-pyo3-cabi-release.md` and the companion
design note `docs/design/rust/ffi-and-bindings.md` §Binding 2 gave
sentinel discriminant suggestions in the 250-range (e.g. `Panic =
254`, `Unknown = 255`). The implementation ships `Panic = 98` and
`Unknown = 99` instead.

Rationale, preserved from the Phase 22.B implementer's notes:

- **Readability in C consumer code.** `TQ_ERR_PANIC == 98` is easier
  to spot in log output and gdb watchpoints than `254`. Two-digit
  values do not look like "bit flags gone wrong".
- **Small-integer discipline.** The `TinyQuantErrorKind` enum is
  `#[repr(C)]` backed by the platform's default C-enum width. A
  8-bit `uint8_t` is not guaranteed, so using values near `u8::MAX`
  buys nothing portability-wise, but it does look like a sentinel
  that callers may want to bit-test — which is not the API contract.
- **No collision risk.** The 0.x line adds variants at the low end
  of the range (next slot is `Io = 6` → `InvalidArgument = 7` →
  future additions go into `8`, `9`, `10`, …). Leaving a gap from
  `7` to `98` reserves ~90 slots for additional well-classified
  errors before ever approaching the `Panic` / `Unknown`
  "wrapper-internal" discriminants.

The header-level contract is frozen for the 0.x line either way —
changing a discriminant is a breaking change, so the specific numbers
are not a blocker.

### 2. §Binding 2 batch + threadpool functions deferred to Phase 22.C

§Binding 2 of `docs/design/rust/ffi-and-bindings.md` lists three
functions that are NOT present in the landed 22.B crate:

- `tq_set_num_threads(n: u32) -> TinyQuantErrorKind` — configure the
  Rayon pool used by batch paths.
- `tq_codec_compress_batch(... *mut *mut CompressedVectorHandle, ...)`
  — parallel batch compress.
- `tq_codec_decompress_batch(... *const *const CompressedVectorHandle,
   f32 *out, ...)` — parallel batch decompress.

**Decision: defer to Phase 22.C follow-up commit on this same
branch.** Rationale:

- `tq_codec_compress_batch` must return an *array* of
  `CompressedVectorHandle*` to the caller. That introduces a new
  ownership rule ("caller owns every element; use `tq_bytes_free`
  plus `tq_compressed_vector_free` in a loop to tear down") which
  warrants a dedicated `abi_handle_lifetime` test-suite expansion
  and an `AGENTS.md`-style ownership rule in `tinyquant.h`. That is
  more than the "small rayon wrap" the blocking-issues note
  suggested.
- `tq_set_num_threads` needs to decide between `rayon::ThreadPool`
  (per-library pool installed via `pool.install(||
  compress_batch_with(...))`) and the Rayon global pool. Phase 21
  adopted per-call `pool.install`, and Part A (`tinyquant-py`)
  inherits the global-pool default. The C ABI should match *one* of
  those; picking correctly is a design call that belongs with the
  CLI crate (Part C), which also needs the same thread-pool story.
- Parts A and B are a release-ready subset without the batch C
  functions: every downstream consumer that currently exists (Python
  wheel, the CLI that Part C will add) goes through the native Rust
  crate, not the C ABI. The C ABI is for *external* consumers and
  they are content to loop over single-vector `tq_codec_compress`
  until 22.C lands the batch entry points.

**Concrete plan for Phase 22.C:**

- Add `tq_set_num_threads` that builds a `rayon::ThreadPoolBuilder`
  and stores the pool in a `OnceLock<Arc<ThreadPool>>` inside
  `tinyquant-sys::internal`.
- Add `tq_codec_compress_batch` + `tq_codec_decompress_batch` that
  install the stored pool before calling `codec.compress_batch_with`
  / `codec.decompress_batch_into` with `Parallelism::Custom(rayon_driver)`.
- Expand `abi_handle_lifetime.rs` with double-array ownership tests
  and a null-element test for the output buffer.
- Regenerate `include/tinyquant.h` via `cargo build -p tinyquant-sys
  --release` (see §Header generation workflow below) in the same
  commit.

### 3. `abi_handle_lifetime.rs` does not cover double-free / UAF

`rust/crates/tinyquant-sys/tests/abi_handle_lifetime.rs:16-18`
documents the opt-out:

```text
We do NOT test double-free: the contract is "must be called exactly
once per successful *_new", and verifying it would require
instrumenting the heap.
```

**Why:** a double-`Box::from_raw` on the same pointer is undefined
behaviour in safe Rust — there is no deterministic way to assert
"this crashed safely" without either:

- running under **Miri** (which we already do for the scalar kernel
  parity tests in Phase 20, but Miri does not currently model
  `catch_unwind` payloads reliably enough to round-trip a panic
  message), or
- wiring in a **debug-build sentinel** pattern (stamp a magic value
  on `Box::into_raw`, zero it on `Box::from_raw`, check for the
  magic on every dereference) — this is the standard C pattern for
  catching UAF, but it requires every accessor (`tq_*_bit_width`,
  `tq_*_dimension`, …) to branch on the sentinel in **release
  builds too**, or the check is useless. That costs one predictable
  branch per accessor call on the hot path and reshapes the ABI
  (we cannot then return a sentinel on null without ambiguity).

**Decision:** keep the opt-out, document it in the header prose
("must be called exactly once per successful `_new`"), and rely on
the single-pass lifetime tests we do have. A future Phase (22.C or
later) can add a **debug-feature-gated** magic-sentinel path —
e.g. `#[cfg(feature = "debug-handles")]` — without touching the
release ABI. That is a strict addition, not a deviation.

### 4. `abi_panic_crossing.rs` covers only `tq_test_panic`

The current panic-crossing integration test exercises the
`catch_unwind` wrapper via a dedicated probe (`tq_test_panic`),
gated on `feature = "panic-probe"`. It does NOT deliberately force
a panic through every `extern "C" fn` entry point and assert the
result is `TinyQuantErrorKind::Panic`.

**Why:** mechanically forcing a panic through `tq_codec_compress`
or `tq_corpus_insert` would require either:

- injecting a panic hook into `tinyquant-core` internals (invasive,
  couples the core crate to the sys crate's test shape), or
- patching out the codec with a mock that panics on call (needs
  dynamic dispatch in `tinyquant-core::codec::Codec`, which the
  phase-design doc explicitly rejects on performance grounds —
  `Codec::new()` is a zero-size struct).

**Decision:** document as a known gap. The invariant we care about
("`catch_unwind(AssertUnwindSafe(...))` wraps every entry point,
converts panic payloads to `TinyQuantErrorKind::Panic`, and returns
before unwinding into C") is:

- *structurally* enforced by the shape of every `extern "C" fn` in
  `codec_abi.rs` and `corpus_abi.rs` (they all follow the same
  3-line boilerplate and a clippy lint could later grep for the
  pattern), and
- *dynamically* enforced by `tq_test_panic`, which shares the exact
  same `catch_unwind` wrapper.

If a future refactor splits or renames the wrapper, the
`tq_test_panic` probe will catch it at test time, and a follow-up
inspection of the other entry points is cheap. Expanding the test
matrix to every entry point is ~24 additional tests for the same
binary-level guarantee.

### 5. cbindgen `with_src` instead of `with_crate`

`build.rs` feeds cbindgen only `src/lib.rs` via `with_src`, not the
full crate via `with_crate`. Reason carried over verbatim from the
22.B commit message:

> `cbindgen`'s `with_crate` runs `cargo metadata`, which in this
> workspace fails because the pgvector crate transitively references
> a registry package that requires a newer Cargo than our MSRV 1.81.
> `with_src` side-steps metadata by enumerating the Rust sources
> directly — cbindgen parses each file, follows `use crate::...` at
> syntactic level, and never invokes cargo.

**Side-effect addressed in this fix round:** cbindgen's `@version@`
substitution in the `header` config entry is ONLY applied by the
`with_crate` code-path (cargo metadata supplies the version). Under
`with_src`, the `@version@` token passes through literally. The
fixed build script now post-processes the emitted header and
substitutes `@version@` with `env!("CARGO_PKG_VERSION")` at build
time; a `const _: ()` compile-time assertion in `src/lib.rs` catches
drift between the manually-kept-in-sync `TINYQUANT_H_VERSION`
constant and the build-time crate version. See §Header generation
workflow for the command.

### 6. cbindgen does NOT fail the build on generation error

`build.rs` catches a cbindgen `Err` and emits
`cargo:warning=cbindgen failed ...` rather than panicking. That is
deliberate and carried over from the 22.B commit: `cargo publish`
repacks the crate without a full source tree in some environments,
and a hard-failing build script would break crates.io publication.
The CI `abi-header-drift` job still catches any real regression via
`git diff --exit-code rust/crates/tinyquant-sys/include/tinyquant.h`.

## Header generation workflow

The generated header `rust/crates/tinyquant-sys/include/tinyquant.h`
is a **committed source file**, not a build artefact. Regenerate it
with:

```bash
cargo build -p tinyquant-sys --release
```

That invokes `build.rs`, which:

1. Loads `cbindgen.toml`.
2. Runs cbindgen with `with_src(src/lib.rs)` to avoid cargo
   metadata (MSRV 1.81 compatibility).
3. Writes the emitted header to `include/tinyquant.h`.
4. Reads it back and substitutes every `@version@` token with the
   build-time value of `CARGO_PKG_VERSION` (the token in the
   `header = ...` config block that cbindgen does not expand under
   `with_src`).
5. Emits `cargo:rerun-if-env-changed=CARGO_PKG_VERSION` so a crate
   version bump triggers regeneration even when no source under
   `src/` was touched.

### Drift guards

- **Rust compile-time:** `src/lib.rs` declares `pub const
  TINYQUANT_H_VERSION: &str = "0.1.0"` and an adjacent `const _: ()
  = { ... }` block that byte-compares it against
  `env!("CARGO_PKG_VERSION")`. A bump of the workspace
  `[workspace.package] version` without also updating this constant
  is a compile-time error with a clear message.
- **Integration test:** `tests/abi_header_compile.rs` contains a
  dedicated `header_version_macro_matches_crate_version` test that
  reads the committed header file and asserts both the substituted
  value and the absence of any literal `@version@`. This test does
  NOT depend on `libclang`, so it runs on every CI host even when
  the bindgen round-trip inside the same file skips.
- **CI:** `rust-ci/abi-header-drift` runs
  `cargo build -p tinyquant-sys --release` followed by
  `git diff --exit-code rust/crates/tinyquant-sys/include/tinyquant.h`.
  A non-zero diff means the committed header is stale vs. Rust source.
- **`abi_header_compile.rs` early-return on missing libclang.** The
  bindgen round-trip check inside that file is wrapped in
  `std::panic::catch_unwind` and returns a skip message if libclang
  is not discoverable (`libclang` is an indirect dep of `bindgen`).
  That early-return is **load-bearing for minimal CI runners** and
  must not be removed — the version-macro check (above) runs
  unconditionally so we do not lose coverage on libclang-less hosts.

## Open items for Phase 22.C / 22.D

Tracked as work for the remaining Phase 22 parts. None of these
block Parts A + B, but 22.C must clear the first three before
Part D publishes the first tagged release.

- [ ] **22.C.1:** Implement `tq_set_num_threads`,
      `tq_codec_compress_batch`, `tq_codec_decompress_batch` per the
      plan in §Deviation 2 above. Regenerate `include/tinyquant.h`
      in the same commit.
- [ ] **22.C.2:** Expand `abi_handle_lifetime.rs` with array-of-
      `CompressedVectorHandle*` ownership tests (null elements,
      partial-failure teardown, double-free of batch outputs).
- [ ] **22.C.3:** `tinyquant-cli` standalone binary per
      `docs/plans/rust/phase-22-pyo3-cabi-release.md` Part C.
- [ ] **22.D.1:** Release workflow + tag-driven publish (PyPI +
      crates.io + GitHub Release artefacts) per Part D.
- [ ] **22.D.2:** Consider a `#[cfg(feature = "debug-handles")]`
      sentinel pattern for catching double-free / UAF in debug
      builds (§Deviation 3). Strict addition; does not touch the
      release ABI.
- [ ] **22.D.3:** Evaluate whether to expand `abi_panic_crossing`
      to loop over every `extern "C" fn` (§Deviation 4). Decision
      gate: cost (~24 tests, 1-second runtime) vs. benefit (catch
      a refactor that removes `catch_unwind` from a single entry
      point). Defer unless such a refactor is actually proposed.

## Commit trail

| Commit | Summary |
|---|---|
| `978ebf1` | `feat(phase-22.A/py): pyo3 wheel with tinyquant_cpu parity surface` |
| `158a132` | `refactor(phase-22.A/py): address code-review feedback on parity suite and features` |
| `f1eae7c` | `feat(phase-22.B/sys): c abi handles, cbindgen header, and test surface` |
| *this PR* | `fix(phase-22.B/sys): substitute CARGO_PKG_VERSION in generated header + const_assert` + `docs(rust/phase-22): implementation notes covering Parts A+B` |
