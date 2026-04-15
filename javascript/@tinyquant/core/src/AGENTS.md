# AGENTS.md — Guide for AI Agents Working in `javascript/@tinyquant/core/src`

`src/` holds the TypeScript wrapper layer for the native npm package.

## Layout

```text
javascript/@tinyquant/core/src/
├── index.ts
├── _loader.ts
├── _errors.ts
├── backend.ts
├── codec.ts
├── corpus.ts
└── README.md
```

## Common Workflows

### Update a wrapper

1. Confirm the matching Rust `#[napi]` export already exists.
2. Keep wrappers thin and ergonomic; do not reimplement codec math here.
3. Re-run the package tests after changing exports or loader behavior.

## Invariants — Do Not Violate

- `index.ts` is the public barrel and must stay aligned with package exports.
- `_loader.ts` owns native-binary selection.
- Wrapper modules delegate behavior to the native binding.

## See Also

- [Parent AGENTS.md](../AGENTS.md)
- [Root AGENTS.md](../../../../AGENTS.md)
- [rust/crates/tinyquant-js/AGENTS.md](../../../../rust/crates/tinyquant-js/AGENTS.md)
