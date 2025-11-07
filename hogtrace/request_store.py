"""
Request-scoped variable storage for HogTrace.

This module provides thread-safe storage for per-request variables ($req.* or $request.*).
Variables are isolated both per-request (via context) and per-program.
"""

from typing import Any


class ProgramStore:
    """
    Program-scoped storage for request variables.

    Each program gets its own isolated storage within a request context.
    """

    def __init__(self, program_id: str):
        self._program_id = program_id
        self._storage: dict[str, Any] = {}

    def set(self, name: str, value: Any) -> None:
        """Set a variable."""
        self._storage[name] = value

    def get(self, name: str, default: Any = None) -> Any:
        """Get a variable."""
        return self._storage.get(name, default)

    def has(self, name: str) -> bool:
        """Check if a variable exists."""
        return name in self._storage

    def delete(self, name: str) -> None:
        """Delete a variable."""
        self._storage.pop(name, None)

    def clear(self) -> None:
        """Clear all variables."""
        self._storage.clear()

    def all(self) -> dict[str, Any]:
        """Get all variables as a dict."""
        return self._storage.copy()

    def __contains__(self, name: str) -> bool:
        """Support 'name in store' syntax."""
        return self.has(name)

    def __getitem__(self, name: str) -> Any:
        """Support store[name] syntax."""
        return self._storage[name]

    def __setitem__(self, name: str, value: Any) -> None:
        """Support store[name] = value syntax."""
        self.set(name, value)

    def __repr__(self) -> str:
        return f"ProgramStore(program_id={self._program_id!r}, vars={self._storage})"


class RequestLocalStore:
    """
    Thread-safe and async-safe storage for request-scoped variables.

    Manages program-scoped stores within a request context.
    Use via context.get_store() to get the store for the current request.

    Example:
        from hogtrace import context

        # In request middleware (handled by integration)
        with context.new_context():
            store = context.get_store()
            program_store = store.for_program("program-123")

            program_store.set("user_id", 123)
            user_id = program_store.get("user_id")  # Returns 123

            # Different program in same request - isolated
            other_store = store.for_program("program-456")
            other_store.get("user_id")  # Returns None
    """

    def __init__(self):
        self._programs: dict[str, ProgramStore] = {}

    def for_program(self, program_id: str) -> ProgramStore:
        """
        Get or create a program-scoped store.

        Args:
            program_id: Unique identifier for the program

        Returns:
            ProgramStore: A scoped store for this program's variables
        """
        if program_id not in self._programs:
            self._programs[program_id] = ProgramStore(program_id)
        return self._programs[program_id]

    def clear(self) -> None:
        """Clear all program stores in this request."""
        for program_store in self._programs.values():
            program_store.clear()

    def all_programs(self) -> list[str]:
        """Get list of all program IDs that have stores."""
        return list(self._programs.keys())

    def __repr__(self) -> str:
        total_vars = sum(len(ps.all()) for ps in self._programs.values())
        return f"RequestLocalStore(programs={list(self._programs.keys())}, total_vars={total_vars})"
