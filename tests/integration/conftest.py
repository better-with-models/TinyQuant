"""Shared fixtures for integration tests — pgvector container provisioning."""

from __future__ import annotations

import os
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from collections.abc import Generator

_ENV_DSN = os.environ.get("PGVECTOR_TEST_DSN", "")


def _try_start_container() -> tuple[Any, str] | None:
    """Start a pgvector Docker container via testcontainers.

    Returns ``(container, dsn)`` on success, or ``None`` if
    testcontainers is not installed or Docker is unavailable.
    """
    try:
        from testcontainers.postgres import PostgresContainer  # type: ignore[import-not-found]  # noqa: I001
    except ImportError:
        return None

    try:
        container = PostgresContainer(
            image="pgvector/pgvector:pg17",
            username="test",
            password="test",  # noqa: S106 — test-only fixture credentials
            dbname="tinyquant_test",
            driver="psycopg",
        )
        container.start()
        host = container.get_container_host_ip()
        port = container.get_exposed_port(5432)
        dsn = f"postgresql://test:test@{host}:{port}/tinyquant_test"
        return container, dsn
    except Exception:  # Docker not running, permission error, etc.
        return None


@pytest.fixture(scope="session")
def pgvector_dsn() -> Generator[str, None, None]:
    """Provide a PostgreSQL+pgvector DSN for integration tests.

    Resolution order:

    1. ``PGVECTOR_TEST_DSN`` environment variable (CI / manual override).
    2. testcontainers Docker container (local dev with Docker Desktop).
    3. Skip if neither is available.
    """
    if _ENV_DSN:
        yield _ENV_DSN
        return

    result = _try_start_container()
    if result is None:
        pytest.skip(
            "pgvector tests require PGVECTOR_TEST_DSN or Docker (testcontainers)",
        )

    container, dsn = result
    try:
        yield dsn
    finally:
        container.stop()
