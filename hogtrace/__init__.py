"""
HogTrace - DTrace-inspired instrumentation language for Python web applications.

Usage:
    import hogtrace

    # Compile HogTrace code
    program = hogtrace.compile('''
        fn:myapp.users.create:entry {
            capture(user_id=args[0]);
        }
    ''')

    # Execute probe against a frame
    import sys
    from hogtrace.request_store import RequestLocalStore

    probe = program.probes[0]
    frame = sys._getframe()
    store = RequestLocalStore()
    result = hogtrace.execute_probe(program, probe, frame, store)

    # Or use ProbeExecutor
    executor = hogtrace.ProbeExecutor(program, probe, store)
    result = executor.execute(frame)
"""

from __future__ import annotations

from pathlib import Path
from typing import Union

# Import Rust VM components
from hogtrace.vm import (
    compile,
    package,
    execute_probe,
    Program,
    ProgramList,
    Probe,
    ProbeSpec,
    ProbeExecutor,
    BYTECODE_VERSION,
)

from hogtrace.request_store import RequestLocalStore
from hogtrace.context import get_store, get_scope

__version__ = "0.1.0"

__all__ = [
    # Core VM (Rust)
    "compile",
    "package",
    "compile_file",
    "execute_probe",
    "ProgramList",
    "Program",
    "Probe",
    "ProbeSpec",
    "ProbeExecutor",
    "BYTECODE_VERSION",
    # Utilities
    "RequestLocalStore",
    "get_store",
    "get_scope",
    # Errors
    "CompilationError",
]


class CompilationError(Exception):
    """Error compiling HogTrace code"""

    pass


def compile_file(file_path: Union[str, Path]) -> Program:
    """
    Compile a HogTrace file and return a Program.

    Args:
        file_path: Path to .hogtrace file

    Returns:
        Program object with compiled bytecode

    Raises:
        CompilationError: If the code has syntax errors
        FileNotFoundError: If the file doesn't exist

    Example:
        >>> program = hogtrace.compile_file("traces.hogtrace")
        >>> for probe in program.probes:
        ...     print(probe.spec)
    """
    path = Path(file_path)

    if not path.exists():
        raise FileNotFoundError(f"File not found: {file_path}")

    with open(path, "r") as f:
        code = f.read()

    try:
        return compile(code)
    except ValueError as e:
        raise CompilationError(str(e)) from e
