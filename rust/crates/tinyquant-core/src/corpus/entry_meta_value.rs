//! `EntryMetaValue` — a `no_std + alloc` substitute for `serde_json::Value`.
//!
//! # Design rationale
//!
//! `serde_json::Value` allocates owned `String`/`Vec` at every node and
//! unconditionally pulls `std`. `EntryMetaValue` instead wraps payloads in
//! [`Arc`] so that clones are O(1) reference-count bumps, fitting the
//! `tinyquant-core` `no_std + alloc` posture.
//!
//! # Float equality semantics
//!
//! `PartialEq` for [`EntryMetaValue::Float`] uses **bit-exact comparison**
//! (`a.to_bits() == b.to_bits()`), not IEEE 754 semantics.  This means
//! `NaN == NaN` returns `true`, mirroring Python's `dict` key-stability
//! contract: a metadata round-trip must not drop `NaN` keys or values.
//!
//! > **Warning:** this diverges from IEEE 754.  Do not use
//! > `EntryMetaValue::Float` as a numeric comparison type; use it only for
//! > metadata storage and identity checks.

use alloc::{collections::BTreeMap, string::String, sync::Arc, vec::Vec};

/// A `no_std`-compatible JSON-like value for [`VectorEntry`](super::VectorEntry) metadata.
///
/// All heap-owning variants wrap their payload in [`Arc`] so that cloning
/// is O(1) (a reference-count bump), never O(n) (a deep copy).
///
/// # Equality
///
/// All variants use structural equality **except** `Float`, which uses
/// bit-exact comparison so that `NaN == NaN` (see module-level docs).
#[derive(Clone, Debug)]
pub enum EntryMetaValue {
    /// JSON `null`.
    Null,
    /// JSON `true` / `false`.
    Bool(bool),
    /// JSON integer, stored as `i64`.
    Int(i64),
    /// JSON number with fractional part.
    ///
    /// Equality is **bit-exact** (`f64::to_bits`), not IEEE 754.
    /// `NaN == NaN` returns `true` here; see the module docs for rationale.
    Float(f64),
    /// JSON string, reference-counted for O(1) clone.
    String(Arc<str>),
    /// Raw bytes (Python `bytes` values), reference-counted for O(1) clone.
    ///
    /// Stored as a byte slice rather than base64-encoded string to preserve
    /// Python round-trip parity.
    Bytes(Arc<[u8]>),
    /// JSON array, reference-counted for O(1) clone.
    Array(Arc<[Self]>),
    /// JSON object (key-ordered `BTreeMap`), reference-counted for O(1) clone.
    ///
    /// Keys are [`Arc<str>`] for shared ownership; ordering is deterministic
    /// (lexicographic), which helps with test fixtures. `HashMap` is not used
    /// because it requires `std`'s default hasher.
    Object(Arc<BTreeMap<Arc<str>, Self>>),
}

impl PartialEq for EntryMetaValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Int(a), Self::Int(b)) => a == b,
            // Bit-exact float comparison — NaN == NaN for Python dict-key stability.
            // This intentionally diverges from IEEE 754.
            (Self::Float(a), Self::Float(b)) => a.to_bits() == b.to_bits(),
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Bytes(a), Self::Bytes(b)) => a == b,
            (Self::Array(a), Self::Array(b)) => a == b,
            (Self::Object(a), Self::Object(b)) => a == b,
            _ => false,
        }
    }
}

/// `Eq` is safe because float equality is bit-exact (NaN == NaN by bits).
impl Eq for EntryMetaValue {}

impl EntryMetaValue {
    /// Construct a `String` variant from any `&str`, allocating a new Arc.
    #[must_use]
    pub fn string(s: &str) -> Self {
        Self::String(Arc::from(s))
    }

    /// Construct a `Bytes` variant from a byte slice.
    #[must_use]
    pub fn bytes(b: &[u8]) -> Self {
        Self::Bytes(Arc::from(b))
    }

    /// Construct an `Array` variant from a `Vec<EntryMetaValue>`.
    #[must_use]
    pub fn array(items: Vec<Self>) -> Self {
        Self::Array(Arc::from(items.into_boxed_slice()))
    }

    /// Construct an `Object` variant from a `BTreeMap`.
    #[must_use]
    pub fn object(map: BTreeMap<Arc<str>, Self>) -> Self {
        Self::Object(Arc::new(map))
    }

    /// Returns `true` if this value is `Null`.
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Returns the inner `bool`, or `None` if this is not a `Bool` variant.
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the inner `i64`, or `None` if this is not an `Int` variant.
    #[must_use]
    pub const fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Returns the inner `f64`, or `None` if this is not a `Float` variant.
    #[must_use]
    pub const fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Returns the inner `Arc<str>`, or `None` if this is not a `String` variant.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_ref()),
            _ => None,
        }
    }

    /// Returns the inner bytes, or `None` if this is not a `Bytes` variant.
    #[must_use]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(b) => Some(b.as_ref()),
            _ => None,
        }
    }

    /// Returns the inner array slice, or `None` if this is not an `Array` variant.
    #[must_use]
    pub fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(a) => Some(a.as_ref()),
            _ => None,
        }
    }

    /// Returns the inner object map, or `None` if this is not an `Object` variant.
    #[must_use]
    pub fn as_object(&self) -> Option<&BTreeMap<Arc<str>, Self>> {
        match self {
            Self::Object(o) => Some(o.as_ref()),
            _ => None,
        }
    }
}

impl From<bool> for EntryMetaValue {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<i64> for EntryMetaValue {
    fn from(i: i64) -> Self {
        Self::Int(i)
    }
}

impl From<i32> for EntryMetaValue {
    fn from(i: i32) -> Self {
        Self::Int(i64::from(i))
    }
}

impl From<f64> for EntryMetaValue {
    fn from(f: f64) -> Self {
        Self::Float(f)
    }
}

impl From<f32> for EntryMetaValue {
    fn from(f: f32) -> Self {
        Self::Float(f64::from(f))
    }
}

impl From<String> for EntryMetaValue {
    fn from(s: String) -> Self {
        Self::String(Arc::from(s.as_str()))
    }
}

impl From<&str> for EntryMetaValue {
    fn from(s: &str) -> Self {
        Self::String(Arc::from(s))
    }
}
