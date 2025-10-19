"""
Whitelisted built-in functions for HogTrace VM.

Only these functions can be called from HogTrace probes for security.
"""

import time
import random
from typing import Any, Callable


# Whitelist of safe built-in functions
BUILTIN_FUNCTIONS: dict[str, Callable] = {
    # Time
    'timestamp': lambda: time.time(),

    # Random
    'rand': lambda: random.random(),

    # Type conversion
    'str': str,
    'int': int,
    'float': float,
    'bool': bool,

    # Collections
    'len': len,
    'list': list,
    'dict': dict,
    'tuple': tuple,
    'set': set,

    # Math (basic)
    'abs': abs,
    'min': min,
    'max': max,
    'sum': sum,
    'round': round,

    # String operations
    'upper': lambda s: s.upper() if hasattr(s, 'upper') else str(s).upper(),
    'lower': lambda s: s.lower() if hasattr(s, 'lower') else str(s).lower(),
    'strip': lambda s: s.strip() if hasattr(s, 'strip') else str(s).strip(),

    # Type checking
    'isinstance': isinstance,
    'hasattr': hasattr,
    'getattr': getattr,
}


def call_builtin(name: str, *args, **kwargs) -> Any:
    """
    Call a whitelisted built-in function.

    Args:
        name: Function name
        *args: Positional arguments
        **kwargs: Keyword arguments

    Returns:
        Function result

    Raises:
        NameError: If function is not whitelisted
        Exception: If function call fails
    """
    if name not in BUILTIN_FUNCTIONS:
        raise NameError(f"Function '{name}' is not available in HogTrace")

    func = BUILTIN_FUNCTIONS[name]
    return func(*args, **kwargs)


def is_safe_function(name: str) -> bool:
    """
    Check if a function name is whitelisted.

    Args:
        name: Function name

    Returns:
        True if the function is safe to call
    """
    return name in BUILTIN_FUNCTIONS
