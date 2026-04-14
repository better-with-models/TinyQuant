// src/_loader.ts
//
// Binary layout (npm package, authoritative):
//   binaries/<triple>.node    (e.g. binaries/linux-x64-gnu.node)
// This is distinct from the Python fat wheel's `_lib/<key>/` layout
// because the npm package publishes one tarball per platform via
// `optionalDependencies`, whereas the Python wheel bundles every
// supported arch into a single fat tarball. Both layouts are
// intentional — do not unify them.
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

// Phase 25.2 surfaces: codec value objects + Codec + module-level
// compress/decompress. Corpus / backend / TS-wrapper polish land in
// later slices. The shape below mirrors what `napi build --dts`
// auto-generates for the `#[napi]` annotations in
// `rust/crates/tinyquant-js/src/codec.rs`; we type it by hand here
// so tooling that bypasses the generated `.d.ts` still type-checks.

export interface CodecConfigOpts {
  bitWidth: number;
  seed: bigint;
  dimension: number;
  residualEnabled?: boolean;
}

export interface NativeCodecConfig {
  readonly bitWidth: number;
  readonly seed: bigint;
  readonly dimension: number;
  readonly residualEnabled: boolean;
  readonly numCodebookEntries: number;
  readonly configHash: string;
}

export interface NativeCodebook {
  readonly bitWidth: number;
  readonly numEntries: number;
  // `entries` is a method, not a getter — each call allocates and
  // memcpy's the full buffer. Surfacing it as a function signature
  // syntactically signals the allocation cost at every call site.
  entries(): Float32Array;
}

export interface NativeCompressedVector {
  readonly configHash: string;
  readonly dimension: number;
  readonly bitWidth: number;
  readonly hasResidual: boolean;
  readonly sizeBytes: number;
  // `indices` is a method, not a getter — see `NativeCodebook.entries`.
  indices(): Uint8Array;
}

export interface NativeRotationMatrix {
  readonly seed: bigint;
  readonly dimension: number;
}

export interface NativeCodec {
  compress(
    vector: Float32Array,
    config: NativeCodecConfig,
    codebook: NativeCodebook,
  ): NativeCompressedVector;
  decompress(
    compressed: NativeCompressedVector,
    config: NativeCodecConfig,
    codebook: NativeCodebook,
  ): Float32Array;
}

// Phase 25.3 surfaces: corpus + backend value objects. Same
// hand-typed-alongside-auto-generated contract as the codec types —
// `_loader.ts` stays the single source of truth for the native
// binding's JS-visible shape so tooling that bypasses the generated
// `.d.ts` still type-checks.
export interface NativeCompressionPolicy {
  readonly kind: "compress" | "passthrough" | "fp16";
  requiresCodec(): boolean;
  storageDtype(): "uint8" | "float16" | "float32";
}

export interface NativeCompressionPolicyClass {
  compress(): NativeCompressionPolicy;
  passthrough(): NativeCompressionPolicy;
  fp16(): NativeCompressionPolicy;
}

export interface NativeVectorEntry {
  readonly vectorId: string;
  compressed(): NativeCompressedVector;
  readonly configHash: string;
  readonly dimension: number;
  readonly hasResidual: boolean;
  readonly insertedAtMillis: number;
  readonly metadataJson: string | null;
}

export interface NativeCorpus {
  readonly corpusId: string;
  codecConfig(): NativeCodecConfig;
  codebook(): NativeCodebook;
  compressionPolicy(): NativeCompressionPolicy;
  readonly metadataJson: string | null;
  readonly vectorCount: number;
  readonly isEmpty: boolean;
  readonly vectorIds: string[];
  insert(
    vectorId: string,
    vector: Float32Array,
    metadataJson: string | null | undefined,
  ): NativeVectorEntry;
  insertBatch(
    vectors: Record<string, Float32Array>,
    metadataJson: string | null | undefined,
  ): NativeVectorEntry[];
  get(vectorId: string): NativeVectorEntry;
  contains(vectorId: string): boolean;
  decompress(vectorId: string): Float32Array;
  decompressAll(): Record<string, Float32Array>;
  remove(vectorId: string): boolean;
  pendingEvents(): string[];
}

export interface NativeCorpusClass {
  new (
    corpusId: string,
    codecConfig: NativeCodecConfig,
    codebook: NativeCodebook,
    compressionPolicy: NativeCompressionPolicy,
    metadataJson: string | null | undefined,
  ): NativeCorpus;
}

export interface NativeSearchResult {
  readonly vectorId: string;
  readonly score: number;
}

export interface NativeBruteForceBackend {
  readonly count: number;
  ingest(vectors: Record<string, Float32Array>): void;
  search(query: Float32Array, topK: number): NativeSearchResult[];
  remove(vectorIds: readonly string[]): void;
  clear(): void;
}

type NativeBinding = {
  version: () => string;
  CodecConfig: new (opts: CodecConfigOpts) => NativeCodecConfig;
  Codebook: {
    train(vectors: Float32Array, config: NativeCodecConfig): NativeCodebook;
  };
  // CompressedVector is not directly constructed from JS — it is
  // produced by `Codec.compress` / the module-level `compress` — but
  // napi-rs still exposes the class on the binding for `instanceof`
  // checks. Typed as a no-arg constructor returning the instance shape.
  CompressedVector: new () => NativeCompressedVector;
  RotationMatrix: {
    fromConfig(config: NativeCodecConfig): NativeRotationMatrix;
  };
  Codec: new () => NativeCodec;
  compress: (
    vector: Float32Array,
    config: NativeCodecConfig,
    codebook: NativeCodebook,
  ) => NativeCompressedVector;
  decompress: (
    compressed: NativeCompressedVector,
    config: NativeCodecConfig,
    codebook: NativeCodebook,
  ) => Float32Array;
  CompressionPolicy: NativeCompressionPolicyClass;
  VectorEntry: new () => NativeVectorEntry;
  Corpus: NativeCorpusClass;
  SearchResult: new () => NativeSearchResult;
  BruteForceBackend: new () => NativeBruteForceBackend;
};

// Walk up from `start` until we find a directory containing
// `package.json` whose `name` is `@tinyquant/core`. This keeps the
// loader robust regardless of how the source is packaged:
//
//   - published wheel:   dist/_loader.cjs     → pkg root  is `..`
//   - tsc dev build:     dist/_loader.js      → pkg root  is `..`
//   - test build:        dist-tests/src/_loader.js → pkg root is `../..`
//
// Instead of hard-coding any of those, anchor on the marker file.
function findPackageRoot(start: string): string {
  let dir = start;
  // Cap iteration so a mis-configured install can't spin forever.
  for (let i = 0; i < 8; i++) {
    const pjson = path.join(dir, "package.json");
    if (fs.existsSync(pjson)) {
      try {
        const parsed = JSON.parse(fs.readFileSync(pjson, "utf8")) as {
          name?: string;
        };
        if (parsed.name === "@tinyquant/core") return dir;
      } catch {
        // ignore malformed package.json on the walk up
      }
    }
    const parent = path.dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  // Fallback: the published-wheel layout (dist/ sibling of binaries/).
  return path.resolve(start, "..");
}

function loadNative(): NativeBinding {
  const key = binaryKey();
  // Keep path.join so Windows backslashes are inserted correctly —
  // `require()` on Windows accepts either separator but
  // `path.join` produces the platform-native form, which shows up
  // cleanly in stack traces.
  const root = findPackageRoot(HERE);
  const candidate = path.join(root, "binaries", `${key}.node`);

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
