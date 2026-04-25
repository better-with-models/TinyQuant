//! End-to-end C ABI round-trip: train → compress → serialize → deserialize → decompress.

#![allow(unsafe_code, clippy::undocumented_unsafe_blocks)]

use tinyquant_sys::codec_abi::{
    tq_bytes_free, tq_codebook_free, tq_codebook_train, tq_codec_compress, tq_codec_config_free,
    tq_codec_config_new, tq_codec_decompress, tq_compressed_vector_free,
    tq_compressed_vector_from_bytes, tq_compressed_vector_to_bytes,
};
use tinyquant_sys::error::tq_error_free;
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
        tq_error_free(&mut err);
        assert_eq!(kind, TinyQuantErrorKind::Ok, "config_new failed");
        assert!(!config.is_null());

        // Step 2: train codebook
        let mut codebook: *mut CodebookHandle = core::ptr::null_mut();
        let kind = tq_codebook_train(
            training.as_ptr(),
            ROWS,
            DIM,
            config,
            &mut codebook,
            &mut err,
        );
        tq_error_free(&mut err);
        assert_eq!(kind, TinyQuantErrorKind::Ok, "codebook_train failed");
        assert!(!codebook.is_null());

        // Step 3: compress one vector
        let mut cv: *mut CompressedVectorHandle = core::ptr::null_mut();
        let kind = tq_codec_compress(config, codebook, vector.as_ptr(), DIM, &mut cv, &mut err);
        tq_error_free(&mut err);
        assert_eq!(kind, TinyQuantErrorKind::Ok, "compress failed");
        assert!(!cv.is_null());

        // Step 4: serialize to bytes
        let mut bytes_ptr: *mut u8 = core::ptr::null_mut();
        let mut bytes_len: usize = 0;
        let kind = tq_compressed_vector_to_bytes(cv, &mut bytes_ptr, &mut bytes_len, &mut err);
        tq_error_free(&mut err);
        assert_eq!(kind, TinyQuantErrorKind::Ok, "to_bytes failed");
        assert!(!bytes_ptr.is_null());
        assert!(bytes_len > 0, "serialized bytes must be non-empty");

        // Step 5: deserialize from bytes
        let mut cv2: *mut CompressedVectorHandle = core::ptr::null_mut();
        let kind = tq_compressed_vector_from_bytes(bytes_ptr, bytes_len, &mut cv2, &mut err);
        tq_error_free(&mut err);
        assert_eq!(kind, TinyQuantErrorKind::Ok, "from_bytes failed");
        assert!(!cv2.is_null());

        // Step 6: decompress and assert reconstruction quality
        let mut output = vec![0f32; DIM];
        let kind = tq_codec_decompress(config, codebook, cv2, output.as_mut_ptr(), DIM, &mut err);
        tq_error_free(&mut err);
        assert_eq!(kind, TinyQuantErrorKind::Ok, "decompress failed");

        // All decompressed values must be finite.
        assert!(
            output.iter().all(|x| x.is_finite()),
            "all decompressed values must be finite"
        );

        // Reconstruction quality: mean squared error must be below 0.5 for
        // 4-bit quantization over a well-distributed training corpus.  A stub
        // that fills the output with a constant (e.g. 1.0) would fail this
        // gate: MSE(xorshift-in-[0,1], 1.0) ≈ 0.25 for half the values and
        // up to 1.0 for the rest, so the average greatly exceeds 0.5.
        let mse: f32 = vector
            .iter()
            .zip(output.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
            / DIM as f32;
        assert!(
            mse < 0.5,
            "MSE {mse:.4} too high — decompress does not invert compress"
        );

        // Cleanup (all must be called — reverse allocation order)
        tq_bytes_free(bytes_ptr, bytes_len);
        tq_compressed_vector_free(cv2);
        tq_compressed_vector_free(cv);
        tq_codebook_free(codebook);
        tq_codec_config_free(config);
    }
}
