// src/index.ts
//
// Public entry point for `tinyquant`. Each sub-module adds
// hand-written JSDoc on top of the native binding and keeps the
// import surface organised by architectural layer:
//
//   - `./codec`   — codec value objects and primitives
//   - `./corpus`  — aggregate root + events
//   - `./backend` — search backends
//   - `./_errors` — structured error wrapper
//
// `version` is re-exported directly from the loader because it is
// a naked function on the native binding with no TS polish needed.
export * from "./codec.js";
export * from "./corpus.js";
export * from "./backend.js";
export { TinyQuantError } from "./_errors.js";

import { native } from "./_loader.js";

/** Semver of the installed native binding, sourced from Cargo.toml. */
export const version: () => string = native.version;
