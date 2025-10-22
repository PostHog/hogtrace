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

from pathlib import Path

# Import Rust VM components
from hogtrace.vm import (
    compile,
    execute_probe,
    Program,
    Probe,
    ProbeSpec,
    ProbeExecutor,
    BYTECODE_VERSION,
)

# Import utilities
from hogtrace.request_store import RequestLocalStore, RequestContext

# Serialization removed - use Program.to_bytes() / Program.from_bytes() instead
# from hogtrace.serialization import (
#     program_to_json,
#     program_from_json,
#     serialize_program,
#     deserialize_program,
# )

__version__ = "0.1.0"

__all__ = [
    # Core VM (Rust)
    "compile",
    "compile_file",
    "execute_probe",
    "Program",
    "Probe",
    "ProbeSpec",
    "ProbeExecutor",
    "BYTECODE_VERSION",
    # Utilities
    "RequestLocalStore",
    "RequestContext",
    # Errors
    "CompilationError",
]


class CompilationError(Exception):
    """Error compiling HogTrace code"""

    pass


def compile_file(file_path: str | Path) -> Program:
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
