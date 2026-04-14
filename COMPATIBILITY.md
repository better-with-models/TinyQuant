# Compatibility matrix

Authoritative ledger of supported `(tinyquant_cpu, tinyquant_rs)`
pairs. Each row maps one Rust release to the Python reference
implementation it has been parity-tested against.

The policy this ledger implements lives in
[Release and Versioning](docs/design/rust/release-strategy.md);
this file is the data side of that contract.

## Supported pairs

| `tinyquant_cpu` | `tinyquant_rs` | Known drift | Notes |
| --- | --- | --- | --- |
| 0.1.1 | 0.1.0 | R19 / R2 rotation kernel: max \|py − rs\| ≈ 3.15e-4 (below the 1e-3 parity-suite tolerance) | First Rust release. Parity covers `config_hash`, `Codebook.train` bytes, `CompressedVector.to_bytes` bytes, corpus lifecycle, batch methods, and the exception hierarchy. The rotation drift is tracked in [numerical-semantics.md](docs/design/rust/numerical-semantics.md) §R19. |

> Phase 24 pairs the forthcoming `tinyquant_cpu 0.2.0` fat wheel with the same
> `tinyquant_rs 0.1.0` per-arch wheels — no Rust-side bump is required. The
> matching row (`0.2.0 | 0.1.0`) is deliberately **not** added until the
> `py-v0.2.0` tag push produces a real PyPI artifact via
> [`python-fatwheel.yml`](.github/workflows/python-fatwheel.yml). The R19
> rotation-drift entry carries forward unchanged: the Rust core is
> byte-identical between the two pairs.

## Update cadence

Every release of either package adds a new row. Drift entries
must link to:

- the corresponding GitHub issue or risk ID
  ([risks-and-mitigations.md](docs/design/rust/risks-and-mitigations.md)),
- the parity-suite tolerance against which the drift is measured,
- a mitigation plan (or an explicit "accepted" rationale).

## Validation

`cargo xtask compatibility-check` (Phase 22.D follow-up) reads this
table and asserts that the topmost `tinyquant_rs` row matches the
current workspace version. CI runs it on every PR that touches
`rust/**/Cargo.toml`. Until that subcommand lands, reviewers
manually verify the table during release prep.

## Ownership

Phase 22 created this file. Every subsequent phase that ships a
release of either package (Python or Rust) maintains it as part of
that phase's release checklist.
