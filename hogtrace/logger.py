"""
Structured logging for HogTrace.

Provides a centralized logger with consistent formatting and log levels.
"""

import logging
import sys
from typing import Any, Dict, Optional

# Create logger instance
logger = logging.getLogger("hogtrace")

# Default to WARNING level (production-safe)
logger.setLevel(logging.WARNING)

# Create console handler with formatting
_handler = logging.StreamHandler(sys.stderr)
_handler.setLevel(logging.DEBUG)

# Format: [LEVEL] hogtrace: message
_formatter = logging.Formatter(
    fmt="[%(levelname)s] %(name)s: %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S"
)
_handler.setFormatter(_formatter)

# Add handler if not already added
if not logger.handlers:
    logger.addHandler(_handler)


def set_log_level(level: str) -> None:
    """
    Set the log level for HogTrace.

    Args:
        level: Log level (DEBUG, INFO, WARNING, ERROR, CRITICAL)

    Example:
        >>> import hogtrace
        >>> hogtrace.logger.set_log_level("DEBUG")
    """
    numeric_level = getattr(logging, level.upper(), None)
    if not isinstance(numeric_level, int):
        raise ValueError(f"Invalid log level: {level}")
    logger.setLevel(numeric_level)


def log_probe_registered(probe_spec: str) -> None:
    """Log when a probe is registered."""
    logger.info(f"Probe registered: {probe_spec}")


def log_probe_executed(probe_spec: str, captured: Dict[str, Any]) -> None:
    """Log when a probe successfully executes."""
    logger.debug(f"Probe executed: {probe_spec}, captured: {captured}")


def log_probe_failed(probe_spec: str, error: Exception) -> None:
    """Log when a probe fails to execute."""
    logger.warning(f"Probe failed: {probe_spec}, error: {error}")


def log_predicate_failed(probe_spec: str, predicate: str) -> None:
    """Log when a predicate evaluates to False."""
    logger.debug(f"Predicate failed: {probe_spec}, predicate: {predicate}")


def log_predicate_error(probe_spec: str, error: Exception) -> None:
    """Log when a predicate evaluation raises an error."""
    logger.warning(f"Predicate error: {probe_spec}, error: {error}")


def log_capture_error(probe_spec: str, field: str, error: Exception) -> None:
    """Log when capturing a field fails."""
    logger.warning(f"Capture error: {probe_spec}, field: {field}, error: {error}")


def log_capture_size_exceeded(probe_spec: str, size_bytes: int, limit_bytes: int) -> None:
    """Log when captured data exceeds size limit."""
    logger.warning(
        f"Capture size exceeded: {probe_spec}, "
        f"size: {size_bytes} bytes, limit: {limit_bytes} bytes"
    )


def log_rate_limit_exceeded(probe_spec: str, rate: int, limit: int) -> None:
    """Log when a probe exceeds rate limit."""
    logger.warning(
        f"Rate limit exceeded: {probe_spec}, "
        f"rate: {rate}/s, limit: {limit}/s"
    )


def log_timeout(probe_spec: str, timeout_ms: int) -> None:
    """Log when a probe times out."""
    logger.warning(f"Probe timeout: {probe_spec}, limit: {timeout_ms}ms")


def log_posthog_send(event_name: str, properties: Dict[str, Any]) -> None:
    """Log when sending data to PostHog."""
    logger.debug(f"PostHog event: {event_name}, properties: {properties}")


def log_posthog_error(event_name: str, error: Exception) -> None:
    """Log when PostHog send fails."""
    logger.error(f"PostHog error: {event_name}, error: {error}")
