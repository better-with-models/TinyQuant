//! Rejection tests: malformed inputs must produce specific `IoError` variants.

use std::sync::Arc;
use tinyquant_core::codec::CompressedVector;
use tinyquant_io::codec_file::reader::CodecFileReader;
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

/// Craft a minimal Level-2 header byte stream whose metadata_len field exceeds
/// the decode-time cap. The streaming reader must reject before allocating.
#[test]
fn oversized_metadata_len_in_codec_file_header_rejected() {
    // Fixed 24-byte header: magic=TQCV, version=0x01, reserved=[0;3],
    // vector_count=1, dimension=4, bit_width=8, residual=0, config_hash_len=4
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"TQCV"); // magic
    buf.push(0x01); // version
    buf.extend_from_slice(&[0u8; 3]); // reserved
    buf.extend_from_slice(&1u64.to_le_bytes()); // vector_count
    buf.extend_from_slice(&4u32.to_le_bytes()); // dimension=4
    buf.push(8u8); // bit_width=8
    buf.push(0u8); // residual=false
    buf.extend_from_slice(&4u16.to_le_bytes()); // config_hash_len=4
                                                // 4-byte config hash
    buf.extend_from_slice(b"test");
    // metadata_len = MAX_METADATA_LEN + 1 = 16 MiB + 1
    let bad_len: u32 = (16 * 1024 * 1024 + 1) as u32;
    buf.extend_from_slice(&bad_len.to_le_bytes());
    // Stream ends here — no metadata bytes present

    let cursor = std::io::Cursor::new(buf);
    match CodecFileReader::new(cursor) {
        Err(IoError::InvalidHeader) => {}
        Err(e) => panic!("expected InvalidHeader for oversized metadata_len, got: {e:?}"),
        Ok(_) => panic!("expected Err for oversized metadata_len, got Ok"),
    }
}

/// Craft a valid Level-2 header followed by a record_len field that exceeds the
/// derived per-header maximum. The reader must reject before allocating the record.
#[test]
fn oversized_record_len_in_codec_file_rejected() {
    // Construct a minimal but complete Level-2 header:
    // config_hash_len=0, metadata_len=0 → header_end=28, body_offset=32
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"TQCV"); // magic
    buf.push(0x01); // version
    buf.extend_from_slice(&[0u8; 3]); // reserved
    buf.extend_from_slice(&1u64.to_le_bytes()); // vector_count=1 (needed so next_vector doesn't short-circuit)
    buf.extend_from_slice(&4u32.to_le_bytes()); // dimension=4
    buf.push(8u8); // bit_width=8
    buf.push(0u8); // residual=false
    buf.extend_from_slice(&0u16.to_le_bytes()); // config_hash_len=0
                                                // metadata_len field = 0
    buf.extend_from_slice(&0u32.to_le_bytes());
    // padding to reach body_offset=32 (header_end=28, needs 4 bytes of padding)
    buf.extend_from_slice(&[0u8; 4]);
    // Now at body_offset=32 — append an oversized record_len
    // MAX_RECORD_LEN = 4 MiB; use 4 MiB + 1
    let bad_record_len: u32 = 4 * 1024 * 1024 + 1;
    buf.extend_from_slice(&bad_record_len.to_le_bytes());
    // Stream ends here (no payload bytes)

    let cursor = std::io::Cursor::new(buf);
    let mut reader = CodecFileReader::new(cursor).unwrap();
    let err = reader.next_vector().unwrap_err();
    assert!(
        matches!(err, IoError::InvalidHeader),
        "expected InvalidHeader for oversized record_len, got: {err:?}"
    );
}

/// Verify that an extreme dimension value in the Level-1 header does not
/// cause integer overflow or an unconditional panic in `from_bytes`.
#[test]
fn extreme_dimension_does_not_panic_in_from_bytes() {
    use tinyquant_io::compressed_vector::from_bytes;
    // Construct a 70-byte Level-1 header with dimension = u32::MAX and bit_width = 8.
    // The packed_len computation must not overflow; the Truncated check must fire.
    let mut buf = vec![0u8; 70];
    buf[0] = 0x01; // FORMAT_VERSION
                   // config_hash bytes 1..65 remain zero
                   // dimension at bytes 65..69 = u32::MAX
    buf[65..69].copy_from_slice(&u32::MAX.to_le_bytes());
    buf[69] = 8; // bit_width = 8
                 // from_bytes must return an error (not panic)
    let result = from_bytes(&buf);
    assert!(
        result.is_err(),
        "expected error for extreme dimension, got success"
    );
}
