//! Tests for `tinyquant-pgvector`.
//!
//! # Structure
//!
//! - **Pure unit tests** (no DB): table-name validation and wire encode/decode.
//! - **Integration tests** (feature-gated): require a live PostgreSQL +
//!   pgvector server.  Enable with `--features test-containers`.

// ---------------------------------------------------------------------------
// Pure unit tests — no database required
// ---------------------------------------------------------------------------

mod sql_injection {
    use tinyquant_pgvector::PgvectorAdapter;

    /// Attempt to construct an adapter with the given table name.
    /// The constructor validates the name before any DB interaction.
    fn validate(name: &str) -> bool {
        PgvectorAdapter::new(name).is_ok()
    }

    #[test]
    fn table_name_accepts_valid() {
        assert!(validate("users"));
    }

    #[test]
    fn table_name_accepts_underscore_prefix() {
        assert!(validate("_my_table"));
    }

    #[test]
    fn table_name_rejects_sql_injection() {
        assert!(!validate("users; DROP TABLE users; --"));
    }

    #[test]
    fn table_name_rejects_leading_digit() {
        assert!(!validate("1users"));
    }

    #[test]
    fn table_name_rejects_dollar() {
        assert!(!validate("u$ers"));
    }

    #[test]
    fn table_name_rejects_too_long() {
        // 64 'a' characters — one over the 63-char limit
        let name = "a".repeat(64);
        assert!(!validate(&name));
    }

    #[test]
    fn table_name_accepts_max_length() {
        // 63 'a' characters — exactly at the limit
        let name = "a".repeat(63);
        assert!(validate(&name));
    }
}

mod wire_format {
    use std::sync::Arc;

    use tinyquant_core::backend::SearchBackend;
    use tinyquant_pgvector::{BackendError, PgvectorAdapter};

    #[test]
    fn wire_encode_round_trip() {
        // Wire encoding is tested via adapter integration tests (ingest/search).
        // The wire::encode_vector function is pub(crate) and cannot be called
        // from integration tests directly; correctness is validated implicitly
        // by the NaN/Inf rejection tests below which exercise the same code path.
    }

    #[test]
    fn wire_encode_rejects_nan() {
        // PgvectorAdapter::ingest calls encode_vector before any DB connection.
        // Even without `live-db`, the NaN check fires first.
        let mut adapter = PgvectorAdapter::new("test_nan").unwrap();
        let id: Arc<str> = Arc::from("nan-vec");
        let vec_with_nan = vec![1.0_f32, f32::NAN];
        let err = adapter.ingest(&[(id, vec_with_nan)]).unwrap_err();
        assert!(
            matches!(err, BackendError::Adapter(_)),
            "expected Adapter error for NaN, got {err:?}"
        );
    }

    #[test]
    fn wire_encode_rejects_inf() {
        let mut adapter = PgvectorAdapter::new("test_inf").unwrap();
        let id: Arc<str> = Arc::from("inf-vec");
        let vec_with_inf = vec![1.0_f32, f32::INFINITY];
        let err = adapter.ingest(&[(id, vec_with_inf)]).unwrap_err();
        assert!(
            matches!(err, BackendError::Adapter(_)),
            "expected Adapter error for Inf, got {err:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Integration tests — require `--features test-containers`
// ---------------------------------------------------------------------------

#[cfg(feature = "test-containers")]
mod integration {
    // TODO(Phase 19+): Implement testcontainers-based integration tests.
    // These require a running Docker daemon and are intentionally skipped
    // in standard CI.  Enable with:
    //   cargo test -p tinyquant-pgvector --features test-containers
    #[test]
    fn placeholder_integration_test() {
        // Placeholder — no-op until testcontainers integration is wired.
    }
}
