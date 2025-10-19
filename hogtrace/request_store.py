"""
Request-scoped variable storage for HogTrace.

This module provides thread-safe storage for per-request variables ($req.* or $request.*).
"""

import threading
from typing import Any, Optional
from contextvars import ContextVar


class RequestLocalStore:
    """
    Thread-safe and async-safe storage for request-scoped variables.

    Uses contextvars for proper async support, with threading.local as fallback.
    This allows probes to set and read variables that persist across function
    calls within a single request.

    Example:
        # In request middleware
        store = RequestLocalStore()
        store.set("user_id", 123)
        store.set("start_time", time.time())

        # In a probe later in the request
        user_id = store.get("user_id")  # Returns 123
    """

    def __init__(self):
        # Use ContextVar for async-safe storage (Python 3.7+)
        # Each ContextVar holds a dict of variables for the current context
        self._context: ContextVar[dict[str, Any]] = ContextVar(
            'hogtrace_request_vars',
            default=None
        )

        # Fallback to thread-local storage for non-async contexts
        self._thread_local = threading.local()

    def _get_storage(self) -> dict[str, Any]:
        """Get the storage dict for the current context"""
        try:
            # Try to get from context var (works in async)
            storage = self._context.get()
            if storage is None:
                storage = {}
                self._context.set(storage)
            return storage
        except LookupError:
            # Fallback to thread-local
            if not hasattr(self._thread_local, 'storage'):
                self._thread_local.storage = {}
            return self._thread_local.storage

    def set(self, name: str, value: Any) -> None:
        """
        Set a request-scoped variable.

        Args:
            name: Variable name (without $req. prefix)
            value: Value to store
        """
        storage = self._get_storage()
        storage[name] = value

    def get(self, name: str, default: Any = None) -> Any:
        """
        Get a request-scoped variable.

        Args:
            name: Variable name (without $req. prefix)
            default: Default value if not found

        Returns:
            The stored value or default
        """
        storage = self._get_storage()
        return storage.get(name, default)

    def has(self, name: str) -> bool:
        """
        Check if a variable exists.

        Args:
            name: Variable name

        Returns:
            True if the variable is set
        """
        storage = self._get_storage()
        return name in storage

    def delete(self, name: str) -> None:
        """
        Delete a request-scoped variable.

        Args:
            name: Variable name
        """
        storage = self._get_storage()
        storage.pop(name, None)

    def clear(self) -> None:
        """Clear all variables in the current context"""
        storage = self._get_storage()
        storage.clear()

    def all(self) -> dict[str, Any]:
        """
        Get all variables in the current context.

        Returns:
            Dict of all stored variables
        """
        return self._get_storage().copy()

    def __contains__(self, name: str) -> bool:
        """Support 'name in store' syntax"""
        return self.has(name)

    def __getitem__(self, name: str) -> Any:
        """Support store[name] syntax"""
        storage = self._get_storage()
        return storage[name]

    def __setitem__(self, name: str, value: Any) -> None:
        """Support store[name] = value syntax"""
        self.set(name, value)

    def __repr__(self) -> str:
        storage = self._get_storage()
        return f"RequestLocalStore({storage})"


class RequestContext:
    """
    Context manager for request-scoped storage.

    Ensures variables are cleared when the request completes.

    Example:
        store = RequestLocalStore()

        with RequestContext(store):
            store.set("user_id", 123)
            # Process request...
            # Variables available here
        # Variables cleared automatically
    """

    def __init__(self, store: RequestLocalStore):
        self.store = store
        self._token = None

    def __enter__(self):
        # Create a new context with empty storage
        self._token = self.store._context.set({})
        return self.store

    def __exit__(self, exc_type, exc_val, exc_tb):
        # Clear storage
        self.store.clear()
        # Reset context
        if self._token is not None:
            try:
                self.store._context.reset(self._token)
            except:
                pass  # Ignore reset errors
        return False
