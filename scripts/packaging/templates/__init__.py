"""TinyQuant: CPU-only vector quantization codec (Rust-backed fat wheel)."""

from tinyquant_cpu._selector import load_core as _load_core

_core = _load_core()     # side effect: registers tinyquant_cpu._core

__version__ = _core.__version__

__all__ = ["__version__"]
