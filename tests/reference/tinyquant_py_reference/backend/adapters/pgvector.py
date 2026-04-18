"""Pgvector search backend: PostgreSQL + pgvector adapter."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from tinyquant_py_reference.backend.protocol import SearchResult

if TYPE_CHECKING:
    from collections.abc import Callable, Sequence

    import numpy as np
    from numpy.typing import NDArray


class PgvectorAdapter:
    """Anti-corruption layer translating TinyQuant vectors to pgvector wire format.

    Implements the SearchBackend protocol for PostgreSQL + pgvector
    deployments. All SQL uses parameterized queries — no string
    interpolation for data values.

    Args:
        connection_factory: Callable returning a psycopg connection (DIP).
        table_name: Target table for vector storage.
        dimension: Expected vector dimensionality.
    """

    def __init__(
        self,
        connection_factory: Callable[[], Any],
        table_name: str,
        dimension: int,
    ) -> None:
        """Initialize the adapter with connection factory and table config."""
        self._connection_factory = connection_factory
        self._table_name = table_name
        self._dimension = dimension

    def ingest(
        self,
        vectors: dict[str, NDArray[np.float32]],
    ) -> None:
        """Insert or upsert vectors into the pgvector table.

        Args:
            vectors: Mapping of vector_id to 1-D FP32 array.
        """
        conn = self._connection_factory()
        with conn.cursor() as cur:
            for vector_id, vector in vectors.items():
                cur.execute(
                    f"INSERT INTO {self._table_name} (id, embedding) "
                    "VALUES (%s, %s) "
                    "ON CONFLICT (id) DO UPDATE SET embedding = EXCLUDED.embedding",
                    (vector_id, vector.tolist()),
                )
        conn.commit()

    def search(
        self,
        query: NDArray[np.float32],
        top_k: int,
    ) -> list[SearchResult]:
        """Execute a pgvector cosine similarity query.

        Args:
            query: 1-D FP32 query vector.
            top_k: Maximum results to return.

        Returns:
            Up to top_k results ranked by descending cosine similarity.
        """
        conn = self._connection_factory()
        with conn.cursor() as cur:
            cur.execute(
                f"SELECT id, 1 - (embedding <=> %s::vector) AS score "
                f"FROM {self._table_name} "
                "ORDER BY embedding <=> %s::vector LIMIT %s",
                (query.tolist(), query.tolist(), top_k),
            )
            rows = cur.fetchall()
        return [SearchResult(vector_id=row[0], score=float(row[1])) for row in rows]

    def remove(
        self,
        vector_ids: Sequence[str],
    ) -> None:
        """Remove vectors from the pgvector table by ID.

        Args:
            vector_ids: IDs to remove. Unknown IDs are silently ignored.
        """
        conn = self._connection_factory()
        with conn.cursor() as cur:
            cur.execute(
                f"DELETE FROM {self._table_name} WHERE id = ANY(%s)",
                (list(vector_ids),),
            )
        conn.commit()
