// tinyquant-sys C++ ABI compile-smoke test.
//
// Included from C++; the header must stay C-compat AND C++-compat.
// cbindgen's `cpp_compat = true` emits the `extern "C"` guard.

#include <cstddef>
#include <cstdint>
#include <cstring>

#include "tinyquant.h"

// A C++ translation unit that round-trips every type declared by the
// header. Called for its side effect of forcing the compiler to parse
// every declaration at C++ semantics.
extern "C" int tinyquant_sys_cxx_signature_probe()
{
    TinyQuantError err{};
    CodecConfigHandle* cfg = nullptr;
    CodebookHandle* cb = nullptr;
    CompressedVectorHandle* cv = nullptr;
    CorpusHandle* corpus = nullptr;
    uint8_t* bytes = nullptr;
    uintptr_t bytes_len = 0;
    float buf[4] = {0};

    static_cast<void>(tq_version());
    static_cast<void>(tq_codec_config_new(4, 0, 4, true, &cfg, &err));
    static_cast<void>(tq_codec_config_bit_width(cfg));
    static_cast<void>(tq_codec_config_seed(cfg));
    static_cast<void>(tq_codec_config_dimension(cfg));
    static_cast<void>(tq_codec_config_residual_enabled(cfg));
    static_cast<void>(tq_codec_config_hash(cfg));
    static_cast<void>(tq_codebook_train(buf, 1, 4, cfg, &cb, &err));
    static_cast<void>(tq_codebook_bit_width(cb));
    static_cast<void>(tq_codec_compress(cfg, cb, buf, 4, &cv, &err));
    static_cast<void>(tq_codec_decompress(cfg, cb, cv, buf, 4, &err));
    static_cast<void>(tq_compressed_vector_bit_width(cv));
    static_cast<void>(tq_compressed_vector_dimension(cv));
    static_cast<void>(tq_compressed_vector_to_bytes(cv, &bytes, &bytes_len, &err));
    static_cast<void>(tq_compressed_vector_from_bytes(bytes, bytes_len, &cv, &err));
    tq_bytes_free(bytes, bytes_len);
    tq_compressed_vector_free(cv);
    tq_codebook_free(cb);

    TinyQuantCompressionPolicy policy = TINY_QUANT_COMPRESSION_POLICY_PASSTHROUGH;
    static_cast<void>(tq_corpus_new(nullptr, cfg, cb, policy, &corpus, &err));
    static_cast<void>(tq_corpus_insert(corpus, "id", buf, 4, 0, &err));
    static_cast<void>(tq_corpus_vector_count(corpus));
    static_cast<void>(tq_corpus_contains(corpus, "id"));
    tq_corpus_free(corpus);

    tq_codec_config_free(cfg);
    tq_error_free(&err);
    tq_error_free_message(nullptr);

    return static_cast<int>(TINY_QUANT_ERROR_KIND_OK);
}
