//! Opaque metadata blob view.

/// Opaque metadata bytes from a Level-2 TQCV header.
pub struct MetadataBlob<'a> {
    bytes: &'a [u8],
}

impl<'a> MetadataBlob<'a> {
    /// Wrap a slice as a metadata blob.
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    /// Return the raw bytes.
    pub const fn as_bytes(&self) -> &[u8] {
        self.bytes
    }
}
