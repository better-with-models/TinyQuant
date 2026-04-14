# rust/crates/tinyquant-bench/src

Library source for `tinyquant-bench`. The crate exposes a single public module
(`calibration`) used by both the `benches/` Criterion targets and the `tests/`
quality-gate suite.

## What lives here

| Path | Role |
| --- | --- |
| `lib.rs` | Crate root; declares `pub mod calibration` |
| `calibration/mod.rs` | Module entry; re-exports calibration helpers |
| `calibration/neighbor_recall.rs` | Top-k neighbor recall metric |
| `calibration/pearson.rs` | Pearson ρ correlation metric |

## How this area fits the system

Criterion bench files in `../benches/` and integration tests in `../tests/`
both depend on this library crate. No backend logic lives here; the module
purely provides measurement utilities that are independent of any specific
`SearchBackend` implementation.

## Common edit paths

- Adjusting the recall formula: `calibration/neighbor_recall.rs`
- Adjusting the Pearson ρ formula: `calibration/pearson.rs`
- Adding a new calibration metric: new file in `calibration/`, declared in `calibration/mod.rs`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
