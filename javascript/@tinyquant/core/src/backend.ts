// src/backend.ts
//
// Thin wrapper module for the search-backend layer. Re-exports
// `SearchResult` and `BruteForceBackend` from the native binding
// with hand-written JSDoc.
import { native } from "./_loader.js";

/**
 * Immutable `(vectorId, score)` pair returned by
 * {@link BruteForceBackend.search}. `score` is cosine similarity in
 * `[-1, 1]`; higher is better. Not directly constructable from JS —
 * produced only by `search`.
 */
export interface SearchResult {
  readonly vectorId: string;
  readonly score: number;
}

/**
 * Reference exhaustive cosine-similarity search backend. Suitable
 * for corpora up to ~100 000 vectors. Dimension-locks on the first
 * non-empty `ingest`; duplicate ids overwrite silently.
 *
 * @example
 * ```ts
 * const backend = new BruteForceBackend();
 * backend.ingest({ v1: new Float32Array([1, 0, 0]) });
 * const hits = backend.search(new Float32Array([1, 0, 0]), 1);
 * ```
 */
export interface BruteForceBackend {
  readonly count: number;
  ingest(vectors: Record<string, Float32Array>): void;
  search(query: Float32Array, topK: number): SearchResult[];
  remove(vectorIds: readonly string[]): void;
  clear(): void;
}

// `native` carries typed shapes from `_loader.ts`; each wrapper
// re-exposes the class through `typeof X` so TS consumers get an
// `instanceof`-compatible constructor in addition to the interface.
const n = native as unknown as {
  SearchResult: new () => SearchResult;
  BruteForceBackend: new () => BruteForceBackend;
};

/** Native class handle for `instanceof SearchResult` checks. */
export const SearchResult: new () => SearchResult = n.SearchResult;

/**
 * Native class handle. Construct with `new BruteForceBackend()`.
 */
export const BruteForceBackend: new () => BruteForceBackend = n.BruteForceBackend;
