"""
Resource limits and safety configuration for HogTrace.

These limits ensure that probes cannot consume excessive resources
or impact application performance.
"""

from dataclasses import dataclass
from typing import Optional


@dataclass
class HogTraceLimits:
    """Configuration for HogTrace resource limits and safety constraints."""

    # Expression evaluation limits
    max_recursion_depth: int = 100
    """Maximum depth for recursive expression evaluation (default: 100)"""

    max_predicate_time_ms: Optional[int] = 10
    """
    Maximum time in milliseconds for predicate evaluation (default: 10ms).
    Set to None to disable timeout (not recommended in production).
    """

    # Data capture limits
    max_capture_size_bytes: int = 10_000
    """Maximum size in bytes for captured data (default: 10KB)"""

    max_capture_depth: int = 10
    """
    Maximum depth when traversing nested objects/dicts/lists for capture.
    Prevents excessive memory usage from deeply nested structures (default: 10).
    """

    max_capture_items: int = 100
    """
    Maximum number of items to capture from dicts/lists (default: 100).
    Prevents capturing huge collections.
    """

    # Rate limiting
    max_probe_fires_per_second: Optional[int] = 1000
    """
    Maximum number of times a single probe can fire per second (default: 1000).
    Set to None to disable rate limiting.
    """

    # Security
    allow_private_attributes: bool = False
    """
    Whether to allow access to private attributes (starting with _).
    Default: False for security.
    """

    allow_dunder_attributes: bool = False
    """
    Whether to allow access to dunder attributes (__xxx__).
    Default: False to prevent introspection attacks.
    """

    # Logging
    log_predicate_failures: bool = False
    """
    Whether to log when predicates evaluate to False.
    Default: False to avoid noise, but useful for debugging.
    """

    log_probe_execution: bool = False
    """
    Whether to log every probe execution.
    Default: False to avoid performance impact, enable for debugging.
    """

    def __post_init__(self):
        """Validate limits make sense."""
        if self.max_recursion_depth < 1:
            raise ValueError("max_recursion_depth must be at least 1")

        if self.max_predicate_time_ms is not None and self.max_predicate_time_ms < 1:
            raise ValueError("max_predicate_time_ms must be at least 1 or None")

        if self.max_capture_size_bytes < 100:
            raise ValueError("max_capture_size_bytes must be at least 100")

        if self.max_capture_depth < 1:
            raise ValueError("max_capture_depth must be at least 1")

        if self.max_capture_items < 1:
            raise ValueError("max_capture_items must be at least 1")

        if (
            self.max_probe_fires_per_second is not None
            and self.max_probe_fires_per_second < 1
        ):
            raise ValueError("max_probe_fires_per_second must be at least 1 or None")


# Default production-safe limits
DEFAULT_LIMITS = HogTraceLimits()

# Stricter limits for high-traffic production environments
STRICT_LIMITS = HogTraceLimits(
    max_recursion_depth=50,
    max_predicate_time_ms=5,
    max_capture_size_bytes=5_000,
    max_capture_depth=5,
    max_capture_items=50,
    max_probe_fires_per_second=500,
)

# Relaxed limits for development/testing
RELAXED_LIMITS = HogTraceLimits(
    max_recursion_depth=200,
    max_predicate_time_ms=50,
    max_capture_size_bytes=50_000,
    max_capture_depth=20,
    max_capture_items=500,
    max_probe_fires_per_second=None,
    log_predicate_failures=True,
    log_probe_execution=True,
)
