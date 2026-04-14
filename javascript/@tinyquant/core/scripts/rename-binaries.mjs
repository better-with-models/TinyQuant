#!/usr/bin/env node
// Normalize napi-rs v3 output (`<napi.name>.<triple>.node`, e.g.
// `tinyquant.win32-x64-msvc.node`) to the loader's expected
// `<triple>.node` layout. See src/_loader.ts for the authoritative
// binaries/<triple>.node convention.
//
// napi-rs v3 prefixes the artifact with the crate's `napi.name` when
// `--platform --output-dir <dir>` is used; our loader (and the
// optionalDependencies layout that ships with the npm package)
// predates that change, so we normalize here.
//
// Recognized triples are drawn from _loader.ts's binaryKey() switch.
// Idempotent: running twice is a no-op.
import { readdirSync, renameSync, existsSync, rmSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join } from "node:path";

const KNOWN_TRIPLES = new Set([
  "linux-x64-gnu",
  "linux-x64-musl",
  "linux-arm64-gnu",
  "linux-arm64-musl",
  "darwin-x64",
  "darwin-arm64",
  "win32-x64-msvc",
]);

const dir = fileURLToPath(new URL("../binaries/", import.meta.url));

if (!existsSync(dir)) {
  // Nothing to do — `napi build` hasn't run, or was invoked with a
  // different output directory. Let the caller surface that error.
  process.exit(0);
}

for (const name of readdirSync(dir)) {
  if (!name.endsWith(".node")) continue;
  const stem = name.slice(0, -".node".length);
  // Accept both `<prefix>.<triple>.node` (napi-rs v3 with --platform)
  // and bare `<triple>.node` (already normalized). Strip the first
  // dot-separated segment iff the remainder is a known triple.
  const dotIdx = stem.indexOf(".");
  if (dotIdx < 0) continue; // already `<triple>.node`-only — skip.
  const triple = stem.slice(dotIdx + 1);
  if (!KNOWN_TRIPLES.has(triple)) continue;
  const from = join(dir, name);
  const to = join(dir, `${triple}.node`);
  if (from === to) continue;
  if (existsSync(to)) {
    // Overwrite: a stale `<triple>.node` from a previous build would
    // shadow the fresh `<prefix>.<triple>.node` we just emitted.
    rmSync(to);
  }
  renameSync(from, to);
  console.log(`renamed ${name} -> ${triple}.node`);
}
