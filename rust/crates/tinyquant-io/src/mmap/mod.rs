//! Memory-mapped corpus file reader (requires the `mmap` feature).

#[cfg(feature = "mmap")]
pub mod corpus_file;

#[cfg(feature = "mmap")]
pub use corpus_file::{CorpusFileIter, CorpusFileReader};
