"""
Type stubs for the HogTrace Rust VM extension module.

This module provides Python bindings to the Rust-based HogTrace virtual machine.
"""

from typing import List, Optional, Dict, Any, TYPE_CHECKING, Union
from types import FrameType

if TYPE_CHECKING:
    from hogtrace.request_store import RequestLocalStore, ProgramStore

    # Type alias for store parameter - accepts both RequestLocalStore and ProgramStore
    StoreType = Union[RequestLocalStore, ProgramStore]

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

class ProgramBytecode:
    """
    The actual bytecode
    """

    @property
    def probes(self) -> List[Probe]: ...
    @property
    def bytecode_version(self) -> int: ...

class Program:
    """A compiled HogTrace program.

    Contains bytecode for all probes and a shared constant pool.
    """
    @property
    def id(self) -> str: ...
    @property
    def probes(self) -> List[Probe]:
        """Get the list of probes in this program."""
        ...

    @property
    def bytecode_version(self) -> int:
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

    @property
    def program_bytecode(self) -> ProgramBytecode:
        """
        Get the program bytecode
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

def compile(source: str) -> ProgramBytecode:
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

def package(id: str, bytecode: ProgramBytecode) -> Program:
    """Package a compiled program into a full HogTrace program."""
    ...

def execute_probe(
    program: ProgramBytecode,
    probe: Probe,
    frame: FrameType,
    store: "StoreType",
    retval: Optional[Any] = None,
    exception: Optional[BaseException] = None,
) -> Optional[Dict[str, Any]]:
    """Execute a probe against a Python frame.

    Args:
        program: Program bytecode containing the probe
        probe: The probe to execute
        frame: Python frame object
        store: ProgramStore or RequestLocalStore for cross-probe variable persistence
        retval: Optional return value (for exit probes)
        exception: Optional exception (for exit probes)

    Returns:
        Dictionary of captured data, or None if predicate failed

    Example:
        >>> import sys
        >>> from hogtrace import context
        >>> program = compile("fn:test:entry { capture(arg0=args[0]); }")
        >>> probe = program.probes[0]
        >>> frame = sys._getframe()
        >>> with context.new_context():
        >>>     store = context.get_store()
        >>>     program_store = store.for_program("my-program")
        >>>     result = execute_probe(program, probe, frame, program_store)
    """
    ...

class ProbeExecutor:
    """Probe executor for executing probes against Python frames.

    Example:
        >>> from hogtrace import context
        >>> program = compile("fn:test:entry { capture(args); }")
        >>> with context.new_context():
        >>>     store = context.get_store()
        >>>     program_store = store.for_program("my-program")
        >>>     executor = ProbeExecutor(program, program.probes[0], program_store)
        >>>     result = executor.execute(frame)
    """

    def __init__(self, program: Program, probe: Probe, store: "StoreType") -> None:
        """Create a new probe executor.

        Args:
            program: The compiled program
            probe: The probe to execute
            store: ProgramStore or RequestLocalStore for cross-probe variable persistence
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
