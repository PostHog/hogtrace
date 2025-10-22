"""
Type stubs for the HogTrace Rust VM extension module.

This module provides Python bindings to the Rust-based HogTrace virtual machine.
"""

from typing import List, Optional, Dict, Any, TYPE_CHECKING
from types import FrameType

if TYPE_CHECKING:
    from hogtrace.request_store import RequestLocalStore

# Module constants
BYTECODE_VERSION: int

class ProbeSpec:
    """Probe specification defining where the probe is installed."""

    @property
    def specifier(self) -> str:
        """Get the probe specifier (e.g., 'myapp.users.create')."""
        ...

    @property
    def target(self) -> str:
        """Get the probe target ('entry' or 'exit')."""
        ...

    def __repr__(self) -> str: ...

class Probe:
    """A single probe with its specification and bytecode."""

    @property
    def id(self) -> str:
        """Get the probe ID."""
        ...

    @property
    def spec(self) -> ProbeSpec:
        """Get the probe specification."""
        ...

    @property
    def predicate(self) -> bytes:
        """Get the predicate bytecode (empty if no predicate)."""
        ...

    @property
    def body(self) -> bytes:
        """Get the action body bytecode."""
        ...

    def __repr__(self) -> str: ...

class Program:
    """A compiled HogTrace program.

    Contains bytecode for all probes and a shared constant pool.
    """

    @property
    def probes(self) -> List[Probe]:
        """Get the list of probes in this program."""
        ...

    @property
    def version(self) -> int:
        """Get the bytecode format version."""
        ...

    @property
    def sampling(self) -> float:
        """Get the global sampling rate."""
        ...

    def to_bytes(self) -> bytes:
        """Serialize the program to protobuf bytes.

        Returns:
            bytes: Serialized program data

        Example:
            >>> program = parse("fn:test:entry {}")
            >>> data = program.to_bytes()
            >>> loaded = Program.from_bytes(data)
        """
        ...

    @staticmethod
    def from_bytes(data: bytes) -> Program:
        """Deserialize a program from protobuf bytes.

        Args:
            data: Serialized program data

        Returns:
            Program: Deserialized program

        Raises:
            RuntimeError: If deserialization fails
        """
        ...

    def __repr__(self) -> str: ...

def compile(source: str) -> Program:
    """Compile HogTrace source code into a Program with bytecode.

    Args:
        source: HogTrace source code

    Returns:
        Program: Compiled program with bytecode ready for execution

    Raises:
        ValueError: If compilation fails

    Example:
        >>> program = compile("fn:myapp.users.*:entry { capture(args); }")
        >>> print(len(program.probes))
        1
    """
    ...

def execute_probe(
    program: Program,
    probe: Probe,
    frame: FrameType,
    store: "RequestLocalStore",
    retval: Optional[Any] = None,
    exception: Optional[BaseException] = None,
) -> Optional[Dict[str, Any]]:
    """Execute a probe against a Python frame.

    Args:
        program: The compiled program containing the probe
        probe: The probe to execute
        frame: Python frame object
        store: RequestLocalStore for cross-probe variable persistence
        retval: Optional return value (for exit probes)
        exception: Optional exception (for exit probes)

    Returns:
        Dictionary of captured data, or None if predicate failed

    Example:
        >>> import sys
        >>> from hogtrace.request_store import RequestLocalStore
        >>> program = compile("fn:test:entry { capture(arg0=args[0]); }")
        >>> probe = program.probes[0]
        >>> frame = sys._getframe()
        >>> store = RequestLocalStore()
        >>> result = execute_probe(program, probe, frame, store)
    """
    ...

class ProbeExecutor:
    """Probe executor for executing probes against Python frames.

    Example:
        >>> from hogtrace.request_store import RequestLocalStore
        >>> program = compile("fn:test:entry { capture(args); }")
        >>> store = RequestLocalStore()
        >>> executor = ProbeExecutor(program, program.probes[0], store)
        >>> result = executor.execute(frame)
    """

    def __init__(self, program: Program, probe: Probe, store: "RequestLocalStore") -> None:
        """Create a new probe executor.

        Args:
            program: The compiled program
            probe: The probe to execute
            store: RequestLocalStore for cross-probe variable persistence
        """
        ...

    def execute(
        self,
        frame: FrameType,
        retval: Optional[Any] = None,
        exception: Optional[BaseException] = None,
    ) -> Optional[Dict[str, Any]]:
        """Execute the probe against a Python frame.

        Args:
            frame: Python frame object
            retval: Optional return value (for exit probes)
            exception: Optional exception (for exit probes)

        Returns:
            Dictionary of captured data, or None if predicate failed
        """
        ...

    def __repr__(self) -> str: ...
