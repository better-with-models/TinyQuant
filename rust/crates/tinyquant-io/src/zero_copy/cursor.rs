//! Streaming iterator over a Level-1 record byte slice (non-mmap).

use crate::errors::IoError;
use crate::zero_copy::view::CompressedVectorView;

/// Iterates Level-1 records from a borrowed byte slice.
///
/// Iteration stops on the first error or when the slice is exhausted.
#[allow(dead_code)]
pub struct SliceCursor<'a> {
    remaining: &'a [u8],
    errored: bool,
}

impl<'a> SliceCursor<'a> {
    /// Create a new cursor over `data`.
    #[allow(dead_code)]
    pub const fn new(data: &'a [u8]) -> Self {
        Self {
            remaining: data,
            errored: false,
        }
    }
}

impl<'a> Iterator for SliceCursor<'a> {
    type Item = Result<CompressedVectorView<'a>, IoError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.errored || self.remaining.is_empty() {
            return None;
        }
        match CompressedVectorView::parse(self.remaining) {
            Ok((view, tail)) => {
                self.remaining = tail;
                Some(Ok(view))
            }
            Err(e) => {
                self.errored = true;
                Some(Err(e))
            }
        }
    }
}
