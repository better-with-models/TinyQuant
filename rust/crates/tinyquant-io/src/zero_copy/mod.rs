//! Zero-copy views into serialized `CompressedVector` data.

pub(crate) mod cursor;
pub mod view;

pub use view::CompressedVectorView;
