# tests

ABI smoke and safety tests for `tinyquant-sys`.

## Files

| File | Purpose |
| --- | --- |
| `abi_smoke.rs` | Baseline C ABI smoke coverage |
| `abi_c_smoke.rs` | C-facing smoke tests |
| `abi_cxx_smoke.rs` | C++-facing smoke tests |
| `abi_handle_lifetime.rs` | Handle lifetime checks |
| `abi_header_compile.rs` | Header compilation checks |
| `abi_panic_crossing.rs` | Panic containment checks |
| `c/` | C fixtures or harness code |
| `cxx/` | C++ fixtures or harness code |

## See Also

- [Local AGENTS.md](./AGENTS.md)
- [Parent README](../README.md)
