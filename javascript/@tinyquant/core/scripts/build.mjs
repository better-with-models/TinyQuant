#!/usr/bin/env node
// scripts/build.mjs
//
// Sequential driver for `npm run build`. The original package.json
// `build` script chained five steps (clean, napi build, rename, tsc,
// emit-cjs-types, emit-cjs-bundle) on one line with `&&`; extracting
// them to this file keeps each step individually invokable through
// the `scripts/*.mjs` set while giving `npm run build` a single
// entry point that reports which step failed.
//
// Individual steps remain runnable as before via `npm run clean`,
// `npm run build:cjs`, etc. — this script only composes them.

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import url from "node:url";

const HERE = path.dirname(url.fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, "..");
const REPO_ROOT = path.resolve(ROOT, "../../..");

/**
 * Run a single build step. `exe` is either a Node-resolvable entry
 * (resolved to an absolute path before spawn) or an argv array passed
 * straight through. `label` is the step name used in error reports.
 */
function runStep(label, exe, args, opts = {}) {
  process.stdout.write(`\n[build] ${label}...\n`);
  const result = spawnSync(exe, args, {
    cwd: opts.cwd ?? ROOT,
    stdio: "inherit",
    env: process.env,
  });
  if (result.error) {
    console.error(`[build] step failed (spawn): ${label}`);
    console.error(result.error);
    process.exit(1);
  }
  if (typeof result.status === "number" && result.status !== 0) {
    console.error(`[build] step failed (exit ${result.status}): ${label}`);
    process.exit(result.status);
  }
  if (result.status === null) {
    console.error(`[build] step terminated by signal: ${label} (${result.signal})`);
    process.exit(1);
  }
}

function runNode(label, script, args = []) {
  const scriptPath = path.join(ROOT, script);
  if (!fs.existsSync(scriptPath)) {
    console.error(`[build] missing script: ${scriptPath}`);
    process.exit(1);
  }
  runStep(label, process.execPath, [scriptPath, ...args]);
}

// --- Step 1: clean all build outputs.
runStep(
  "clean",
  process.execPath,
  [
    "-e",
    "for (const d of ['dist','dist-cjs','dist-tests','src-cjs-staging']) require('fs').rmSync(d,{recursive:true,force:true})",
  ],
);

// --- Step 2: napi build — produces the native binaries/ output.
// Invoke through npx so the locally-installed @napi-rs/cli is used.
// `shell: true` is intentional on Windows for the `.cmd` shim, but
// argv is static so there is no injection surface.
//
// Cross-compilation: if NAPI_CROSS_TARGET is set (e.g. when CI builds
// the darwin-x64 binary on a darwin-arm64 runner), keep --platform
// (so napi names the output with the platform suffix) and add
// --target <triple> so cargo cross-compiles for the right arch.
// NOTE: deliberately NOT CARGO_BUILD_TARGET — that is an official Cargo
// env var; setting it to empty string causes `cargo metadata` to fail
// with "target was empty".
const napiCrate = path.resolve(REPO_ROOT, "rust/crates/tinyquant-js");
const binariesOut = path.join(ROOT, "binaries");
const isWindows = process.platform === "win32";
const napiCmd = isWindows ? "npx.cmd" : "npx";
const crossTarget = process.env.NAPI_CROSS_TARGET || "";
process.stdout.write("\n[build] napi build (release)...\n");
{
  // napi-rs v3 requires --platform to include the platform name in the
  // output filename.  When cross-compiling, also pass --target so cargo
  // builds for the right triple while --platform controls the name.
  const napiTargetArgs = crossTarget
    ? ["--platform", "--target", crossTarget]
    : ["--platform"];
  const result = spawnSync(
    napiCmd,
    [
      "napi",
      "build",
      "--cwd",
      napiCrate,
      ...napiTargetArgs,
      "--release",
      "--output-dir",
      binariesOut,
    ],
    { cwd: ROOT, stdio: "inherit", shell: isWindows },
  );
  if (result.error || result.status !== 0) {
    console.error("[build] step failed: napi build");
    if (result.error) console.error(result.error);
    process.exit(result.status ?? 1);
  }
}

// --- Step 3: normalise napi-rs v3 output layout.
runNode("rename binaries", "scripts/rename-binaries.mjs");

// --- Step 4: tsc ESM build (dist/).
const tscArgs = ["-p", "tsconfig.json"];
const { createRequire } = await import("node:module");
const req = url.pathToFileURL(path.join(ROOT, "package.json"));
const tscEntry = createRequire(req).resolve("typescript/bin/tsc");
runStep("tsc (esm)", process.execPath, [tscEntry, ...tscArgs]);

// --- Step 5: emit CJS `.d.cts` typings.
runNode("emit-cjs-types", "scripts/emit-cjs-types.mjs");

// --- Step 6: emit the real CJS bundle (dist/*.cjs).
runNode("emit-cjs-bundle", "scripts/emit-cjs-bundle.mjs");

process.stdout.write("\n[build] all steps completed\n");
