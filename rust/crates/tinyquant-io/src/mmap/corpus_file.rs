//! Memory-mapped Level-2 TQCV corpus file reader.

use crate::codec_file::header::{decode_header, CorpusFileHeader};
use crate::errors::IoError;
use crate::zero_copy::view::CompressedVectorView;
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;

/// Memory-mapped reader for a Level-2 TQCV corpus file.
///
/// The entire file is memory-mapped read-only. Iteration yields zero-copy
/// [`CompressedVectorView`]s that borrow directly from the mapped region.
pub struct CorpusFileReader {
    mmap: Mmap,
    header: CorpusFileHeader,
}

impl std::fmt::Debug for CorpusFileReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CorpusFileReader")
            .field("mmap_len", &self.mmap.len())
            .field("vector_count", &self.header.vector_count)
            .field("dimension", &self.header.dimension)
            .field("bit_width", &self.header.bit_width)
            .field("body_offset", &self.header.body_offset)
            .finish()
    }
}

impl CorpusFileReader {
    /// Open and validate a TQCV corpus file at `path`.
    ///
    /// The file is memory-mapped read-only. The Level-2 header is decoded
    /// and validated immediately; the file pointer is not advanced.
    ///
    /// # Errors
    ///
    /// Returns [`IoError`] if the file cannot be opened, mapped, or if the
    /// header is invalid.
    ///
    /// # Safety
    ///
    /// Memory mapping is inherently unsafe — external modification of the
    /// file after mapping results in undefined behaviour. This is a
    /// read-only map of a finalized TQCV file; callers must not truncate
    /// or overwrite the file while a reader is alive.
    pub fn open(path: &Path) -> Result<Self, IoError> {
        let file = File::open(path)?;
        // SAFETY: The file is opened read-only. The caller guarantees the
        // file is not modified while this reader is alive.
        let mmap = unsafe { Mmap::map(&file)? };
        let header = decode_header(&mmap)?;
        Ok(Self { mmap, header })
    }

    /// Return a reference to the decoded Level-2 header.
    pub const fn header(&self) -> &CorpusFileHeader {
        &self.header
    }

    /// Return an iterator over all records in the file.
    ///
    /// The iterator yields zero-copy [`CompressedVectorView`]s that borrow
    /// from the memory-mapped region. The iterator is bounded by
    /// `header.vector_count`.
    pub fn iter(&self) -> CorpusFileIter<'_> {
        let body = self.mmap.get(self.header.body_offset..).unwrap_or_default();
        CorpusFileIter {
            remaining: body,
            count: self.header.vector_count,
            errored: false,
        }
    }
}

impl<'a> IntoIterator for &'a CorpusFileReader {
    type Item = Result<CompressedVectorView<'a>, IoError>;
    type IntoIter = CorpusFileIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over Level-2 TQCV records, yielding zero-copy views.
pub struct CorpusFileIter<'a> {
    remaining: &'a [u8],
    count: u64,
    errored: bool,
}

/// Read the 4-byte record length prefix from the front of `data`.
/// Returns `(record_len, tail_after_length)` or an error.
fn read_record_len(data: &[u8]) -> Result<(usize, &[u8]), IoError> {
    let len_bytes: [u8; 4] = data
        .get(0..4)
        .ok_or(IoError::Truncated {
            needed: 4,
            got: data.len(),
        })?
        .try_into()
        .map_err(|_| IoError::Truncated {
            needed: 4,
            got: data.len(),
        })?;
    let record_len = u32::from_le_bytes(len_bytes) as usize;
    let tail = data.get(4..).ok_or(IoError::Truncated {
        needed: 4,
        got: data.len(),
    })?;
    Ok((record_len, tail))
}

impl<'a> Iterator for CorpusFileIter<'a> {
    type Item = Result<CompressedVectorView<'a>, IoError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.errored || self.count == 0 {
            return None;
        }

        let (record_len, after_len) = match read_record_len(self.remaining) {
            Ok(v) => v,
            Err(e) => {
                self.errored = true;
                return Some(Err(e));
            }
        };

        let Some(payload) = after_len.get(..record_len) else {
            self.errored = true;
            return Some(Err(IoError::Truncated {
                needed: record_len,
                got: after_len.len(),
            }));
        };

        match CompressedVectorView::parse(payload) {
            Ok((view, _tail)) => {
                self.remaining = after_len.get(record_len..).unwrap_or_default();
                self.count -= 1;
                Some(Ok(view))
            }
            Err(e) => {
                self.errored = true;
                Some(Err(e))
            }
        }
    }
}
