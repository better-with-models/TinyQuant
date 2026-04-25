# AGENTS.md — Guide for AI Agents Working in `rust/crates/tinyquant-js/src`

`src/` implements the Rust `napi-rs` layer behind the npm package.

## Layout

```text
rust/crates/tinyquant-js/src/
├── lib.rs
├── backend.rs
├── buffers.rs
├── codec.rs
├── corpus.rs
├── errors.rs
└── README.md
```

## Common Workflows

### Update a JavaScript binding

1. Keep the Rust-side export aligned with the TypeScript wrapper surface.
2. Add or adjust `#[napi]` items in the narrowest module possible.
3. Re-run npm package tests after changing binding shape or errors.

## Invariants — Do Not Violate

- `lib.rs` owns module registration.
- This crate exposes bindings; it does not redefine core codec behavior.
- Error mapping must stay aligned with the JavaScript wrapper layer.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../../AGENTS.md)
- [javascript/@tinyquant/core/AGENTS.md](../../../../javascript/@tinyquant/core/AGENTS.md)
