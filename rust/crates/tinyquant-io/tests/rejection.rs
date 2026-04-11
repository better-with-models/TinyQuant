//! Rejection tests: malformed inputs must produce specific `IoError` variants.

use std::sync::Arc;
use tinyquant_core::codec::CompressedVector;
use tinyquant_io::compressed_vector::{from_bytes, to_bytes};
use tinyquant_io::errors::IoError;

/// Build a minimal valid serialized payload for bw=8, dim=4, no residual.
fn valid_payload() -> Vec<u8> {
    let indices = vec![0u8, 1, 2, 3];
    let cv =
        CompressedVector::new(indices.into_boxed_slice(), None, Arc::from("test"), 4, 8).unwrap();
    to_bytes(&cv)
}

#[test]
fn truncated_at_69_bytes_errors() {
    let payload = valid_payload();
    let short = &payload[..69];
    let err = from_bytes(short).unwrap_err();
    assert!(
        matches!(
            err,
            IoError::Truncated {
                needed: 70,
                got: 69
            }
        ),
        "expected Truncated{{needed:70,got:69}}, got: {err:?}"
    );
}

#[test]
fn empty_slice_errors() {
    let err = from_bytes(&[]).unwrap_err();
    assert!(
        matches!(err, IoError::Truncated { .. }),
        "expected Truncated, got: {err:?}"
    );
}

#[test]
fn unknown_version_errors() {
    let mut payload = valid_payload();
    payload[0] = 0x02; // corrupt version byte
    let err = from_bytes(&payload).unwrap_err();
    assert!(
        matches!(err, IoError::UnknownVersion { got: 0x02 }),
        "expected UnknownVersion{{got:0x02}}, got: {err:?}"
    );
}

#[test]
fn invalid_bit_width_errors() {
    let mut payload = valid_payload();
    payload[69] = 3; // bit_width=3 is not in {2,4,8}
    let err = from_bytes(&payload).unwrap_err();
    assert!(
        matches!(err, IoError::InvalidBitWidth { got: 3 }),
        "expected InvalidBitWidth{{got:3}}, got: {err:?}"
    );
}

#[test]
fn invalid_utf8_in_config_hash_errors() {
    let mut payload = valid_payload();
    payload[1] = 0xFF; // first byte of config_hash is not valid UTF-8
    let err = from_bytes(&payload).unwrap_err();
    assert!(
        matches!(err, IoError::InvalidUtf8),
        "expected InvalidUtf8, got: {err:?}"
    );
}

#[test]
fn invalid_residual_flag_errors() {
    // Build bw=8, dim=4, no residual: flag is at offset 70+4=74
    let mut payload = valid_payload();
    let flag_offset = 70 + 4; // header=70, packed=dim*bw/8=4*8/8=4
    payload[flag_offset] = 0x02; // invalid residual flag
    let err = from_bytes(&payload).unwrap_err();
    assert!(
        matches!(
            err,
            IoError::Decode(tinyquant_core::errors::CodecError::InvalidResidualFlag { got: 0x02 })
        ),
        "expected Decode(InvalidResidualFlag{{got:0x02}}), got: {err:?}"
    );
}

#[test]
fn truncated_after_flag_errors() {
    // Build bw=8, dim=4, with residual but truncate after the flag byte
    let indices = vec![0u8, 1, 2, 3];
    let residual = vec![0u8; 8]; // 2*dim
    let cv = CompressedVector::new(
        indices.into_boxed_slice(),
        Some(residual.into_boxed_slice()),
        Arc::from("test"),
        4,
        8,
    )
    .unwrap();
    let full = to_bytes(&cv);
    // Truncate to just past the flag byte (flag at offset 74, so len=76 still has 4 rlen bytes but no residual)
    let flag_offset = 70 + 4; // 74
    let truncated = &full[..flag_offset + 2]; // only 2 bytes of the 4-byte rlen field
    let err = from_bytes(truncated).unwrap_err();
    assert!(
        matches!(err, IoError::Truncated { .. }),
        "expected Truncated, got: {err:?}"
    );
}

/// Verify that `From<std::io::Error>` is implemented (compile-time check).
#[test]
fn from_io_error_compiles() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    let tq_err: IoError = io_err.into();
    assert!(
        matches!(tq_err, IoError::Io(_)),
        "expected IoError::Io variant, got: {tq_err:?}"
    );
}

/// Verify that `From<CodecError>` is implemented (compile-time check).
#[test]
fn from_codec_error_compiles() {
    let codec_err = tinyquant_core::errors::CodecError::UnsupportedBitWidth { got: 3 };
    let tq_err: IoError = codec_err.into();
    assert!(
        matches!(tq_err, IoError::Decode(_)),
        "expected IoError::Decode variant, got: {tq_err:?}"
    );
}
