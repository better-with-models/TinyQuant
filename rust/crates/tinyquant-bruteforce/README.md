# rust/crates/tinyquant-bruteforce

`tinyquant-bruteforce` implements the `SearchBackend` trait by computing cosine
similarity against every stored vector on each query — a linear scan. It is
kept as a standalone crate so the in-process reference implementation can be
swapped out for production backends (e.g. `tinyquant-pgvector`) without
touching shared core types. The crate is suitable for corpora up to roughly
100 k vectors and is the primary target for SIMD kernel development.

## What lives here

List the important file groups, entrypoints, or submodules in this directory.

## How this area fits the system

Explain who calls into this directory, what it depends on, and which local invariants matter.

## Common edit paths

Note the files or subdirectories most likely to change for routine work.

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
