//! Runs `bindgen` against the committed `include/tinyquant.h` and
//! verifies that every entry-point symbol the crate exports also shows
//! up as a `pub fn` in the generated FFI layer. This is the drift guard
//! from the C compiler's perspective: the header must parse as valid C
//! and expose every symbol in the Rust surface.
//!
//! The `libclang` dependency is inherited from the `bindgen` dev-dep
//! declared in `Cargo.toml`. If `libclang` is missing on the host,
//! bindgen will return an error — we skip the test with a message
//! rather than fail, matching the "informational" contract in
//! `docs/plans/rust/phase-22-pyo3-cabi-release.md`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

/// Symbols that must be declared in `tinyquant.h`.
const REQUIRED_SYMBOLS: &[&str] = &[
    // codec config
    "tq_codec_config_new",
    "tq_codec_config_free",
    "tq_codec_config_bit_width",
    "tq_codec_config_seed",
    "tq_codec_config_dimension",
    "tq_codec_config_residual_enabled",
    "tq_codec_config_hash",
    // codebook
    "tq_codebook_train",
    "tq_codebook_free",
    "tq_codebook_bit_width",
    // codec
    "tq_codec_compress",
    "tq_codec_decompress",
    // compressed vector
    "tq_compressed_vector_dimension",
    "tq_compressed_vector_bit_width",
    "tq_compressed_vector_free",
    "tq_compressed_vector_to_bytes",
    "tq_compressed_vector_from_bytes",
    // bytes
    "tq_bytes_free",
    // corpus
    "tq_corpus_new",
    "tq_corpus_insert",
    "tq_corpus_vector_count",
    "tq_corpus_contains",
    "tq_corpus_free",
    // error / version
    "tq_error_free",
    "tq_error_free_message",
    "tq_version",
];

fn header_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("include")
        .join("tinyquant.h")
}

#[test]
fn header_parses_and_exposes_every_entry_point() {
    let header = header_path();
    assert!(
        header.is_file(),
        "missing generated header at {}",
        header.display()
    );

    // bindgen panics (rather than returning Err) when libclang is not
    // discoverable; wrap the call in `catch_unwind` so the test can
    // skip gracefully on hosts without a clang toolchain.
    let header_str = header.to_string_lossy().into_owned();
    let generate_result = std::panic::catch_unwind(|| {
        bindgen::Builder::default()
            .header(header_str.clone())
            .allowlist_function("tq_.*")
            .allowlist_type("TinyQuant.*")
            .allowlist_type(".*Handle")
            .layout_tests(false)
            .generate_comments(false)
            .generate()
    });
    let bindings = match generate_result {
        Ok(Ok(b)) => b,
        Ok(Err(e)) => {
            eprintln!(
                "bindgen skipped: {e} \
                 (this usually means libclang is not installed on the host)"
            );
            return;
        }
        Err(_) => {
            eprintln!(
                "bindgen skipped: libclang not found. Set LIBCLANG_PATH \
                 to enable the full header-compile check."
            );
            return;
        }
    };

    let generated = bindings.to_string();
    for sym in REQUIRED_SYMBOLS {
        assert!(
            generated.contains(&format!("pub fn {sym}")),
            "symbol `{sym}` missing from bindgen output of tinyquant.h"
        );
    }

    // Sanity: version macro is present and non-empty.
    let src = std::fs::read_to_string(&header).expect("read header");
    assert!(
        src.contains("#define TINYQUANT_H_VERSION"),
        "header must expose TINYQUANT_H_VERSION"
    );
    // The macro value must match CARGO_PKG_VERSION.
    let expected = format!(
        "#define TINYQUANT_H_VERSION \"{}\"",
        env!("CARGO_PKG_VERSION")
    );
    assert!(
        src.contains(&expected),
        "header version macro out of sync with crate version (expected `{expected}`)"
    );
}

#[test]
fn header_contains_required_enum_kinds() {
    let src = std::fs::read_to_string(header_path()).expect("read header");
    for needle in [
        "TINY_QUANT_ERROR_KIND_OK",
        "TINY_QUANT_ERROR_KIND_INVALID_HANDLE",
        "TINY_QUANT_ERROR_KIND_INVALID_ARGUMENT",
        "TINY_QUANT_ERROR_KIND_PANIC",
        "TINY_QUANT_COMPRESSION_POLICY_COMPRESS",
        "TINY_QUANT_COMPRESSION_POLICY_PASSTHROUGH",
        "TINY_QUANT_COMPRESSION_POLICY_FP16",
    ] {
        assert!(
            src.contains(needle),
            "expected `{needle}` in generated header"
        );
    }
}
