# rust/crates/tinyquant-js

napi-rs binding crate that compiles TinyQuant to a native Node.js addon and
publishes it as the `@tinyquant/core` npm package.

## What lives here

| File / Directory | Purpose |
| --- | --- |
| `src/` | napi-rs module source — exported structs and functions |
| `build.rs` | napi-build script that generates the N-API glue |
| `Cargo.toml` | Rust crate manifest with napi-rs dependencies |
| `package.json` | npm package manifest for `@tinyquant/core` |

## How this area fits the system

`tinyquant-js` is the Phase 25 TypeScript boundary. It wraps
`tinyquant-core` (codec, codebook, compressed-vector types) and exposes them
to JavaScript consumers via N-API without any Python dependency. The compiled
`.node` binary is bundled into the npm package by the fat-wheel assembler
script in `scripts/packaging/`.

## Common edit paths

- **`src/lib.rs`** — add or modify napi-exported types when the core codec API changes.
- **`package.json`** — bump the npm version on any API change.
- **`build.rs`** — update only when napi-build or node-api-headers need reconfiguring.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
