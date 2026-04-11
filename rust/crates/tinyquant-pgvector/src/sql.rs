//! Table-name validation and SQL constant strings.
//!
//! `pgvector` table names are interpolated into SQL statements.  To prevent
//! SQL injection, names are validated against an allowlist regex before use.

use once_cell::sync::Lazy;
use regex::Regex;

use crate::errors::BackendError;

/// Allowlist regex for pgvector table names.
///
/// Must start with a letter or underscore, followed by up to 62 alphanumeric
/// or underscore characters (total max = 63, matching `PostgreSQL`'s
/// `NAMEDATALEN - 1` limit).
#[allow(clippy::expect_used)] // Infallible: the regex literal is valid by construction.
pub static TABLE_NAME_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_]{0,62}$").expect("valid regex"));

/// Validate a table name against the allowlist regex.
///
/// Returns `Err(BackendError::Adapter(...))` when the name contains
/// characters outside `[A-Za-z0-9_]`, starts with a digit, or exceeds
/// 63 characters.
///
/// # Errors
///
/// Returns `Err(BackendError::Adapter(_))` if `name` does not match the
/// allowlist regex `^[A-Za-z_][A-Za-z0-9_]{0,62}$`.
pub fn validate_table_name(name: &str) -> Result<(), BackendError> {
    if TABLE_NAME_RE.is_match(name) {
        Ok(())
    } else {
        Err(BackendError::Adapter(std::sync::Arc::from(format!(
            "invalid table name: {name:?} — must match [A-Za-z_][A-Za-z0-9_]{{0,62}}"
        ))))
    }
}
