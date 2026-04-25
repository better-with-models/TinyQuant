// src/corpus.ts
//
// TypeScript wrapper over the native corpus bindings. The native
// side exposes JSON-stringified events and a millisecond-epoch
// timestamp (see `rust/crates/tinyquant-js/src/corpus.rs`); this
// module layers idiomatic JS types over that surface:
//
//   * `CompressionPolicy.COMPRESS / PASSTHROUGH / FP16` statics
//   * `Corpus.vectorIds` returns a `ReadonlySet<string>`
//   * `Corpus.pendingEvents()` returns a typed `CorpusEvent[]`
//   * `VectorEntry.insertedAt` returns a `Date`
//   * Metadata round-trips through a JSON string at the FFI seam
//
// Behaviour is delegated to the native `Corpus` aggregate in
// `tinyquant-core` — this file is a presentational shim, no domain
// logic lives here.
import { native } from "./_loader.js";
import type { Codebook, CodecConfig, CompressedVector, ConfigHash } from "./codec.js";
import { TinyQuantError } from "./_errors.js";

/**
 * Canonical compression-policy tag: `"compress"`, `"passthrough"`,
 * or `"fp16"`.
 */
export type CompressionPolicyKind = "compress" | "passthrough" | "fp16";

/**
 * Storage dtype tag produced by {@link CompressionPolicy.storageDtype}.
 * Drives decompression routines that need to pick the right byte
 * interpretation without inspecting the policy directly.
 */
export type StorageDtype = "uint8" | "float16" | "float32";

interface NativeCompressionPolicy {
  readonly kind: CompressionPolicyKind;
  requiresCodec(): boolean;
  storageDtype(): StorageDtype;
}

interface NativeCompressionPolicyClass {
  compress(): NativeCompressionPolicy;
  passthrough(): NativeCompressionPolicy;
  fp16(): NativeCompressionPolicy;
}

interface NativeVectorEntry {
  readonly vectorId: string;
  compressed(): CompressedVector;
  readonly configHash: ConfigHash;
  readonly dimension: number;
  readonly hasResidual: boolean;
  readonly insertedAtMillis: number;
  readonly metadataJson: string | null;
}

interface NativeCorpus {
  readonly corpusId: string;
  codecConfig(): CodecConfig;
  codebook(): Codebook;
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

interface NativeCorpusClass {
  new (
    corpusId: string,
    codecConfig: CodecConfig,
    codebook: Codebook,
    compressionPolicy: NativeCompressionPolicy,
    metadataJson: string | null | undefined,
  ): NativeCorpus;
}

const n = native as unknown as {
  CompressionPolicy: NativeCompressionPolicyClass;
  VectorEntry: new () => NativeVectorEntry;
  Corpus: NativeCorpusClass;
};

/**
 * Immutable compression policy enum. Exposed as three canonical
 * instances (`COMPRESS`, `PASSTHROUGH`, `FP16`) to mirror the
 * Python `tinyquant_cpu.corpus.CompressionPolicy` enum.
 */
export class CompressionPolicy {
  /** Full codec pipeline: rotate → scalar quantise → optional FP16 residual. */
  static readonly COMPRESS: CompressionPolicy = new CompressionPolicy(
    n.CompressionPolicy.compress(),
  );
  /** Store raw FP32 bytes with no transformation (1x ratio). */
  static readonly PASSTHROUGH: CompressionPolicy = new CompressionPolicy(
    n.CompressionPolicy.passthrough(),
  );
  /** Cast each FP32 to FP16 little-endian (2x ratio). */
  static readonly FP16: CompressionPolicy = new CompressionPolicy(n.CompressionPolicy.fp16());

  /** @internal - held for handoff to the native `Corpus` constructor. */
  readonly native: NativeCompressionPolicy;
  /** Canonical tag: `"compress" | "passthrough" | "fp16"`. */
  readonly kind: CompressionPolicyKind;

  private constructor(native: NativeCompressionPolicy) {
    this.native = native;
    this.kind = native.kind;
  }

  /** True only for `COMPRESS` — other policies bypass the codec. */
  requiresCodec(): boolean {
    return this.native.requiresCodec();
  }

  /** Storage dtype — drives byte-reinterpretation at decompress time. */
  storageDtype(): StorageDtype {
    return this.native.storageDtype();
  }
}

/**
 * One compressed vector entry in the corpus.
 *
 * Equality is by `vectorId` alone (matches the core's `PartialEq`
 * contract). `insertedAt` is a `Date` derived from the native
 * millisecond-epoch value; the underlying core storage is
 * nanosecond-precision but JS `Date` only resolves to milliseconds.
 */
export class VectorEntry {
  readonly vectorId: string;
  readonly compressed: CompressedVector;
  readonly insertedAt: Date;
  /**
   * Phase 25.3 deviation: the native side does not yet thread
   * metadata through `CoreCorpus`, so this getter always returns
   * `null` regardless of what was passed to `insert` / `insertBatch`.
   * Tracked for Phase 25.4 or beyond — see
   * `rust/crates/tinyquant-js/src/corpus.rs` `metadata_json` getter
   * for the rationale.
   */
  readonly metadata: Record<string, unknown> | null;
  readonly configHash: ConfigHash;
  readonly dimension: number;
  readonly hasResidual: boolean;

  /** @internal */
  constructor(native: NativeVectorEntry) {
    this.vectorId = native.vectorId;
    this.compressed = native.compressed();
    this.insertedAt = new Date(native.insertedAtMillis);
    this.metadata = parseMetadataJson(native.metadataJson);
    this.configHash = native.configHash;
    this.dimension = native.dimension;
    this.hasResidual = native.hasResidual;
  }
}

/**
 * Options bag for {@link Corpus}. Mirrors the Python keyword-argument
 * constructor signature.
 */
export interface CorpusOpts {
  corpusId: string;
  codecConfig: CodecConfig;
  codebook: Codebook;
  compressionPolicy: CompressionPolicy;
  metadata?: Record<string, unknown>;
}

/**
 * Common fields on every {@link CorpusEvent}. `timestamp` is a `Date`
 * converted from the native millisecond-epoch float.
 */
interface CorpusEventBase {
  readonly corpusId: string;
  readonly timestamp: Date;
}

/** Emitted when a `Corpus` is constructed. */
export interface CorpusCreatedEvent extends CorpusEventBase {
  readonly type: "CorpusCreated";
  readonly compressionPolicy: CompressionPolicyKind;
}

/** Emitted after one or more vectors are inserted. */
export interface VectorsInsertedEvent extends CorpusEventBase {
  readonly type: "VectorsInserted";
  readonly vectorIds: readonly string[];
  readonly count: number;
}

/** Emitted after `decompressAll()` succeeds. */
export interface CorpusDecompressedEvent extends CorpusEventBase {
  readonly type: "CorpusDecompressed";
  readonly vectorCount: number;
}

/** Emitted when a policy or dimension violation is detected. */
export interface CompressionPolicyViolationDetectedEvent extends CorpusEventBase {
  readonly type: "CompressionPolicyViolationDetected";
  readonly violationType: string;
  readonly detail: string;
}

/** Union of every event kind `Corpus.pendingEvents()` may produce. */
export type CorpusEvent =
  | CorpusCreatedEvent
  | VectorsInsertedEvent
  | CorpusDecompressedEvent
  | CompressionPolicyViolationDetectedEvent;

/**
 * Aggregate root — a collection of compressed vectors under one
 * codec config + policy. Mirrors `tinyquant_cpu.corpus.Corpus`.
 *
 * @example
 * ```ts
 * const corpus = new Corpus({
 *   corpusId: "docs",
 *   codecConfig: cfg,
 *   codebook: cb,
 *   compressionPolicy: CompressionPolicy.COMPRESS,
 * });
 * corpus.insert("doc-1", vector);
 * const events = corpus.pendingEvents();
 * ```
 */
export class Corpus {
  readonly #native: NativeCorpus;
  readonly codecConfig: CodecConfig;
  readonly codebook: Codebook;
  // Private field + getter so the property descriptor is an accessor (no setter).
  // TypeScript `readonly` only prevents compile-time writes; a data property is
  // still writable at runtime via untyped assignment. An accessor property with
  // no setter is non-writable in strict mode (throws TypeError) and silently a
  // no-op in sloppy mode — either way the value cannot change after construction.
  readonly #compressionPolicy: CompressionPolicy;
  readonly metadata: Record<string, unknown> | null;

  get compressionPolicy(): CompressionPolicy {
    return this.#compressionPolicy;
  }

  constructor(opts: CorpusOpts) {
    const metadataJson =
      opts.metadata !== undefined ? JSON.stringify(opts.metadata) : null;
    this.#native = new n.Corpus(
      opts.corpusId,
      opts.codecConfig,
      opts.codebook,
      opts.compressionPolicy.native,
      metadataJson,
    );
    this.codecConfig = opts.codecConfig;
    this.codebook = opts.codebook;
    this.#compressionPolicy = opts.compressionPolicy;
    this.metadata = opts.metadata ?? null;
  }

  get corpusId(): string {
    return this.#native.corpusId;
  }

  get vectorCount(): number {
    return this.#native.vectorCount;
  }

  get isEmpty(): boolean {
    return this.#native.isEmpty;
  }

  /**
   * Returns a fresh snapshot `Set` built from the native vector-id
   * list. **O(n) to construct**; O(1) per `.has()` lookup thereafter.
   * **Cache the result if iterating** — re-accessing the getter
   * rebuilds the set from the native list on every call.
   *
   * Iteration order matches insertion order in the Rust core up to
   * the native-side `HashMap` conversion.
   */
  get vectorIds(): ReadonlySet<string> {
    return new Set(this.#native.vectorIds);
  }

  /** Insert a single vector. Duplicate ids throw. */
  insert(
    vectorId: string,
    vector: Float32Array,
    metadata?: Record<string, unknown>,
  ): VectorEntry {
    const metadataJson = metadata !== undefined ? JSON.stringify(metadata) : null;
    return new VectorEntry(this.#native.insert(vectorId, vector, metadataJson));
  }

  /**
   * Atomic batch insert — on error the corpus is unchanged. One
   * `VectorsInserted` event covers the full batch on success.
   */
  insertBatch(
    vectors: Record<string, Float32Array>,
    metadata?: Record<string, Record<string, unknown>>,
  ): VectorEntry[] {
    const metadataJson = metadata !== undefined ? JSON.stringify(metadata) : null;
    const natives = this.#native.insertBatch(vectors, metadataJson);
    return natives.map((n) => new VectorEntry(n));
  }

  /** Look up a vector entry by id. Throws on unknown ids. */
  get(vectorId: string): VectorEntry {
    return new VectorEntry(this.#native.get(vectorId));
  }

  /** Does this corpus contain a vector with the given id? */
  contains(vectorId: string): boolean {
    return this.#native.contains(vectorId);
  }

  /** Decompress a single vector by id. */
  decompress(vectorId: string): Float32Array {
    return this.#native.decompress(vectorId);
  }

  /**
   * Decompress every vector. Emits a `CorpusDecompressed` event on
   * success; no event is emitted for an empty corpus.
   */
  decompressAll(): Record<string, Float32Array> {
    return this.#native.decompressAll();
  }

  /**
   * Remove a vector silently (matches Python `del corpus[id]`).
   * Returns `true` when the id was present, `false` when it was not.
   */
  remove(vectorId: string): boolean {
    return this.#native.remove(vectorId);
  }

  /**
   * Drain and return pending domain events. Each call empties the
   * buffer — subsequent calls see only events emitted after.
   */
  pendingEvents(): CorpusEvent[] {
    return this.#native.pendingEvents().map(parseEventJson);
  }
}

function parseMetadataJson(raw: string | null): Record<string, unknown> | null {
  if (raw === null || raw === undefined) return null;
  try {
    return JSON.parse(raw) as Record<string, unknown>;
  } catch (err) {
    // Silently falling back to `null` on malformed JSON would hide
    // schema drift between the native side and the TS wrapper — a
    // genuine bug that's worth surfacing. Escalate to a typed error.
    throw new TinyQuantError(
      "MetadataParseError",
      `@better-with-models/tinyquant-core: failed to parse metadata JSON from the native side: ${
        err instanceof Error ? err.message : String(err)
      }`,
      err,
    );
  }
}

function parseEventJson(raw: string): CorpusEvent {
  const parsed = JSON.parse(raw) as Record<string, unknown>;
  const millis = parsed["timestampMillis"];
  if (typeof millis !== "number") {
    // Silently substituting epoch 1970 would make a schema drift
    // between the Rust event emitter and this parser look like a
    // legitimate (but very stale) event. Escalate instead.
    throw new TinyQuantError(
      "CorpusEventSchemaError",
      `@better-with-models/tinyquant-core: corpus event is missing a numeric timestampMillis field: ${JSON.stringify(parsed["type"])}`,
    );
  }
  const timestamp = new Date(millis);
  const base = {
    corpusId: String(parsed["corpusId"] ?? ""),
    timestamp,
  };
  switch (parsed["type"]) {
    case "CorpusCreated":
      return {
        ...base,
        type: "CorpusCreated",
        compressionPolicy: parsed["compressionPolicy"] as CompressionPolicyKind,
      };
    case "VectorsInserted":
      return {
        ...base,
        type: "VectorsInserted",
        vectorIds: (parsed["vectorIds"] as string[] | undefined) ?? [],
        count: Number(parsed["count"] ?? 0),
      };
    case "CorpusDecompressed":
      return {
        ...base,
        type: "CorpusDecompressed",
        vectorCount: Number(parsed["vectorCount"] ?? 0),
      };
    case "CompressionPolicyViolationDetected":
      return {
        ...base,
        type: "CompressionPolicyViolationDetected",
        violationType: String(parsed["violationType"] ?? ""),
        detail: String(parsed["detail"] ?? ""),
      };
    default:
      // Surface the unknown type so the TS union stays honest; this
      // only fires on a schema drift between Rust and TS.
      throw new TinyQuantError(
        "CorpusEventSchemaError",
        `@better-with-models/tinyquant-core: unknown CorpusEvent type ${JSON.stringify(parsed["type"])}`,
      );
  }
}
