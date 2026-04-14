# rust/crates/tinyquant-core/src/corpus

The `corpus` module is the aggregate root for named vector collections. It owns the `Corpus` struct and all domain types that govern how vectors are stored: `VectorEntry`, `CompressionPolicy`, `StorageTag`, `EntryMetaValue`, `CorpusEvent`, `ViolationKind`, and the crate-private `VectorIdMap`. It is separated from the codec layer because it handles identity, ordering, and policy — not the mathematical transform.

## What lives here

| File | Public items |
| --- | --- |
| `aggregate.rs` | `Corpus`, `BatchReport` |
| `compression_policy.rs` | `CompressionPolicy`, `StorageTag` |
| `entry_meta_value.rs` | `EntryMetaValue` |
| `vector_entry.rs` | `VectorEntry` |
| `vector_id_map.rs` | `VectorIdMap` (crate-private) |
| `events.rs` | `CorpusEvent`, `ViolationKind` |
| `errors.rs` | `CorpusError` (re-exported from `crate::errors`) |
| `mod.rs` | Module declarations and public re-exports |

## How this area fits the system

`Corpus` is the primary mutable object in a TinyQuant session. It calls into `tinyquant_core::codec` (via `Codec`) to compress raw vectors when policy requires it, and it emits `CorpusEvent` values for callers that need to react to state changes. `tinyquant-io` reads and writes corpus contents by iterating `VectorEntry` values. Backend crates receive a reference to the corpus for search.

No type in this module may depend on `tinyquant-io` or any backend crate (the dependency is one-way).

## Common edit paths

- **Corpus insertion/compression rules** — `aggregate.rs`
- **Policy configuration** — `compression_policy.rs`
- **Event variants** — `events.rs`
- **Entry metadata schema** — `entry_meta_value.rs`

## See also

- [Parent README](../README.md)
- [Local AGENTS.md](./AGENTS.md)
