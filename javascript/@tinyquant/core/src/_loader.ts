// src/_loader.ts
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import path from "node:path";
import fs from "node:fs";

// Bun and Node ≥ 20.11 both set `import.meta.dirname`.
// Fallback covers Node 20.10.
const HERE =
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (import.meta as any).dirname ??
  path.dirname(fileURLToPath(import.meta.url));

const req = createRequire(import.meta.url);

// Loading libc detection lazily so cold-start cost stays near zero on
// non-Linux platforms. `detect-libc` synchronously probes
// `getconf GNU_LIBC_VERSION` or `/lib/ld-musl-*` as appropriate.
function linuxVariant(): "gnu" | "musl" {
  try {
    // We deliberately use the library (battle-tested across Alpine/
    // Debian/RHEL/Amazon Linux) rather than hand-rolling a probe of
    // `/etc/alpine-release`. The library also handles containerized
    // environments where `/etc/os-release` lies about the libc.
    const detect = req("detect-libc") as { familySync: () => string };
    return detect.familySync() === "musl" ? "musl" : "gnu";
  } catch {
    // If detect-libc is missing (shouldn't happen — it's a direct
    // dep) default to gnu, which matches the most common Linux host.
    return "gnu";
  }
}

export function binaryKey(): string {
  const { platform, arch } = process;

  if (platform === "linux") {
    const libc = linuxVariant();
    if (arch === "x64") return `linux-x64-${libc}`;
    if (arch === "arm64") return `linux-arm64-${libc}`;
  } else if (platform === "darwin") {
    if (arch === "x64") return "darwin-x64";
    if (arch === "arm64") return "darwin-arm64";
  } else if (platform === "win32") {
    if (arch === "x64") return "win32-x64-msvc";
  }

  throw new Error(
    `@tinyquant/core: no pre-built binary for ${platform}/${arch}. ` +
      `Supported combinations: linux/x64, linux/arm64, darwin/x64, ` +
      `darwin/arm64, win32/x64. Please open an issue at ` +
      `https://github.com/better-with-models/TinyQuant/issues.`,
  );
}

type NativeBinding = {
  version: () => string;
  // Codec
  CodecConfig: unknown;
  Codebook: unknown;
  CompressedVector: unknown;
  RotationMatrix: unknown;
  compress: unknown;
  decompress: unknown;
  // Corpus
  Corpus: unknown;
  VectorEntry: unknown;
  CompressionPolicy: unknown;
  // Backend
  BruteForceBackend: unknown;
  SearchResult: unknown;
};

function loadNative(): NativeBinding {
  const key = binaryKey();
  // Keep path.join so Windows backslashes are inserted correctly —
  // `require()` on Windows accepts either separator but
  // `path.join` produces the platform-native form, which shows up
  // cleanly in stack traces.
  const candidate = path.join(HERE, "..", "binaries", `${key}.node`);

  if (!fs.existsSync(candidate)) {
    throw new Error(
      `@tinyquant/core: expected bundled binary at ${candidate} ` +
        `but file is missing. The package tarball may have been ` +
        `truncated; try reinstalling.`,
    );
  }

  try {
    return req(candidate) as NativeBinding;
  } catch (err) {
    const detail = err instanceof Error ? err.message : String(err);
    throw new Error(
      `@tinyquant/core: failed to load native binary at ${candidate}: ${detail}`,
    );
  }
}

export const native = loadNative();
