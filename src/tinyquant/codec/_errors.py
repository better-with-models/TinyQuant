"""Custom error types for the TinyQuant codec package."""


class DimensionMismatchError(ValueError):
    """Raised when a vector's length does not match the expected dimension."""


class ConfigMismatchError(ValueError):
    """Raised when a compressed vector's config hash does not match the config."""


class CodebookIncompatibleError(ValueError):
    """Raised when a codebook's bit width does not match the codec config."""
