// src/index.ts
//
// Phase 25.1 public surface: re-export `version()` from the native
// binding. The codec / corpus / backend re-exports land in
// Phase 25.4 after the Rust value-object surface (Phase 25.2) and
// the parity test harness (Phase 25.3) are in place.
import { native } from "./_loader.js";

export const version: () => string = native.version;
