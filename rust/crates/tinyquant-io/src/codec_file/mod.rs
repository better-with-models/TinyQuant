//! Level-2 TQCV corpus file container.

pub(crate) mod header;
pub mod metadata;
pub mod reader;
pub mod writer;

pub use header::CorpusFileHeader;
pub use metadata::MetadataBlob;
pub use reader::CodecFileReader;
pub use writer::CodecFileWriter;
