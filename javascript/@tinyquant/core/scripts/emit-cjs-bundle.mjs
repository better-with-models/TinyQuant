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

// Wrap the stage + tsc + rename sequence in a try/finally so a mid-
// pipeline crash (e.g. tsc non-zero exit, a rewrite regex throwing)
// still cleans up the transient `src-cjs-staging/` and `dist-cjs/`
// dirs. Without this, a failed `npm run build:cjs` leaves stale
// scaffolding behind that only `npm run clean` could sweep away.
//
// IMPORTANT: failures inside this block MUST `throw` (not call
// `process.exit`). `process.exit()` terminates the event loop
// synchronously and skips pending `finally` blocks — which would
// defeat the whole point of this try/finally cleanup. The top-level
// `main()` wrapper below catches the throw and sets `exitCode = 1`
// so the script still exits non-zero without short-circuiting
// cleanup.
async function main() {
try {
  // --- 1. Stage sources with import.meta rewrites.
  fs.rmSync(STAGE, { recursive: true, force: true });
  fs.mkdirSync(STAGE, { recursive: true });
  const stagedFiles = [];
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
    const dest = path.join(STAGE, name);
    fs.writeFileSync(dest, text);
    stagedFiles.push(dest);
  }

  // Sanity: staging must not contain any lingering `import.meta`
  // reference in executable code after the rewrite pass. The rewrites
  // above are string-replacement-based and would silently miss a
  // newly-added form (e.g. `import.meta.resolve`), producing a CJS
  // bundle that throws `SyntaxError: Cannot use 'import.meta' outside
  // a module` at runtime. Fail loudly here instead.
  //
  // Single-line `//` comments and `/* ... */` block comments are
  // stripped before the check so documentation that mentions
  // `import.meta.dirname` does not trigger a false positive. This
  // comment-stripper is intentionally simple (it does not handle
  // `import.meta` inside strings) — the point is to catch genuine
  // rewrite misses, not to be a full TypeScript tokenizer.
  const stripComments = (text) =>
    text
      .replace(/\/\*[\s\S]*?\*\//g, "") // block comments
      .replace(/(^|[^:])\/\/[^\n]*/g, "$1"); // line comments (not URLs)
  for (const f of stagedFiles) {
    const content = fs.readFileSync(f, "utf8");
    if (/import\.meta/.test(stripComments(content))) {
      throw new Error(
        `emit-cjs-bundle: CJS rewrite missed an import.meta reference in ${f}`,
      );
    }
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
    throw new Error("tsc spawn failed: " + tscRun.error.message);
  }
  if (tscRun.status !== 0) {
    throw new Error("tsc exited " + (tscRun.status ?? 1));
  }

  // --- 3. Move dist-cjs/*.js → dist/*.cjs, rewriting require("./x.js").
  fs.mkdirSync(DIST, { recursive: true });
  // Invariant: we only rewrite OUR OWN tsc output in dist-cjs/ (never
  // third-party code). Relative requires are rewritten .js -> .cjs;
  // bare specifiers (`node:...`, `foo`) are untouched.
  const REQ_RE = /require\("(\.{1,2}\/[^"]+)\.js"\)/g;
  let moved = 0;
  for (const name of fs.readdirSync(DIST_CJS)) {
    const srcPath = path.join(DIST_CJS, name);
    if (name.endsWith(".js")) {
      const stem = name.slice(0, -".js".length);
      let body = fs.readFileSync(srcPath, "utf8");
      body = body.replace(REQ_RE, 'require("$1.cjs")');
      // Same invariant as REQ_RE: only rewrite our own tsc-emitted
      // sourceMappingURL comments; third-party `.js.map` references
      // never appear in this file set.
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

  console.log(`emit-cjs-bundle: emitted ${moved} dist/*.cjs files`);
} finally {
  // --- 4. Always clean up transient dirs, even on crash, so the
  // next build starts from a known state.
  fs.rmSync(DIST_CJS, { recursive: true, force: true });
  fs.rmSync(STAGE, { recursive: true, force: true });
}
}

try {
  await main();
} catch (err) {
  console.error(err instanceof Error ? err.message : err);
  // Use `process.exitCode = 1` (not `process.exit(1)`) at the top
  // level so any pending microtasks — including cleanup that ran
  // inside the `finally` above — flush before termination.
  process.exitCode = 1;
}
