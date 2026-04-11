//! Level-2 TQCV corpus file writer.
//!
//! Protocol:
//!
//! 1. [`CodecFileWriter::create`] — open file, write TQCX header (tentative).
//! 2. [`CodecFileWriter::append`] — write one record (4-byte length prefix + Level-1 payload).
//! 3. [`CodecFileWriter::finalize`] — back-patch `vector_count` and flip magic to TQCV.

use crate::codec_file::header::{encode_header, FIXED_HEADER_SIZE, MAGIC_FINAL};
use crate::compressed_vector::to_bytes;
use crate::errors::IoError;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use tinyquant_core::codec::CompressedVector;

/// Streaming writer for a Level-2 TQCV corpus file.
///
/// The file is opened with TQCX (tentative) magic and finalized to TQCV
/// once all records have been written and the count is known.
pub struct CodecFileWriter {
    file: File,
    vector_count: u64,
    config_hash: String,
    dimension: u32,
    bit_width: u8,
    residual: bool,
}

impl CodecFileWriter {
    /// Create a new TQCV corpus file at `path` and write the tentative header.
    ///
    /// # Errors
    ///
    /// Returns [`IoError`] if the file cannot be created, or if any header
    /// field value is invalid.
    pub fn create(
        path: &Path,
        config_hash: &str,
        dimension: u32,
        bit_width: u8,
        residual: bool,
        metadata: &[u8],
    ) -> Result<Self, IoError> {
        let header = encode_header(config_hash, dimension, bit_width, residual, metadata, 0)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        file.write_all(&header)?;
        // Sync after writing the tentative header.
        file.sync_data()?;
        Ok(Self {
            file,
            vector_count: 0,
            config_hash: config_hash.to_owned(),
            dimension,
            bit_width,
            residual,
        })
    }

    /// Append one [`CompressedVector`] as a length-prefixed Level-1 record.
    ///
    /// # Errors
    ///
    /// Returns [`IoError`] if the vector cannot be serialized or written.
    pub fn append(&mut self, cv: &CompressedVector) -> Result<(), IoError> {
        let payload = to_bytes(cv);
        #[allow(clippy::cast_possible_truncation)]
        let record_len = payload.len() as u32;
        self.file.write_all(&record_len.to_le_bytes())?;
        self.file.write_all(&payload)?;
        self.vector_count += 1;
        Ok(())
    }

    /// Finalize the file: sync body, back-patch `vector_count`, flip magic to TQCV, sync again.
    ///
    /// After calling `finalize`, the writer is consumed and the file is closed.
    ///
    /// # Errors
    ///
    /// Returns [`IoError`] if any seek, write, or sync operation fails.
    pub fn finalize(mut self) -> Result<(), IoError> {
        // 1. Sync the body data so all appended records are durable.
        self.file.sync_data()?;

        // 2. Back-patch vector_count at offset 8 (u64 LE).
        self.file.seek(SeekFrom::Start(8))?;
        self.file.write_all(&self.vector_count.to_le_bytes())?;

        // 3. Sync vector_count before writing the magic flip. This ordering
        //    guarantee is critical: a crash between steps 2 and 4 must leave
        //    the file in a state the reader will reject (TQCX magic), never
        //    in a state with TQCV but an un-synced count.
        self.file.sync_data()?;

        // 4. Flip magic from TQCX to TQCV at offset 0 — the linearization
        //    point. After this write the file is visible to readers.
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(MAGIC_FINAL)?;

        // 5. Final sync — file is now readable and fully durable.
        self.file.sync_data()?;
        Ok(())
    }

    /// Number of vectors written so far.
    pub const fn vector_count(&self) -> u64 {
        self.vector_count
    }

    /// Config hash used when creating the file.
    pub fn config_hash(&self) -> &str {
        &self.config_hash
    }

    /// Dimension declared in the header.
    pub const fn dimension(&self) -> u32 {
        self.dimension
    }

    /// Bit width declared in the header.
    pub const fn bit_width(&self) -> u8 {
        self.bit_width
    }

    /// Whether the file was created with the residual flag set.
    pub const fn residual(&self) -> bool {
        self.residual
    }

    /// Compute the byte offset of the first record body in the file.
    ///
    /// This mirrors the `body_offset` formula from [`encode_header`]: the
    /// header is padded to an 8-byte boundary after the variable prefix.
    ///
    /// # Errors
    ///
    /// Returns [`IoError::InvalidHeader`] if the parameters would produce
    /// an invalid header (same constraints as [`encode_header`]).
    // Cannot be const because Err()/Ok() in const fns require Rust >= 1.83.
    #[allow(clippy::missing_const_for_fn)]
    pub fn body_offset(config_hash: &str, metadata_len: usize) -> Result<usize, IoError> {
        let hash_len = config_hash.as_bytes().len();
        if hash_len > 256 {
            return Err(IoError::InvalidHeader);
        }
        let header_end = FIXED_HEADER_SIZE + hash_len + 4 + metadata_len;
        Ok(((header_end + 7) / 8) * 8)
    }
}
