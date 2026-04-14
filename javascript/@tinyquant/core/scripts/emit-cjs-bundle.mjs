#!/usr/bin/env node
// scripts/emit-cjs-bundle.mjs
//
// Emit a real CommonJS bundle at `dist/<name>.cjs` so consumers hitting
// the `exports.require` branch of `package.json` get actual CJS (not
// an ESM file renamed). TypeScript's `module: CommonJS` mode rejects
// `import.meta`, which `_loader.ts` uses for `import.meta.url` /
// `import.meta.dirname`, so we stage the sources into
// `src-cjs-staging/` with those two references rewritten to their CJS
// equivalents (`__dirname`, `pathToFileURL(__filename).href`) before
// handing the tree to `tsc -p tsconfig.cjs.json`. After the CJS tsc
// pass, the emitted `dist-cjs/*.js` files are renamed into
// `dist/*.cjs` and every `require("./<x>.js")` is rewritten to
// `require("./<x>.cjs")` so the CJS bundle is internally consistent.
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import url from "node:url";

const HERE = path.dirname(url.fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, "..");
const SRC = path.join(ROOT, "src");
const STAGE = path.join(ROOT, "src-cjs-staging");
const DIST_CJS = path.join(ROOT, "dist-cjs");
const DIST = path.join(ROOT, "dist");

// --- 1. Stage sources with import.meta rewrites.
fs.rmSync(STAGE, { recursive: true, force: true });
fs.mkdirSync(STAGE, { recursive: true });
for (const name of fs.readdirSync(SRC)) {
  if (!name.endsWith(".ts")) continue;
  let text = fs.readFileSync(path.join(SRC, name), "utf8");
  // `import.meta.dirname` (Node ≥ 20.11) → `__dirname` in CJS.
  text = text.replace(
    /\(import\.meta as any\)\.dirname/g,
    "(__dirname as any)",
  );
  // `import.meta.url` → `pathToFileURL(__filename).href` in CJS.
  // Inline the require so we don't have to add an import at the top
  // of every staged file; `url` is a Node core module.
  text = text.replace(
    /import\.meta\.url/g,
    'require("url").pathToFileURL(__filename).href',
  );
  fs.writeFileSync(path.join(STAGE, name), text);
}

// --- 2. Run tsc on the staging tree. Invoke the TypeScript JS entry
// directly under the current Node, not via `npx.cmd` — Node 24+ on
// Windows refuses to spawn `.cmd`/`.bat` without `shell: true`, and
// enabling the shell would reintroduce the command-injection shape
// even though the argv here is fully static. Resolving the entry via
// `require.resolve("typescript/bin/tsc")` is robust to hoisting.
const req = url.pathToFileURL(path.join(ROOT, "package.json"));
const { createRequire } = await import("node:module");
const tscEntry = createRequire(req).resolve("typescript/bin/tsc");
const tscRun = spawnSync(
  process.execPath,
  [tscEntry, "-p", "tsconfig.cjs.json"],
  { cwd: ROOT, stdio: "inherit" },
);
if (tscRun.error) {
  console.error("emit-cjs-bundle: tsc spawn failed:", tscRun.error);
  process.exit(1);
}
if (tscRun.status !== 0) {
  process.exit(tscRun.status ?? 1);
}

// --- 3. Move dist-cjs/*.js → dist/*.cjs, rewriting require("./x.js").
fs.mkdirSync(DIST, { recursive: true });
const REQ_RE = /require\("(\.{1,2}\/[^"]+)\.js"\)/g;
let moved = 0;
for (const name of fs.readdirSync(DIST_CJS)) {
  const srcPath = path.join(DIST_CJS, name);
  if (name.endsWith(".js")) {
    const stem = name.slice(0, -".js".length);
    let body = fs.readFileSync(srcPath, "utf8");
    body = body.replace(REQ_RE, 'require("$1.cjs")');
    // Rewrite sourceMappingURL comment to the .cjs.map sibling.
    body = body.replace(
      /sourceMappingURL=(.+)\.js\.map/g,
      "sourceMappingURL=$1.cjs.map",
    );
    fs.writeFileSync(path.join(DIST, `${stem}.cjs`), body);
    moved += 1;
  } else if (name.endsWith(".js.map")) {
    const stem = name.slice(0, -".js.map".length);
    let body = fs.readFileSync(srcPath, "utf8");
    try {
      const json = JSON.parse(body);
      if (typeof json.file === "string") {
        json.file = json.file.replace(/\.js$/u, ".cjs");
      }
      body = JSON.stringify(json);
    } catch {
      // If the map isn't JSON for any reason, leave it alone.
    }
    fs.writeFileSync(path.join(DIST, `${stem}.cjs.map`), body);
  }
}

// --- 4. Clean up transient dirs.
fs.rmSync(DIST_CJS, { recursive: true, force: true });
fs.rmSync(STAGE, { recursive: true, force: true });
console.log(`emit-cjs-bundle: emitted ${moved} dist/*.cjs files`);
