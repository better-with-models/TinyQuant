---
title: "Phase 5: Backend Layer — Implementation Plan"
tags:
  - plans
  - phase-5
  - backend
date-created: 2026-04-08
status: in-progress
category: implementation-plan
---

# Phase 5: Backend Layer — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the backend protocol context — `SearchBackend` protocol, `SearchResult` value object, and `BruteForceBackend` reference implementation — completing the third bounded context.

**Architecture:** The backend layer defines a `SearchBackend` protocol (structural typing via `typing.Protocol`) that accepts only FP32 vectors. `SearchResult` is a frozen dataclass value object for ranked results. `BruteForceBackend` is the reference implementation using exhaustive cosine similarity — O(n*d) per query, intended for testing and small corpora, not production scale.

**Tech Stack:** Python 3.12+, numpy >=1.26, pytest, mypy --strict, ruff

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `src/tinyquant/backend/protocol.py` | Create | `SearchResult` value object + `SearchBackend` protocol |
| `src/tinyquant/backend/brute_force.py` | Create | `BruteForceBackend` concrete implementation |
| `src/tinyquant/backend/__init__.py` | Modify | Public re-exports |
| `tests/backend/test_brute_force.py` | Create | 13 tests across ingest/search/remove/clear |

No changes to codec, corpus, or project config — all tooling is already configured.

---

## Task 1: SearchResult Value Object

**Files:**
- Create: `src/tinyquant/backend/protocol.py`
- Create: `tests/backend/test_brute_force.py`

- [ ] **Step 1: Write failing tests for SearchResult**

Create `tests/backend/test_brute_force.py` with these initial tests:

```python
"""Unit tests for BruteForceBackend and SearchResult."""

from __future__ import annotations

import pytest

from tinyquant.backend.protocol import SearchResult


# ===========================================================================
# SearchResult tests
# ===========================================================================


class TestSearchResult:
    """Tests for SearchResult construction and ordering."""

    def test_construction(self) -> None:
        """SearchResult stores vector_id and score."""
        result = SearchResult(vector_id="vec-001", score=0.95)
        assert result.vector_id == "vec-001"
        assert result.score == pytest.approx(0.95)

    def test_ordering_by_score_descending(self) -> None:
        """Sorted list ranks higher scores first."""
        r1 = SearchResult(vector_id="a", score=0.3)
        r2 = SearchResult(vector_id="b", score=0.9)
        r3 = SearchResult(vector_id="c", score=0.6)
        ranked = sorted([r1, r2, r3])
        assert [r.vector_id for r in ranked] == ["b", "c", "a"]

    def test_frozen(self) -> None:
        """SearchResult is immutable."""
        result = SearchResult(vector_id="vec-001", score=0.5)
        with pytest.raises(AttributeError):
            result.score = 0.99  # type: ignore[misc]
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `pytest tests/backend/test_brute_force.py::TestSearchResult -v`
Expected: FAIL — `ImportError: cannot import name 'SearchResult'`

- [ ] **Step 3: Implement SearchResult**

Create `src/tinyquant/backend/protocol.py`:

```python
"""Search protocol and result value object."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class SearchResult:
    """Immutable value object representing a single ranked search result.

    Attributes:
        vector_id: Identity of the matched vector.
        score: Similarity score (higher is more similar).
    """

    vector_id: str
    score: float

    def __lt__(self, other: object) -> bool:
        """Order by score descending — higher scores sort first."""
        if not isinstance(other, SearchResult):
            return NotImplemented
        return self.score > other.score
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `pytest tests/backend/test_brute_force.py::TestSearchResult -v`
Expected: 3 passed

- [ ] **Step 5: Lint and type-check**

Run: `ruff check src/tinyquant/backend/protocol.py && ruff format --check src/tinyquant/backend/protocol.py && mypy --strict src/tinyquant/backend/protocol.py`
Expected: clean

- [ ] **Step 6: Commit**

```bash
git add src/tinyquant/backend/protocol.py tests/backend/test_brute_force.py
git commit -m "feat(backend): add SearchResult frozen dataclass with descending ordering"
```

---

## Task 2: SearchBackend Protocol

**Files:**
- Modify: `src/tinyquant/backend/protocol.py`

- [ ] **Step 1: Rewrite protocol.py to add SearchBackend**

Replace `src/tinyquant/backend/protocol.py` with the complete file. Key convention: numpy and NDArray are under `TYPE_CHECKING` since they're only used in annotations (matches `corpus.py` pattern).

```python
"""Search protocol and result value object."""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Protocol

if TYPE_CHECKING:
    import numpy as np
    from numpy.typing import NDArray


@dataclass(frozen=True)
class SearchResult:
    """Immutable value object representing a single ranked search result.

    Attributes:
        vector_id: Identity of the matched vector.
        score: Similarity score (higher is more similar).
    """

    vector_id: str
    score: float

    def __lt__(self, other: object) -> bool:
        """Order by score descending — higher scores sort first."""
        if not isinstance(other, SearchResult):
            return NotImplemented
        return self.score > other.score


class SearchBackend(Protocol):
    """Contract for vector search backends.

    Backends receive FP32 vectors only — never compressed representations.
    Any class with matching method signatures satisfies this protocol
    (structural typing, no inheritance needed).
    """

    def search(
        self,
        query: NDArray[np.float32],
        top_k: int,
    ) -> list[SearchResult]:
        """Find the top_k most similar vectors to query.

        Args:
            query: 1-D FP32 array matching ingested vector dimension.
            top_k: Maximum number of results to return. Must be > 0.

        Returns:
            Up to top_k results ranked by descending similarity.
        """
        ...

    def ingest(
        self,
        vectors: dict[str, NDArray[np.float32]],
    ) -> None:
        """Accept decompressed FP32 vectors for indexing.

        Args:
            vectors: Mapping of vector_id to 1-D FP32 array.
                     All vectors must have the same dimension.
        """
        ...

    def remove(
        self,
        vector_ids: Sequence[str],
    ) -> None:
        """Remove vectors from the index by ID.

        Args:
            vector_ids: IDs to remove. Unknown IDs are silently ignored.
        """
        ...
```

This is the complete final state of `protocol.py`. The SearchResult from Task 1 is included here — Task 1 creates the file, Task 2 rewrites it to add the protocol class.

- [ ] **Step 2: Lint and type-check**

Run: `ruff check src/tinyquant/backend/protocol.py && mypy --strict src/tinyquant/backend/protocol.py`
Expected: clean

- [ ] **Step 3: Commit**

```bash
git add src/tinyquant/backend/protocol.py
git commit -m "feat(backend): add SearchBackend protocol with search/ingest/remove"
```

---

## Task 3: BruteForceBackend — Ingest

**Files:**
- Create: `src/tinyquant/backend/brute_force.py`
- Modify: `tests/backend/test_brute_force.py`

- [ ] **Step 1: Write failing ingest tests**

Add to `tests/backend/test_brute_force.py`:

```python
import numpy as np
from numpy.typing import NDArray

from tinyquant.backend.brute_force import BruteForceBackend


# ===========================================================================
# Helpers
# ===========================================================================


def _make_vectors(
    n: int,
    dim: int = 8,
    seed: int = 42,
) -> dict[str, NDArray[np.float32]]:
    """Generate n random FP32 vectors keyed by 'vec-000' .. 'vec-{n-1}'."""
    rng = np.random.default_rng(seed)
    return {
        f"vec-{i:03d}": rng.standard_normal(dim).astype(np.float32)
        for i in range(n)
    }


# ===========================================================================
# Ingest tests
# ===========================================================================


class TestIngest:
    """Tests for BruteForceBackend.ingest."""

    def test_ingest_stores_vectors(self) -> None:
        """Count increases after ingestion."""
        backend = BruteForceBackend()
        backend.ingest(_make_vectors(5))
        assert backend.count == 5

    def test_ingest_duplicate_id_overwrites(self) -> None:
        """Second ingest with same ID replaces the vector, count unchanged."""
        backend = BruteForceBackend()
        vecs = _make_vectors(3)
        backend.ingest(vecs)
        # Re-ingest with one overlapping key
        backend.ingest({"vec-000": np.ones(8, dtype=np.float32)})
        assert backend.count == 3

    def test_ingest_empty_dict_is_noop(self) -> None:
        """Ingesting empty dict causes no error and no count change."""
        backend = BruteForceBackend()
        backend.ingest({})
        assert backend.count == 0
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `pytest tests/backend/test_brute_force.py::TestIngest -v`
Expected: FAIL — `ImportError: cannot import name 'BruteForceBackend'`

- [ ] **Step 3: Implement BruteForceBackend with ingest and count**

Create `src/tinyquant/backend/brute_force.py`:

```python
"""Brute-force search backend: exhaustive cosine similarity."""

from __future__ import annotations

from collections.abc import Sequence
from typing import TYPE_CHECKING

import numpy as np

from tinyquant.backend.protocol import SearchResult

if TYPE_CHECKING:
    from numpy.typing import NDArray


class BruteForceBackend:
    """Reference SearchBackend using exhaustive cosine similarity.

    Intended for testing, calibration, and small-corpus use cases.
    Complexity: O(n * d) per search call.
    """

    def __init__(self) -> None:
        self._vectors: dict[str, NDArray[np.float32]] = {}

    @property
    def count(self) -> int:
        """Number of vectors currently stored."""
        return len(self._vectors)

    def ingest(
        self,
        vectors: dict[str, NDArray[np.float32]],
    ) -> None:
        """Add or replace vectors in the in-memory store.

        Args:
            vectors: Mapping of vector_id to 1-D FP32 array.
        """
        self._vectors.update(vectors)

    def search(
        self,
        query: NDArray[np.float32],
        top_k: int,
    ) -> list[SearchResult]:
        """Compute cosine similarity against all stored vectors.

        Args:
            query: 1-D FP32 query vector.
            top_k: Maximum results to return.

        Returns:
            Up to top_k results ranked by descending cosine similarity.
        """
        raise NotImplementedError  # implemented in Task 4

    def remove(
        self,
        vector_ids: Sequence[str],
    ) -> None:
        """Remove vectors by ID. Unknown IDs are silently ignored.

        Args:
            vector_ids: IDs to remove.
        """
        raise NotImplementedError  # implemented in Task 5

    def clear(self) -> None:
        """Remove all stored vectors."""
        raise NotImplementedError  # implemented in Task 5
```

- [ ] **Step 4: Run ingest tests to verify they pass**

Run: `pytest tests/backend/test_brute_force.py::TestIngest -v`
Expected: 3 passed

- [ ] **Step 5: Lint and type-check**

Run: `ruff check src/tinyquant/backend/brute_force.py && mypy --strict src/tinyquant/backend/brute_force.py`
Expected: clean

- [ ] **Step 6: Commit**

```bash
git add src/tinyquant/backend/brute_force.py tests/backend/test_brute_force.py
git commit -m "feat(backend): add BruteForceBackend with ingest and count"
```

---

## Task 4: BruteForceBackend — Search

**Files:**
- Modify: `src/tinyquant/backend/brute_force.py`
- Modify: `tests/backend/test_brute_force.py`

- [ ] **Step 1: Write failing search tests**

Add to `tests/backend/test_brute_force.py`:

```python
class TestSearch:
    """Tests for BruteForceBackend.search."""

    def test_search_returns_top_k_results(self) -> None:
        """Exactly top_k results when enough vectors exist."""
        backend = BruteForceBackend()
        backend.ingest(_make_vectors(10))
        query = np.ones(8, dtype=np.float32)
        results = backend.search(query, top_k=3)
        assert len(results) == 3

    def test_search_results_ranked_by_similarity(self) -> None:
        """Results are in descending score order."""
        backend = BruteForceBackend()
        backend.ingest(_make_vectors(10))
        query = np.ones(8, dtype=np.float32)
        results = backend.search(query, top_k=5)
        scores = [r.score for r in results]
        assert scores == sorted(scores, reverse=True)

    def test_search_with_fewer_vectors_than_top_k(self) -> None:
        """Returns all available vectors when fewer than top_k exist."""
        backend = BruteForceBackend()
        backend.ingest(_make_vectors(2))
        query = np.ones(8, dtype=np.float32)
        results = backend.search(query, top_k=10)
        assert len(results) == 2

    def test_search_returns_search_result_objects(self) -> None:
        """Each element in results is a SearchResult."""
        backend = BruteForceBackend()
        backend.ingest(_make_vectors(3))
        query = np.ones(8, dtype=np.float32)
        results = backend.search(query, top_k=2)
        for r in results:
            assert isinstance(r, SearchResult)

    def test_search_identical_vector_has_score_near_one(self) -> None:
        """Cosine similarity of a vector with itself is approximately 1.0."""
        vec = np.array([1.0, 2.0, 3.0, 4.0], dtype=np.float32)
        backend = BruteForceBackend()
        backend.ingest({"same": vec})
        results = backend.search(vec, top_k=1)
        assert results[0].score == pytest.approx(1.0, abs=1e-6)

    def test_search_orthogonal_vector_has_score_near_zero(self) -> None:
        """Cosine similarity of orthogonal vectors is approximately 0.0."""
        v1 = np.array([1.0, 0.0, 0.0, 0.0], dtype=np.float32)
        v2 = np.array([0.0, 1.0, 0.0, 0.0], dtype=np.float32)
        backend = BruteForceBackend()
        backend.ingest({"ortho": v2})
        results = backend.search(v1, top_k=1)
        assert results[0].score == pytest.approx(0.0, abs=1e-6)

    def test_search_empty_backend_returns_empty(self) -> None:
        """No vectors ingested yields an empty result list."""
        backend = BruteForceBackend()
        query = np.ones(4, dtype=np.float32)
        results = backend.search(query, top_k=5)
        assert results == []
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `pytest tests/backend/test_brute_force.py::TestSearch -v`
Expected: FAIL — `NotImplementedError`

- [ ] **Step 3: Implement search with cosine similarity**

Replace the `search` method in `src/tinyquant/backend/brute_force.py`:

```python
    def search(
        self,
        query: NDArray[np.float32],
        top_k: int,
    ) -> list[SearchResult]:
        """Compute cosine similarity against all stored vectors.

        Args:
            query: 1-D FP32 query vector.
            top_k: Maximum results to return.

        Returns:
            Up to top_k results ranked by descending cosine similarity.
        """
        if not self._vectors:
            return []

        query_norm = float(np.linalg.norm(query))
        scored: list[SearchResult] = []
        for vid, vec in self._vectors.items():
            vec_norm = float(np.linalg.norm(vec))
            if query_norm == 0.0 or vec_norm == 0.0:
                score = 0.0
            else:
                score = float(np.dot(query, vec) / (query_norm * vec_norm))
            scored.append(SearchResult(vector_id=vid, score=score))

        scored.sort()
        return scored[:top_k]
```

- [ ] **Step 4: Run search tests to verify they pass**

Run: `pytest tests/backend/test_brute_force.py::TestSearch -v`
Expected: 7 passed

- [ ] **Step 5: Run all backend tests**

Run: `pytest tests/backend/ -v`
Expected: 13 passed (3 SearchResult + 3 ingest + 7 search)

- [ ] **Step 6: Commit**

```bash
git add src/tinyquant/backend/brute_force.py tests/backend/test_brute_force.py
git commit -m "feat(backend): implement BruteForceBackend.search with cosine similarity"
```

---

## Task 5: BruteForceBackend — Remove and Clear

**Files:**
- Modify: `src/tinyquant/backend/brute_force.py`
- Modify: `tests/backend/test_brute_force.py`

- [ ] **Step 1: Write failing remove/clear tests**

Add to `tests/backend/test_brute_force.py`:

```python
class TestRemove:
    """Tests for BruteForceBackend.remove."""

    def test_remove_decreases_count(self) -> None:
        """Count drops after removing vectors."""
        backend = BruteForceBackend()
        backend.ingest(_make_vectors(5))
        backend.remove(["vec-000", "vec-001"])
        assert backend.count == 3

    def test_removed_vector_not_in_search_results(self) -> None:
        """Removed ID does not appear in subsequent searches."""
        backend = BruteForceBackend()
        backend.ingest(_make_vectors(5))
        backend.remove(["vec-002"])
        query = np.ones(8, dtype=np.float32)
        results = backend.search(query, top_k=10)
        result_ids = {r.vector_id for r in results}
        assert "vec-002" not in result_ids


class TestClear:
    """Tests for BruteForceBackend.clear."""

    def test_clear_empties_backend(self) -> None:
        """Count is zero after clear."""
        backend = BruteForceBackend()
        backend.ingest(_make_vectors(5))
        backend.clear()
        assert backend.count == 0
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `pytest tests/backend/test_brute_force.py::TestRemove tests/backend/test_brute_force.py::TestClear -v`
Expected: FAIL — `NotImplementedError`

- [ ] **Step 3: Implement remove and clear**

Replace the stubs in `src/tinyquant/backend/brute_force.py`:

```python
    def remove(
        self,
        vector_ids: Sequence[str],
    ) -> None:
        """Remove vectors by ID. Unknown IDs are silently ignored.

        Args:
            vector_ids: IDs to remove.
        """
        for vid in vector_ids:
            self._vectors.pop(vid, None)

    def clear(self) -> None:
        """Remove all stored vectors."""
        self._vectors.clear()
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `pytest tests/backend/test_brute_force.py::TestRemove tests/backend/test_brute_force.py::TestClear -v`
Expected: 3 passed

- [ ] **Step 5: Run all backend tests**

Run: `pytest tests/backend/ -v`
Expected: 16 passed

- [ ] **Step 6: Commit**

```bash
git add src/tinyquant/backend/brute_force.py tests/backend/test_brute_force.py
git commit -m "feat(backend): implement BruteForceBackend.remove and clear"
```

---

## Task 6: Update `__init__.py` and Re-exports

**Files:**
- Modify: `src/tinyquant/backend/__init__.py` (line 1)
- Optionally modify: `src/tinyquant/__init__.py` (line 5)

- [ ] **Step 1: Update backend `__init__.py`**

Replace the contents of `src/tinyquant/backend/__init__.py` with:

```python
"""TinyQuant backend: search protocol and adapter contracts."""

from tinyquant.backend.brute_force import BruteForceBackend
from tinyquant.backend.protocol import SearchBackend, SearchResult

__all__ = [
    "BruteForceBackend",
    "SearchBackend",
    "SearchResult",
]
```

- [ ] **Step 2: Verify imports work**

Run: `python -c "from tinyquant.backend import SearchBackend, SearchResult, BruteForceBackend; print('OK')"`
Expected: `OK`

- [ ] **Step 3: Lint and type-check the package**

Run: `ruff check src/tinyquant/backend/ && mypy --strict src/tinyquant/backend/`
Expected: clean

- [ ] **Step 4: Commit**

```bash
git add src/tinyquant/backend/__init__.py
git commit -m "feat(backend): export SearchBackend, SearchResult, BruteForceBackend"
```

---

## Task 7: Full Verification

**Files:** None modified — verification only.

- [ ] **Step 1: Lint entire project**

Run: `ruff check . && ruff format --check .`
Expected: clean

- [ ] **Step 2: Type-check entire project**

Run: `mypy --strict .`
Expected: clean (no new errors)

- [ ] **Step 3: Run backend tests with coverage**

Run: `pytest tests/backend/ -v --cov=tinyquant/backend --cov-fail-under=80`
Expected: 16 passed, coverage >= 80%

- [ ] **Step 4: Run regression tests**

Run: `pytest tests/codec/ tests/corpus/ -v`
Expected: all existing tests still pass (145 tests from phases 1-4)

- [ ] **Step 5: Run full test suite**

Run: `pytest -v`
Expected: ~161 tests passed (145 existing + 16 new)

- [ ] **Step 6: Final commit (if any format fixes were needed)**

```bash
git add -A
git commit -m "chore(backend): lint and format cleanup for Phase 5"
```

Only commit if there are staged changes. If the previous commits were already clean, skip this.
