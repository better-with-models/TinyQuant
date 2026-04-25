#!/usr/bin/env node
// scripts/emit-cjs-types.mjs
//
// Copy every `dist/*.d.ts` → `dist/*.d.cts` so consumers resolving
// the `require:` branch of the `exports` map in `package.json` see
// typings. The type content is identical between ESM and CJS for
// this package — each wrapper re-exports the same native class
// handles with no `export default` / `module.exports` divergence —
// so a byte-for-byte copy is the right artefact.
//
// Rationale for a script over a second `tsc` pass: the `.d.ts`
// output already encodes the public surface; running `tsc` twice
// would double the emit time without producing different text.
// When a future change introduces a real ESM/CJS type divergence
// (e.g. `export default` interop), this script becomes the seam
// where we fork the two outputs.
import fs from "node:fs";
import path from "node:path";
import url from "node:url";

const HERE = path.dirname(url.fileURLToPath(import.meta.url));
const DIST = path.resolve(HERE, "..", "dist");

if (!fs.existsSync(DIST)) {
  console.error(`emit-cjs-types: ${DIST} missing; run \`tsc\` first.`);
  process.exit(1);
}

const entries = fs.readdirSync(DIST, { withFileTypes: true });
let count = 0;
for (const ent of entries) {
  if (!ent.isFile()) continue;
  if (!ent.name.endsWith(".d.ts") || ent.name.endsWith(".d.cts")) continue;
  const src = path.join(DIST, ent.name);
  const dst = path.join(DIST, ent.name.replace(/\.d\.ts$/u, ".d.cts"));
  fs.copyFileSync(src, dst);
  count += 1;
}
console.log(`emit-cjs-types: copied ${count} .d.ts → .d.cts files`);
