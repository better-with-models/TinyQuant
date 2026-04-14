# rust/crates/tinyquant-fuzz/tests

Smoke tests for the `tinyquant-fuzz` crate. These run under the normal
`cargo test` invocation, not under `cargo fuzz`, and exist solely to verify
that the fuzz crate compiles correctly in the standard workspace build.

## What lives here

| File | Role |
|---|---|
| `smoke.rs` | Confirms the `tinyquant-fuzz` crate links and builds without error |

## How this area fits the system

Because fuzz builds use a separate toolchain and are not part of CI's default
`cargo test` run, `smoke.rs` catches broken imports or missing dependencies
before anyone attempts an actual fuzz session.

## Common edit paths

- `smoke.rs` — only needs changing if the crate's public interface is restructured

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
