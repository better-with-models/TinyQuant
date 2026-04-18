//! Level-2 TQCV corpus file streaming reader (non-mmap).

use crate::codec_file::header::{
    decode_header, CorpusFileHeader, MAX_CONFIG_HASH_LEN, MAX_METADATA_LEN,
};
use crate::compressed_vector::from_bytes;
use crate::compressed_vector::header::HEADER_SIZE;
use crate::errors::IoError;
use std::io::{Read, Seek, SeekFrom};
use tinyquant_core::codec::CompressedVector;

/// Hard cap on a single Level-1 record size (defense in depth).
const MAX_RECORD_LEN: usize = 4 * 1024 * 1024; // 4 MiB

/// Streaming reader for a Level-2 TQCV corpus file.
///
/// Reads records sequentially without memory-mapping the file.
pub struct CodecFileReader<R: Read + Seek> {
    inner: R,
    header: CorpusFileHeader,
    records_read: u64,
}

impl<R: Read + Seek> CodecFileReader<R> {
    /// Open a TQCV corpus file from a seekable reader.
    ///
    /// Reads and validates the Level-2 header, then positions the reader
    /// at the start of the body.
    ///
    /// # Errors
    ///
    /// Returns [`IoError`] if the header is malformed, the magic is wrong,
    /// or the underlying I/O fails.
    pub fn new(mut inner: R) -> Result<Self, IoError> {
        let header = read_and_decode_header(&mut inner)?;
        let body_offset = header.body_offset;
        inner.seek(SeekFrom::Start(
            u64::try_from(body_offset).map_err(|_| IoError::InvalidHeader)?,
        ))?;
        Ok(Self {
            inner,
            header,
            records_read: 0,
        })
    }

    /// Return a reference to the decoded Level-2 header.
    pub const fn header(&self) -> &CorpusFileHeader {
        &self.header
    }

    /// Read the next [`CompressedVector`] from the file.
    ///
    /// Returns `Ok(None)` when all `vector_count` records have been read.
    ///
    /// # Errors
    ///
    /// Returns [`IoError`] if the record is malformed or I/O fails.
    pub fn next_vector(&mut self) -> Result<Option<CompressedVector>, IoError> {
        if self.records_read >= self.header.vector_count {
            return Ok(None);
        }
        let max_record_len = max_record_len_for_header(&self.header);
        let cv = read_record(&mut self.inner, max_record_len)?;
        self.records_read += 1;
        Ok(Some(cv))
    }

    /// Number of records read so far.
    pub const fn records_read(&self) -> u64 {
        self.records_read
    }
}

/// Read and validate the Level-2 header from `r`.
fn read_and_decode_header<R: Read + Seek>(r: &mut R) -> Result<CorpusFileHeader, IoError> {
    // Read the fixed 24-byte header first to find config_hash_len.
    let mut fixed = [0u8; 24];
    r.read_exact(&mut fixed)?;

    // Peek at config_hash_len (bytes 22..24)
    let chl_bytes: [u8; 2] = fixed
        .get(22..24)
        .ok_or(IoError::Truncated {
            needed: 24,
            got: fixed.len(),
        })?
        .try_into()
        .map_err(|_| IoError::InvalidHeader)?;
    let config_hash_len = u16::from_le_bytes(chl_bytes) as usize;
    if config_hash_len > MAX_CONFIG_HASH_LEN {
        return Err(IoError::InvalidHeader);
    }

    // Read config_hash + 4-byte metadata_len
    let mut var_prefix = vec![0u8; config_hash_len + 4];
    r.read_exact(&mut var_prefix)?;

    // Peek at metadata_len
    let ml_bytes: [u8; 4] = var_prefix
        .get(config_hash_len..config_hash_len + 4)
        .ok_or(IoError::Truncated {
            needed: config_hash_len + 4,
            got: var_prefix.len(),
        })?
        .try_into()
        .map_err(|_| IoError::InvalidHeader)?;
    let metadata_len = u32::from_le_bytes(ml_bytes) as usize;
    if metadata_len > MAX_METADATA_LEN {
        return Err(IoError::InvalidHeader);
    }

    // Read metadata + alignment padding
    let header_end = 24_usize
        .checked_add(config_hash_len)
        .and_then(|n| n.checked_add(4))
        .and_then(|n| n.checked_add(metadata_len))
        .ok_or(IoError::InvalidHeader)?;
    let body_offset = ((header_end + 7) / 8) * 8;
    let remaining_header = body_offset
        .checked_sub(24 + config_hash_len + 4)
        .ok_or(IoError::InvalidHeader)?;
    let mut rest = vec![0u8; remaining_header];
    r.read_exact(&mut rest)?;

    // Reassemble for decode_header
    let mut full = Vec::with_capacity(body_offset);
    full.extend_from_slice(&fixed);
    full.extend_from_slice(&var_prefix);
    full.extend_from_slice(&rest);

    decode_header(&full)
}

/// Compute the maximum legal Level-1 record size from validated header fields.
///
/// Derived from dimension, `bit_width`, and residual flag so a crafted corpus
/// cannot trigger a record-sized allocation before `from_bytes` validates the
/// payload.  `MAX_RECORD_LEN` is applied as a hard cap regardless.
fn max_record_len_for_header(header: &CorpusFileHeader) -> usize {
    let dim = header.dimension as usize;
    let bw = header.bit_width as usize;
    // packed indices: ceil(dim * bw / 8)
    let packed = dim.saturating_mul(bw).saturating_add(7) / 8;
    // optional residual: 4-byte length prefix + 2 bytes/element (fp16)
    let residual = if header.residual {
        4_usize.saturating_add(dim.saturating_mul(2))
    } else {
        0
    };
    HEADER_SIZE
        .saturating_add(packed)
        .saturating_add(1) // residual flag byte
        .saturating_add(residual)
        .min(MAX_RECORD_LEN)
}

/// Read one length-prefixed Level-1 record from `r`.
fn read_record<R: Read>(r: &mut R, max_record_len: usize) -> Result<CompressedVector, IoError> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let record_len = u32::from_le_bytes(len_buf) as usize;
    if record_len > max_record_len {
        return Err(IoError::InvalidHeader);
    }

    let mut payload = vec![0u8; record_len];
    r.read_exact(&mut payload)?;

    from_bytes(&payload)
}
