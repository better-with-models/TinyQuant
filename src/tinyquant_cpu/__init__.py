"""TinyQuant (dev-mode shim): re-exports the Phase 22.A Rust extension.

In the fat wheel produced by Phase 24.2 this module is replaced by the
`scripts/packaging/templates/__init__.py` entry point that runs the
per-arch runtime selector in `_selector.py`. For local development the
installed `tinyquant_rs._core` extension is already locked to the host
arch, so we skip the selector entirely and simply re-home the extension
under `sys.modules["tinyquant_cpu._core"]`.

See `docs/plans/rust/phase-24-python-fat-wheel-official.md`
§Shim-layer contract for the contract that both the dev shim and the
fat-wheel shim must satisfy.
"""

from __future__ import annotations

import sys

import tinyquant_rs._core as _core

# Re-register the extension under the `tinyquant_cpu._core` name so the
# per-sub-package shims (codec, corpus, backend) can resolve attributes
# off `sys.modules["tinyquant_cpu._core"]` exactly like they will in the
# fat wheel.
sys.modules["tinyquant_cpu._core"] = _core

__version__: str = _core.__version__

__all__ = ["__version__"]
