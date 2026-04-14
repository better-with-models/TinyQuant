/*
 * tinyquant-sys C ABI compile-smoke test.
 *
 * Built by `tests/abi_c_smoke.rs` through the `cc` crate. The goal is
 * narrow: prove that the committed `include/tinyquant.h` parses as
 * strict C and that every symbol listed in this file has the expected
 * signature. Linking happens indirectly — the Rust test harness links
 * the `tinyquant-sys` rlib, which brings in the real implementations.
 *
 * See docs/plans/rust/phase-22-pyo3-cabi-release.md §C-ABI test surface.
 */

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>

#include "tinyquant.h"

/*
 * Exercise every entry point at the source level so that signature
 * changes are caught at compile time. The function is never called —
 * the compiler still type-checks the body and the linker does not need
 * these symbols because the shared object is provided by Rust in the
 * integration-test runtime.
 */
int tinyquant_sys_c_signature_probe(void)
{
    TinyQuantError err = {0};
    CodecConfigHandle *cfg = NULL;
    CodebookHandle *cb = NULL;
    CompressedVectorHandle *cv = NULL;
    CorpusHandle *corpus = NULL;
    uint8_t *bytes = NULL;
    uintptr_t bytes_len = 0;
    float buf[4] = {0};

    (void) tq_version();
    (void) tq_codec_config_new(4, 0, 4, true, &cfg, &err);
    (void) tq_codec_config_bit_width(cfg);
    (void) tq_codec_config_seed(cfg);
    (void) tq_codec_config_dimension(cfg);
    (void) tq_codec_config_residual_enabled(cfg);
    (void) tq_codec_config_hash(cfg);
    (void) tq_codebook_train(buf, 1, 4, cfg, &cb, &err);
    (void) tq_codebook_bit_width(cb);
    (void) tq_codec_compress(cfg, cb, buf, 4, &cv, &err);
    (void) tq_codec_decompress(cfg, cb, cv, buf, 4, &err);
    (void) tq_compressed_vector_bit_width(cv);
    (void) tq_compressed_vector_dimension(cv);
    (void) tq_compressed_vector_to_bytes(cv, &bytes, &bytes_len, &err);
    (void) tq_compressed_vector_from_bytes(bytes, bytes_len, &cv, &err);
    (void) tq_bytes_free(bytes, bytes_len);
    (void) tq_compressed_vector_free(cv);
    (void) tq_codebook_free(cb);

    TinyQuantCompressionPolicy policy = TINY_QUANT_COMPRESSION_POLICY_COMPRESS;
    (void) tq_corpus_new(NULL, cfg, cb, policy, &corpus, &err);
    (void) tq_corpus_insert(corpus, "id", buf, 4, 0, &err);
    (void) tq_corpus_vector_count(corpus);
    (void) tq_corpus_contains(corpus, "id");
    (void) tq_corpus_free(corpus);

    (void) tq_codec_config_free(cfg);
    (void) tq_error_free(&err);
    (void) tq_error_free_message(NULL);

    /* Error-kind constants must be compile-time integers. */
    TinyQuantErrorKind kinds[] = {
        TINY_QUANT_ERROR_KIND_OK,
        TINY_QUANT_ERROR_KIND_INVALID_HANDLE,
        TINY_QUANT_ERROR_KIND_INVALID_ARGUMENT,
        TINY_QUANT_ERROR_KIND_PANIC,
    };
    return (int) kinds[0];
}
