---
title: Rust Port — Release and Versioning
tags:
  - design
  - rust
  - release
  - semver
  - msrv
date-created: 2026-04-10
status: draft
category: design
---

# Rust Port — Release and Versioning

> [!info] Purpose
> Define versioning, MSRV, release cadence, and the compatibility
> contract between the Rust crates, the Python `tinyquant_cpu`
> package, and downstream consumers.

## Version streams

Two independent version streams coexist in this repository:

| Stream | Artifacts | Version line |
|---|---|---|
| Python | `tinyquant-cpu` on PyPI | currently `0.1.1` |
| Rust | `tinyquant-core`, `tinyquant-io`, `tinyquant-bruteforce`, `tinyquant-pgvector`, `tinyquant-sys`, `tinyquant-cli` on crates.io; `tinyquant-rs` wheel on PyPI; pre-built `tinyquant` binary on GitHub Releases; multi-arch container on GHCR | starts at `0.1.0` |

The two streams may advance independently. Parity between them is
asserted by the cross-language test suite; deliberate drift requires
an explicit `COMPATIBILITY.md` entry.

### Tag scheme

| Purpose | Tag |
|---|---|
| Python release | `v0.1.1` (unchanged) |
| Rust crates.io + PyPI release | `rust-v0.1.0` |
| Rust pre-release | `rust-v0.1.0-alpha.1`, `rust-v0.1.0-beta.2` |

The separate prefix prevents workflow cross-contamination and makes
`git log --tags` self-documenting.

## SemVer policy

All Rust crates follow strict SemVer starting at `0.1.0`:

- **Patch** (`0.1.0 → 0.1.1`): bug fixes only. No new public items.
- **Minor** (`0.1.0 → 0.2.0`): additive public items, new features,
  new enum variants (with `#[non_exhaustive]`), relaxations of error
  conditions.
- **Major** (`0.x.y → 1.0.0`): every breaking change. Removing a
  public item, narrowing an accepted input range, changing a wire
  format, removing a feature.

### The `#[non_exhaustive]` discipline

Every public enum and struct in `tinyquant-core` and `tinyquant-io`
that might ever grow a variant or field is marked
`#[non_exhaustive]`:

```rust
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CompressionPolicy {
    Compress,
    Passthrough,
    Fp16,
}

#[non_exhaustive]
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CodecError {
    UnsupportedBitWidth { got: u8 },
    /* ... */
}
```

Downstream consumers must always use `_` arms when matching, so
adding a new variant is a minor release, not a major one.

Exceptions: error enums inside `tinyquant-sys` (the C ABI) cannot use
`#[non_exhaustive]` because the enum discriminants are stable ABI.
New variants in the C ABI are a major version bump.

## Workspace version policy

All crates in the workspace share the same version, driven by
`[workspace.package] version = "0.1.0"`. Releases are all-or-nothing.

This simplifies the mental model at the cost of sometimes publishing
a crate with no changes; cargo tolerates this.

Once `tinyquant-sys` reaches ABI stability (`1.0.0`), it may move to
an independent version line to signal ABI commitments separately
from the other crates.

## MSRV (Minimum Supported Rust Version)

**MSRV: 1.78.0** (Rust 2024 edition becomes the default in 1.85; we
stay on the 2021 edition until the MSRV reaches 1.85).

Policy:

1. MSRV bumps are a minor release (`0.n.x → 0.(n+1).0`), not a major.
2. The CI matrix includes an `msrv` job that runs
   `cargo +1.78.0 check --workspace`. Any syntax or API that breaks
   on 1.78 fails CI.
3. MSRV policy target: "stable minus two" at release time — if the
   current stable is 1.84 when we cut `0.2.0`, the new MSRV is 1.82.
4. A public `MSRV` constant is exported from `tinyquant-core/src/lib.rs`
   for programmatic introspection.

## Release cadence

No fixed schedule. Releases are cut when:

- A milestone (phase completion from the roadmap) is done.
- A parity regression is fixed.
- A consumer (better-router, nordic-mcp) requests a feature
  blockage release.

Pre-release identifiers (`alpha`, `beta`) are used freely between
milestones; a `0.1.0` release only ships after:

1. All parity tests pass on the gold corpus.
2. All CI gates pass.
3. Weekly `rust-parity.yml` has run green for at least 7 days.
4. A human approves the release PR.

## Changelog discipline

Each crate has its own `CHANGELOG.md` with the standard format:

```markdown
# Changelog

All notable changes to this crate are documented here.

The format is based on Keep a Changelog,
and this project adheres to Semantic Versioning.

## [Unreleased]

### Added
- New `Codec::compress_batch_packed` zero-alloc variant.

### Changed
- Rotation matrix cache now uses `ArcSwap` on the hot path.

### Fixed
- Parity regression on `bit_width = 2` due to off-by-one in the
  packed-index tail handler.

## [0.1.0] - 2026-XX-XX

### Added
- Initial release.
```

The root `CHANGELOG.md` is a curated rollup referencing each crate's
own changelog. `xtask release` generates both.

## Compatibility with the Python package

| Python version | Rust version | Parity |
|---|---|---|
| `0.1.1` | `0.1.0` | Full codec-layer parity, full IO parity, corpus parity pending |
| `0.2.0` | `0.2.0` | Full parity required — the version numbers track each other only at feature-milestone boundaries |

The `COMPATIBILITY.md` file at the repo root (created in phase 20)
tracks the exact supported pairs and any known drift.

## ABI stability (tinyquant-sys specifically)

Pre-`1.0.0`:

- Minor releases may add new `extern "C" fn`s and new opaque handle
  types.
- Minor releases may add new enum discriminants at the high end of
  the numeric range (never reusing an old value).
- Minor releases may not change the layout of any `#[repr(C)]`
  struct once declared.
- Header (`tinyquant.h`) changes are always paired with a
  corresponding version bump.

Post-`1.0.0`:

- The ABI is frozen per major version. Any layout change requires a
  `2.0.0`.
- A stability report runs on each release candidate comparing the
  generated header to the previous release's header; non-additive
  diffs block the release.

## Yanking policy

Yanking a release:

- Allowed for: security issues, parity regressions, ABI mistakes.
- Not allowed for: "we have a better version now."

Yanked releases stay available for consumers pinning the exact
version but are hidden from `cargo update`.

## Platform support tiers

| Tier | Meaning | Targets |
|---|---|---|
| Tier 1 | Tested in every CI run | `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc` |
| Tier 2 | Built in CI, not tested | `i686-pc-windows-msvc`, `x86_64-unknown-freebsd`, `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `thumbv7em-none-eabihf` (`no_std` only) |
| Tier 3 | Best-effort, no support contract | everything else |

Tier 1 targets get **everything**: Python wheels on PyPI, standalone
`tinyquant` binary archives on GitHub Releases, `tinyquant-sys`
`cdylib` + `staticlib` + `tinyquant.h` in the release assets, and
(where applicable) multi-arch container images on GHCR.

Tier 2 targets get standalone binary archives and `tinyquant-sys`
static libs on GitHub Releases; wheels are built only where
`manylinux`/`musllinux` compatibility is trivially met
(musl Linux targets ship `musllinux` wheels). FreeBSD and
i686-windows get binaries but no wheels.

Tier 3 targets: "file an issue with your Dockerfile."

## Release checklist

1. All CI green on `main`.
2. `rust-parity.yml` green for 7 days.
3. Bench budgets not exceeded (and ideally improved).
4. `CHANGELOG.md` updated and `[Unreleased]` converted to the new
   version with today's date.
5. Workspace version bumped via `xtask release <version>`.
6. Commit `chore(release): bump workspace to X.Y.Z`.
7. Tag `rust-vX.Y.Z`.
8. Push tag; `rust-release.yml` handles the rest.
9. Verify the new wheels install cleanly from PyPI.
10. Post release notes to the repo's discussions.

## Deprecation policy

Public items may be deprecated via `#[deprecated(since = "...", note = "...")]`.
The deprecation must:

- Specify a replacement.
- Be a minor release.
- Remain for at least one more minor release cycle before removal.
- Removal itself is a major release.

## Reproducible builds

Release artifacts are built with:

- `SOURCE_DATE_EPOCH` set to the tag's commit timestamp.
- `CARGO_BUILD_RUSTFLAGS="-C codegen-units=1 -C lto=fat"`.
- No `build.rs` timestamps or environment-dependent paths.
- `cargo-vet` enforcement on the third-party dep set to prevent
  supply chain drift.

Two builds of the same tag on different machines produce
byte-identical wheels and crates.

## See also

- [[design/rust/ci-cd|CI/CD]]
- [[design/rust/feature-flags|Feature Flags]]
- [[design/rust/ffi-and-bindings|FFI and Bindings]]
