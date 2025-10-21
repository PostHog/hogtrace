"""
HogTrace - DTrace-inspired instrumentation language for Python web applications.

Usage:
    import hogtrace

    # Parse HogTrace code
    program = hogtrace.parse(code)

    # Access probes
    for probe in program.probes:
        print(probe.spec.module_function)
        print(probe.predicate)
        for action in probe.actions:
            print(action)

    # Parse from file
    program = hogtrace.parse_file("traces.hogtrace")
"""

import sys
from pathlib import Path
from antlr4 import InputStream, CommonTokenStream

# Add generated/ to path so we can import the ANTLR-generated files
_generated_dir = Path(__file__).parent.parent / "generated"
if str(_generated_dir) not in sys.path:
    sys.path.insert(0, str(_generated_dir))

from HogTraceLexer import HogTraceLexer
from HogTraceParser import HogTraceParser
from hogtrace.builder import ASTBuilder
from hogtrace.ast import Program

__version__ = "0.1.0"

__all__ = [
    # Parsing
    "parse",
    "parse_file",
    "Program",
    # VM
    "ProbeExecutor",
    "ProgramExecutor",
    "RequestLocalStore",
    "RequestContext",
    # Limits
    "HogTraceLimits",
    "DEFAULT_LIMITS",
    "STRICT_LIMITS",
    "RELAXED_LIMITS",
    # Logging
    "logger",
    "set_log_level",
    # Errors
    "ParseError",
    "EvaluationError",
    "TimeoutError",
    "CaptureSizeError",
    # Serialization
    "program_to_json",
    "program_from_json",
    "serialize_program",
    "deserialize_program",
]

# Import VM components for convenience
from hogtrace.vm import ProbeExecutor, ProgramExecutor
from hogtrace.request_store import RequestLocalStore, RequestContext
from hogtrace.limits import HogTraceLimits, DEFAULT_LIMITS, STRICT_LIMITS, RELAXED_LIMITS
from hogtrace import logger
from hogtrace.logger import set_log_level
from hogtrace.errors import EvaluationError, TimeoutError, CaptureSizeError
from hogtrace.serialization import (
    program_to_json, program_from_json,
    serialize_program, deserialize_program
)


class ParseError(Exception):
    """Error parsing HogTrace code"""
    pass


def parse(code: str) -> Program:
    """
    Parse HogTrace code and return a Program object.

    Args:
        code: HogTrace source code string

    Returns:
        Program object containing parsed probes

    Raises:
        ParseError: If the code has syntax errors

    Example:
        >>> program = hogtrace.parse('''
        ... fn:myapp.users.create:entry
        ... { capture(args); }
        ... ''')
        >>> len(program.probes)
        1
    """
    try:
        # Create input stream
        input_stream = InputStream(code)

        # Lexer
        lexer = HogTraceLexer(input_stream)
        stream = CommonTokenStream(lexer)

        # Parser
        parser = HogTraceParser(stream)

        # Custom error handling
        parser.removeErrorListeners()
        error_listener = _ErrorListener()
        parser.addErrorListener(error_listener)

        # Parse
        tree = parser.program()

        # Check for errors
        if error_listener.errors:
            error_msg = "\n".join(error_listener.errors)
            raise ParseError(f"Syntax errors:\n{error_msg}")

        # Build AST
        builder = ASTBuilder()
        program = builder.build(tree)

        return program

    except ParseError:
        raise
    except Exception as e:
        raise ParseError(f"Failed to parse HogTrace code: {e}") from e


def parse_file(file_path: str | Path) -> Program:
    """
    Parse a HogTrace file and return a Program object.

    Args:
        file_path: Path to .hogtrace file

    Returns:
        Program object containing parsed probes

    Raises:
        ParseError: If the code has syntax errors
        FileNotFoundError: If the file doesn't exist

    Example:
        >>> program = hogtrace.parse_file("traces.hogtrace")
        >>> for probe in program:
        ...     print(probe.spec)
    """
    path = Path(file_path)

    if not path.exists():
        raise FileNotFoundError(f"File not found: {file_path}")

    with open(path, 'r') as f:
        code = f.read()

    return parse(code)


# Custom error listener for better error messages
from antlr4.error.ErrorListener import ErrorListener


class _ErrorListener(ErrorListener):
    """Collects syntax errors during parsing"""

    def __init__(self):
        super().__init__()
        self.errors = []

    def syntaxError(self, recognizer, offendingSymbol, line, column, msg, e):
        self.errors.append(f"Line {line}:{column} - {msg}")
