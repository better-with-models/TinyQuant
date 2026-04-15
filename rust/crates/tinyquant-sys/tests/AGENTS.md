# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-sys/tests`

`tests/` verifies the stability and safety properties of the TinyQuant C ABI.

## Layout

```text
rust/crates/tinyquant-sys/tests/
├── abi_smoke.rs
├── abi_c_smoke.rs
├── abi_cxx_smoke.rs
├── abi_handle_lifetime.rs
├── abi_header_compile.rs
├── abi_panic_crossing.rs
├── c/
├── cxx/
└── README.md
```

## Common Workflows

### Update an ABI test

1. Keep the assertion focused on ABI behavior, not Rust-internal detail.
2. Update the narrowest smoke or safety test possible.
3. Re-run the crate tests after any ABI, header, or ownership change.

## Invariants — Do Not Violate

- No panic may cross the C boundary.
- Header expectations must stay aligned with `include/tinyquant.h`.
- Lifetime and free-function behavior are part of the public contract.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../../AGENTS.md)
- [rust/crates/tinyquant-sys/include/AGENTS.md](../include/AGENTS.md)
