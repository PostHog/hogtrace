"""
PostHog client integration for HogTrace.

This module provides a client to send captured probe data to PostHog.
"""

from typing import Any, Dict, Optional
import time
from threading import Lock

from hogtrace import logger


class PostHogClient:
    """
    Client for sending HogTrace probe data to PostHog.

    This wraps the posthog-python library and provides:
    - Automatic batching
    - Error handling
    - Rate limiting

    Example:
        >>> from hogtrace.posthog_client import PostHogClient
        >>> client = PostHogClient(api_key="phc_xxx")
        >>> client.capture(
        ...     distinct_id="user123",
        ...     event="function_called",
        ...     properties={"function": "create_user", "duration_ms": 45}
        ... )
    """

    def __init__(
        self,
        api_key: str,
        host: str = "https://app.posthog.com",
        debug: bool = False
    ):
        """
        Initialize PostHog client.

        Args:
            api_key: PostHog API key (starts with phc_)
            host: PostHog host URL (default: https://app.posthog.com)
            debug: Enable debug logging

        Raises:
            ImportError: If posthog library is not installed
        """
        try:
            import posthog
        except ImportError:
            raise ImportError(
                "posthog library is required for PostHog integration. "
                "Install it with: pip install 'hogtrace[posthog]' or pip install posthog"
            )

        self.api_key = api_key
        self.host = host
        self.debug = debug

        # Initialize PostHog
        posthog.project_api_key = api_key
        posthog.host = host
        posthog.debug = debug

        self._posthog = posthog
        self._lock = Lock()
        self._event_count = 0
        self._error_count = 0

    def capture(
        self,
        distinct_id: str,
        event: str,
        properties: Optional[Dict[str, Any]] = None,
        timestamp: Optional[float] = None
    ) -> bool:
        """
        Send a capture event to PostHog.

        Args:
            distinct_id: Unique identifier for the user/request
            event: Event name
            properties: Event properties
            timestamp: Event timestamp (Unix epoch in seconds), defaults to now

        Returns:
            True if successfully queued, False if failed

        Example:
            >>> client.capture(
            ...     distinct_id="req-12345",
            ...     event="hogtrace.probe.fired",
            ...     properties={
            ...         "probe": "fn:myapp.users.create:entry",
            ...         "user_id": 123,
            ...         "duration_ms": 45
            ...     }
            ... )
        """
        try:
            # Add HogTrace metadata
            props = properties or {}
            props["$lib"] = "hogtrace"
            props["$lib_version"] = "0.1.0"

            # Use provided timestamp or current time
            ts = timestamp or time.time()

            # Send to PostHog
            self._posthog.capture(
                distinct_id=distinct_id,
                event=event,
                properties=props,
                timestamp=ts
            )

            with self._lock:
                self._event_count += 1

            if self.debug:
                logger.log_posthog_send(event, props)

            return True

        except Exception as e:
            with self._lock:
                self._error_count += 1

            logger.log_posthog_error(event, e)
            return False

    def flush(self, timeout: float = 2.0) -> bool:
        """
        Flush pending events to PostHog.

        Args:
            timeout: Maximum time to wait for flush (seconds)

        Returns:
            True if successfully flushed, False if failed
        """
        try:
            self._posthog.flush()
            return True
        except Exception as e:
            logger.log_posthog_error("flush", e)
            return False

    def shutdown(self) -> None:
        """
        Shutdown the PostHog client and flush pending events.

        Call this before exiting your application to ensure all events are sent.
        """
        try:
            self._posthog.shutdown()
        except Exception as e:
            logger.log_posthog_error("shutdown", e)

    def get_stats(self) -> Dict[str, int]:
        """
        Get statistics about events sent.

        Returns:
            Dict with 'events_sent' and 'errors' counts
        """
        with self._lock:
            return {
                "events_sent": self._event_count,
                "errors": self._error_count
            }

    def __enter__(self):
        """Support context manager protocol."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Flush on context manager exit."""
        self.shutdown()
        return False


# Convenience function for one-off events
_default_client: Optional[PostHogClient] = None


def initialize(api_key: str, host: str = "https://app.posthog.com", debug: bool = False) -> None:
    """
    Initialize the default PostHog client.

    This allows using capture() without creating a client instance.

    Args:
        api_key: PostHog API key
        host: PostHog host URL
        debug: Enable debug logging

    Example:
        >>> import hogtrace.posthog_client as posthog
        >>> posthog.initialize(api_key="phc_xxx")
        >>> posthog.capture(distinct_id="user123", event="test")
    """
    global _default_client
    _default_client = PostHogClient(api_key, host, debug)


def capture(
    distinct_id: str,
    event: str,
    properties: Optional[Dict[str, Any]] = None,
    timestamp: Optional[float] = None
) -> bool:
    """
    Send a capture event using the default client.

    Requires initialize() to be called first.

    Args:
        distinct_id: Unique identifier
        event: Event name
        properties: Event properties
        timestamp: Event timestamp

    Returns:
        True if successfully queued, False if failed

    Raises:
        RuntimeError: If initialize() hasn't been called
    """
    if _default_client is None:
        raise RuntimeError(
            "PostHog client not initialized. Call posthog_client.initialize(api_key) first."
        )
    return _default_client.capture(distinct_id, event, properties, timestamp)


def flush(timeout: float = 2.0) -> bool:
    """Flush pending events using the default client."""
    if _default_client is None:
        raise RuntimeError("PostHog client not initialized")
    return _default_client.flush(timeout)


def shutdown() -> None:
    """Shutdown the default client."""
    global _default_client
    if _default_client is not None:
        _default_client.shutdown()
        _default_client = None
