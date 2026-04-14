"""Integration tests for PgvectorAdapter → PostgreSQL (EB-02).

Requires a running PostgreSQL instance with the pgvector extension.
The ``pgvector_dsn`` fixture (provided by ``conftest.py``) resolves
the connection string from the environment, testcontainers, or skips.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any
from unittest.mock import MagicMock

import numpy as np
import pytest
from numpy.typing import NDArray

from tinyquant_py_reference.backend.adapters.pgvector import PgvectorAdapter
from tinyquant_py_reference.backend.protocol import SearchResult

if TYPE_CHECKING:
    from collections.abc import Callable, Generator

pytestmark = [pytest.mark.pgvector]

_TABLE = "tinyquant_test_vectors"
_DIM = 8


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture(scope="session")
def pgvector_connection(pgvector_dsn: str) -> Generator[Any, None, None]:
    """Session-scoped PostgreSQL connection with test table setup/teardown."""
    import psycopg

    conn = psycopg.connect(pgvector_dsn)
    with conn.cursor() as cur:
        cur.execute("CREATE EXTENSION IF NOT EXISTS vector")
        cur.execute(f"DROP TABLE IF EXISTS {_TABLE}")
        cur.execute(
            f"CREATE TABLE {_TABLE} (id TEXT PRIMARY KEY, embedding vector({_DIM}))"
        )
    conn.commit()
    yield conn
    with conn.cursor() as cur:
        cur.execute(f"DROP TABLE IF EXISTS {_TABLE}")
    conn.commit()
    conn.close()


@pytest.fixture()
def _clean_table(pgvector_connection: Any) -> Generator[None, None, None]:
    """Truncate the test table before each test."""
    with pgvector_connection.cursor() as cur:
        cur.execute(f"TRUNCATE {_TABLE}")
    pgvector_connection.commit()
    yield


@pytest.fixture()
def connection_factory(pgvector_connection: Any) -> Callable[[], Any]:
    """Factory that always returns the session connection."""

    def factory() -> Any:
        return pgvector_connection

    return factory


@pytest.fixture()
def adapter(connection_factory: Callable[[], Any]) -> PgvectorAdapter:
    """PgvectorAdapter wired to the test table."""
    return PgvectorAdapter(
        connection_factory=connection_factory,
        table_name=_TABLE,
        dimension=_DIM,
    )


def _make_vectors(
    n: int,
    dim: int = _DIM,
    seed: int = 42,
) -> dict[str, NDArray[np.float32]]:
    """Generate n random FP32 vectors keyed by 'vec-000' .. 'vec-{n-1}'."""
    rng = np.random.default_rng(seed)
    return {
        f"vec-{i:03d}": rng.standard_normal(dim).astype(np.float32) for i in range(n)
    }


# ---------------------------------------------------------------------------
# Tests — EB-02
# ---------------------------------------------------------------------------


@pytest.mark.usefixtures("_clean_table")
class TestPgvectorAdapter:
    """EB-02: PgvectorAdapter → PostgreSQL integration tests."""

    def test_ingest_writes_to_database(
        self,
        adapter: PgvectorAdapter,
        pgvector_connection: Any,
    ) -> None:
        """Vectors appear in the target table after ingest."""
        vectors = _make_vectors(3)
        adapter.ingest(vectors)
        with pgvector_connection.cursor() as cur:
            cur.execute(f"SELECT COUNT(*) FROM {_TABLE}")
            count = cur.fetchone()[0]
        assert count == 3

    def test_search_returns_ranked_results(
        self,
        adapter: PgvectorAdapter,
    ) -> None:
        """SQL query produces ordered SearchResult objects."""
        vectors = _make_vectors(5)
        adapter.ingest(vectors)
        query = np.ones(_DIM, dtype=np.float32)
        results = adapter.search(query, top_k=3)
        assert len(results) == 3
        assert all(isinstance(r, SearchResult) for r in results)
        scores = [r.score for r in results]
        assert scores == sorted(scores, reverse=True)

    def test_dimension_preserved_through_wire_format(
        self,
        adapter: PgvectorAdapter,
        pgvector_connection: Any,
    ) -> None:
        """Vector length survives NumPy → pgvector → NumPy round trip."""
        vec = np.arange(_DIM, dtype=np.float32)
        adapter.ingest({"dim-check": vec})
        with pgvector_connection.cursor() as cur:
            cur.execute(
                f"SELECT embedding::text FROM {_TABLE} WHERE id = 'dim-check'",
            )
            raw = cur.fetchone()[0]
        # pgvector returns '[0,1,2,...,7]'; parse to count elements
        elements = raw.strip("[]").split(",")
        assert len(elements) == _DIM

    def test_remove_deletes_from_database(
        self,
        adapter: PgvectorAdapter,
        pgvector_connection: Any,
    ) -> None:
        """Removed vectors absent from subsequent queries."""
        vectors = _make_vectors(3)
        adapter.ingest(vectors)
        adapter.remove(["vec-001"])
        with pgvector_connection.cursor() as cur:
            cur.execute(f"SELECT COUNT(*) FROM {_TABLE}")
            count = cur.fetchone()[0]
        assert count == 2
        results = adapter.search(np.ones(_DIM, dtype=np.float32), top_k=10)
        result_ids = {r.vector_id for r in results}
        assert "vec-001" not in result_ids

    def test_parameterized_queries_no_injection(
        self,
        adapter: PgvectorAdapter,
        pgvector_connection: Any,
    ) -> None:
        """SQL injection attempts are safely parameterized."""
        evil_id = "'; DROP TABLE tinyquant_test_vectors; --"
        vec = np.ones(_DIM, dtype=np.float32)
        adapter.ingest({evil_id: vec})
        # Table must still exist and contain the row
        with pgvector_connection.cursor() as cur:
            cur.execute(f"SELECT COUNT(*) FROM {_TABLE}")
            count = cur.fetchone()[0]
        assert count >= 1
        with pgvector_connection.cursor() as cur:
            cur.execute(f"SELECT id FROM {_TABLE} WHERE id = %s", (evil_id,))
            row = cur.fetchone()
        assert row is not None

    def test_connection_factory_is_called(
        self,
        pgvector_connection: Any,
    ) -> None:
        """Adapter uses the injected factory, not a hardcoded connection."""
        spy = MagicMock(return_value=pgvector_connection)
        adapter = PgvectorAdapter(
            connection_factory=spy,
            table_name=_TABLE,
            dimension=_DIM,
        )
        vec = np.ones(_DIM, dtype=np.float32)
        adapter.ingest({"factory-test": vec})
        assert spy.call_count >= 1
