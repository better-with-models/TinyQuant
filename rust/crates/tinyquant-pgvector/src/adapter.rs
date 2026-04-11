//! `PgvectorAdapter` — `SearchBackend` backed by a `PostgreSQL` `pgvector` table.
//!
//! # Feature flags
//!
//! The adapter is always present in the public API but only performs actual
//! database operations when the `live-db` feature is enabled.  Without
//! `live-db`, every method returns a descriptive `BackendError::Adapter`
//! indicating that the feature is required.
//!
//! Integration tests requiring a live server are additionally gated behind
//! the `test-containers` feature.

use std::sync::Arc;

use tinyquant_core::backend::{SearchBackend, SearchResult};
use tinyquant_core::types::VectorId;

use crate::errors::{adapter_err, BackendError};
use crate::sql::validate_table_name;
use crate::wire::encode_vector;

/// A search backend that stores and queries vectors via pgvector.
///
/// Compile-time availability of the postgres connection depends on the
/// `live-db` feature flag.  Without it, all methods return
/// `BackendError::Adapter("live-db feature required")`.
pub struct PgvectorAdapter {
    /// Validated table name (safe for SQL interpolation).
    table: String,
    /// Dimension of the vector column (`None` until schema is ensured).
    dim: Option<usize>,
    /// Connection factory, only instantiated under `live-db`.
    #[cfg(feature = "live-db")]
    factory: Box<dyn Fn() -> Result<postgres::Client, postgres::Error> + Send + Sync>,
}

impl PgvectorAdapter {
    /// Create a new adapter for the given table.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `table` fails the allowlist regex validation.
    #[cfg(feature = "live-db")]
    pub fn new(
        factory: impl Fn() -> Result<postgres::Client, postgres::Error> + Send + Sync + 'static,
        table: &str,
        dim: u32,
    ) -> Result<Self, BackendError> {
        let table = table.to_string();
        validate_table_name(&table)?;
        let _ = dim;
        Ok(Self {
            table,
            dim: None,
            factory: Box::new(factory),
        })
    }

    /// Create a new adapter for the given table (stub without `live-db`).
    ///
    /// Only validates the table name; all operational methods will return an
    /// error until the `live-db` feature is enabled.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `table` fails the allowlist regex validation.
    #[cfg(not(feature = "live-db"))]
    pub fn new(table: impl Into<String>) -> Result<Self, BackendError> {
        let table = table.into();
        validate_table_name(&table)?;
        Ok(Self { table, dim: None })
    }

    /// Create the `vector` extension and the vectors table if they do not
    /// exist.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the `live-db` feature is disabled, or on any
    /// `postgres::Error`.
    pub fn ensure_schema(&mut self, dim: usize) -> Result<(), BackendError> {
        #[cfg(not(feature = "live-db"))]
        {
            let _ = dim;
            return Err(adapter_err(
                "live-db feature required to connect to PostgreSQL",
            ));
        }
        #[cfg(feature = "live-db")]
        {
            use crate::errors::from_pg;
            let mut client = (self.factory)().map_err(from_pg)?;
            client
                .batch_execute("CREATE EXTENSION IF NOT EXISTS vector;")
                .map_err(from_pg)?;
            let sql = format!(
                "CREATE TABLE IF NOT EXISTS {} (id TEXT PRIMARY KEY, embedding vector({}));",
                self.table, dim
            );
            client.batch_execute(&sql).map_err(from_pg)?;
            self.dim = Some(dim);
            Ok(())
        }
    }

    /// Create an approximate nearest-neighbour index on the embedding column.
    ///
    /// `lists` controls the number of `IVFFlat` lists.  If `0`, defaults to 100.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the `live-db` feature is disabled, or on any
    /// `postgres::Error`.
    pub fn ensure_index(&self, lists: u32) -> Result<(), BackendError> {
        #[cfg(not(feature = "live-db"))]
        {
            let _ = lists;
            return Err(adapter_err(
                "live-db feature required to connect to PostgreSQL",
            ));
        }
        #[cfg(feature = "live-db")]
        {
            use crate::errors::from_pg;
            let effective_lists = if lists == 0 { 100 } else { lists };
            let mut client = (self.factory)().map_err(from_pg)?;
            let sql = format!(
                "CREATE INDEX IF NOT EXISTS {table}_embedding_idx \
                 ON {table} USING ivfflat (embedding vector_cosine_ops) \
                 WITH (lists = {lists});",
                table = self.table,
                lists = effective_lists
            );
            client.batch_execute(&sql).map_err(from_pg)
        }
    }

    /// The table name used by this adapter.
    pub fn table(&self) -> &str {
        &self.table
    }
}

impl SearchBackend for PgvectorAdapter {
    fn ingest(&mut self, vectors: &[(VectorId, Vec<f32>)]) -> Result<(), BackendError> {
        if vectors.is_empty() {
            return Ok(());
        }
        // Dimension-lock check and wire encode validation happen before
        // any DB connection so we can test them without `live-db`.
        if let Some(expected) = self.dim {
            for (_, v) in vectors {
                if v.len() != expected {
                    return Err(BackendError::Adapter(Arc::from(format!(
                        "dimension mismatch: expected {expected}, got {}",
                        v.len()
                    ))));
                }
            }
        }
        // Encode each vector — rejects NaN/Inf before any DB call.
        let encoded: Vec<(&VectorId, String)> = vectors
            .iter()
            .map(|(id, v)| encode_vector(v).map(|enc| (id, enc)))
            .collect::<Result<_, _>>()?;

        #[cfg(not(feature = "live-db"))]
        {
            let _ = encoded;
            return Err(adapter_err(
                "live-db feature required to connect to PostgreSQL",
            ));
        }
        #[cfg(feature = "live-db")]
        {
            use crate::errors::from_pg;
            let mut client = (self.factory)().map_err(from_pg)?;
            let sql = format!(
                "INSERT INTO {table} (id, embedding) VALUES ($1, $2::vector) \
                 ON CONFLICT (id) DO UPDATE SET embedding = EXCLUDED.embedding;",
                table = self.table
            );
            for (id, enc) in &encoded {
                client
                    .execute(sql.as_str(), &[&id.as_ref(), enc])
                    .map_err(from_pg)?;
            }
            Ok(())
        }
    }

    fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<SearchResult>, BackendError> {
        if top_k == 0 {
            return Err(BackendError::InvalidTopK);
        }
        let encoded = encode_vector(query)?;

        #[cfg(not(feature = "live-db"))]
        {
            let _ = encoded;
            return Err(adapter_err(
                "live-db feature required to connect to PostgreSQL",
            ));
        }
        #[cfg(feature = "live-db")]
        {
            use crate::errors::from_pg;
            let sql = format!(
                "SELECT id, 1 - (embedding <=> $1::vector) AS score \
                 FROM {table} \
                 ORDER BY embedding <=> $1::vector \
                 LIMIT $2;",
                table = self.table
            );
            let mut client = (self.factory)().map_err(from_pg)?;
            let top_k_i64 = i64::try_from(top_k).unwrap_or(i64::MAX);
            let rows = client
                .query(sql.as_str(), &[&encoded, &top_k_i64])
                .map_err(from_pg)?;
            let mut results = Vec::with_capacity(rows.len());
            for row in rows {
                let id: String = row.get(0);
                let score: f64 = row.get(1);
                #[allow(clippy::cast_possible_truncation)]
                results.push(SearchResult {
                    vector_id: Arc::from(id.as_str()),
                    score: score as f32,
                });
            }
            Ok(results)
        }
    }

    fn remove(&mut self, vector_ids: &[VectorId]) -> Result<(), BackendError> {
        if vector_ids.is_empty() {
            return Ok(());
        }
        #[cfg(not(feature = "live-db"))]
        {
            return Err(adapter_err(
                "live-db feature required to connect to PostgreSQL",
            ));
        }
        #[cfg(feature = "live-db")]
        {
            use crate::errors::from_pg;
            let mut client = (self.factory)().map_err(from_pg)?;
            let sql = format!("DELETE FROM {table} WHERE id = $1;", table = self.table);
            for id in vector_ids {
                client
                    .execute(sql.as_str(), &[&id.as_ref()])
                    .map_err(from_pg)?;
            }
            Ok(())
        }
    }

    fn len(&self) -> usize {
        #[cfg(not(feature = "live-db"))]
        {
            0
        }
        #[cfg(feature = "live-db")]
        {
            let Ok(mut client) = (self.factory)() else {
                return 0;
            };
            let sql = format!("SELECT COUNT(*) FROM {};", self.table);
            let Ok(row) = client.query_one(sql.as_str(), &[]) else {
                return 0;
            };
            let count: i64 = row.get(0);
            usize::try_from(count).unwrap_or(0)
        }
    }

    fn dim(&self) -> Option<usize> {
        self.dim
    }
}
