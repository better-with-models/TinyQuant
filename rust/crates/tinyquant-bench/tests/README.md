# rust/crates/tinyquant-bench/tests

Calibration quality-gate tests for `tinyquant-bench`. These run under the
normal `cargo test` invocation (not `cargo bench`) and enforce minimum
acceptable recall and correlation values.

## What lives here

| File | Role |
|---|---|
| `calibration.rs` | Asserts neighbor-recall and Pearson ρ exceed defined thresholds |
| `smoke.rs` | Confirms the test harness links and the calibration helpers are reachable |

## How this area fits the system

CI runs these tests on every push. A regression in `BruteForceBackend`
accuracy or a bad change to a calibration formula will cause `calibration.rs`
to fail before the change reaches a benchmark run. The tests use the
`tinyquant-bench` library crate directly and require no external services.

## Common edit paths

- Adjusting pass/fail thresholds: `calibration.rs`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
