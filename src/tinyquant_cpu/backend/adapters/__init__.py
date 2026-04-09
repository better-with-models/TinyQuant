"""TinyQuant backend adapters: concrete search system integrations."""

from tinyquant_cpu.backend.adapters.pgvector import PgvectorAdapter

__all__ = [
    "PgvectorAdapter",
]
