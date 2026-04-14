# @tinyquant/core

CPU-only vector quantization codec for embedding storage compression.

> [!warning] Phase 25 work-in-progress
> This package is the Phase 25.1 scaffold. It currently exports only
> `version()`; the public codec / corpus / backend surface lands in
> Phase 25.2 – 25.4. Do not depend on this release line in production
> until Phase 25 ships its first stable tag.

## Install

```bash
npm install @tinyquant/core
# or
bun add @tinyquant/core
```

## Supported runtimes

- Node.js `>=20.10.0`
- Bun `>=1.1.0`

Pre-built native binaries ship for `linux-x64-gnu`, `linux-arm64-gnu`,
`darwin-x64`, `darwin-arm64`, and `win32-x64-msvc`.

## API reference

Full API docs will land alongside Phase 25.4. Until then the only
callable export is:

```ts
import { version } from "@tinyquant/core";

console.log(version()); // "0.1.0"
```

## License

Apache-2.0 — see `LICENSE`.
