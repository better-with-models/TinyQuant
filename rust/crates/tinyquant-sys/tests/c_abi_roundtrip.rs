//! End-to-end C ABI round-trip: train → compress → serialize → deserialize → decompress.

#![allow(unsafe_code, clippy::undocumented_unsafe_blocks)]

use tinyquant_sys::codec_abi::{
    tq_bytes_free, tq_codebook_free, tq_codebook_train, tq_codec_compress, tq_codec_config_free,
    tq_codec_config_new, tq_codec_decompress, tq_compressed_vector_free,
    tq_compressed_vector_from_bytes, tq_compressed_vector_to_bytes,
};
use tinyquant_sys::{
    CodebookHandle, CodecConfigHandle, CompressedVectorHandle, TinyQuantError, TinyQuantErrorKind,
};

fn empty_error() -> TinyQuantError {
    TinyQuantError {
        kind: TinyQuantErrorKind::Ok,
        message: core::ptr::null_mut(),
    }
}

#[test]
fn c_abi_compress_decompress_round_trip() {
    const DIM: usize = 64;
    const ROWS: usize = 256;

    // Training data: 256 sine-wave rows × 64 columns
    let training: Vec<f32> = (0..ROWS * DIM)
        .map(|i| (i as f32 * 0.001_f32).sin())
        .collect();

    // Test vector: deterministic xorshift32
    let mut s: u32 = 0xDEAD_BEEF;
    let vector: Vec<f32> = (0..DIM)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            s as f32 / u32::MAX as f32
        })
        .collect();

    unsafe {
        // Step 1: create config
        let mut config: *mut CodecConfigHandle = core::ptr::null_mut();
        let mut err = empty_error();
        let kind = tq_codec_config_new(4, 42, DIM as u32, false, &mut config, &mut err);
        assert_eq!(kind, TinyQuantErrorKind::Ok, "config_new failed");
        assert!(!config.is_null());

        // Step 2: train codebook
        let mut codebook: *mut CodebookHandle = core::ptr::null_mut();
        let kind =
            tq_codebook_train(training.as_ptr(), ROWS, DIM, config, &mut codebook, &mut err);
        assert_eq!(kind, TinyQuantErrorKind::Ok, "codebook_train failed");
        assert!(!codebook.is_null());

        // Step 3: compress one vector
        let mut cv: *mut CompressedVectorHandle = core::ptr::null_mut();
        let kind = tq_codec_compress(config, codebook, vector.as_ptr(), DIM, &mut cv, &mut err);
        assert_eq!(kind, TinyQuantErrorKind::Ok, "compress failed");
        assert!(!cv.is_null());

        // Step 4: serialize to bytes
        let mut bytes_ptr: *mut u8 = core::ptr::null_mut();
        let mut bytes_len: usize = 0;
        let kind = tq_compressed_vector_to_bytes(cv, &mut bytes_ptr, &mut bytes_len, &mut err);
        assert_eq!(kind, TinyQuantErrorKind::Ok, "to_bytes failed");
        assert!(!bytes_ptr.is_null());
        assert!(bytes_len > 0, "serialized bytes must be non-empty");

        // Step 5: deserialize from bytes
        let mut cv2: *mut CompressedVectorHandle = core::ptr::null_mut();
        let kind = tq_compressed_vector_from_bytes(bytes_ptr, bytes_len, &mut cv2, &mut err);
        assert_eq!(kind, TinyQuantErrorKind::Ok, "from_bytes failed");
        assert!(!cv2.is_null());

        // Step 6: decompress
        let mut output = vec![0f32; DIM];
        let kind =
            tq_codec_decompress(config, codebook, cv2, output.as_mut_ptr(), DIM, &mut err);
        assert_eq!(kind, TinyQuantErrorKind::Ok, "decompress failed");
        assert_eq!(output.len(), DIM, "output must have DIM elements");
        // Decompressed values must be finite (sanity check — we don't expect exact round-trip)
        assert!(
            output.iter().all(|x| x.is_finite()),
            "all decompressed values must be finite"
        );

        // Cleanup (all must be called)
        tq_bytes_free(bytes_ptr, bytes_len);
        tq_compressed_vector_free(cv2);
        tq_compressed_vector_free(cv);
        tq_codebook_free(codebook);
        tq_codec_config_free(config);
    }
}
