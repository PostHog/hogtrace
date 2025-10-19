"""
Stack frame utilities for HogTrace VM.

Extracts variables and context from Python stack frames for probe execution.
"""

from types import FrameType
from typing import Any, Optional


class FrameContext:
    """
    Context extracted from a Python stack frame for probe execution.

    Provides access to:
    - Function arguments (args, arg0, arg1, ..., kwargs, self)
    - Local variables
    - Global variables
    - Return value (for exit probes)
    - Exception (for exit probes)
    """

    def __init__(
        self,
        frame: FrameType,
        retval: Any = None,
        exception: Optional[BaseException] = None
    ):
        self.frame = frame
        self._retval = retval
        self._exception = exception
        self._context = self._build_context()

    def _build_context(self) -> dict[str, Any]:
        """Build the execution context from the frame"""
        context = {}

        # Get locals and globals
        frame_locals = self.frame.f_locals
        frame_globals = self.frame.f_globals

        # Extract function arguments
        code = self.frame.f_code
        arg_count = code.co_argcount
        kwonly_arg_count = code.co_kwonlyargcount
        var_names = code.co_varnames

        # Positional arguments
        args_list = []
        for i in range(arg_count):
            arg_name = var_names[i]
            if arg_name in frame_locals:
                value = frame_locals[arg_name]
                args_list.append(value)
                # Also add as arg0, arg1, etc.
                context[f'arg{i}'] = value

        # Create args tuple
        context['args'] = tuple(args_list)

        # Keyword-only arguments
        kwargs_dict = {}
        for i in range(arg_count, arg_count + kwonly_arg_count):
            arg_name = var_names[i]
            if arg_name in frame_locals:
                kwargs_dict[arg_name] = frame_locals[arg_name]

        # Variable keyword arguments (**kwargs)
        if code.co_flags & 0x08:  # CO_VARKEYWORDS
            kwarg_name = var_names[arg_count + kwonly_arg_count + (1 if code.co_flags & 0x04 else 0)]
            if kwarg_name in frame_locals:
                extra_kwargs = frame_locals[kwarg_name]
                if isinstance(extra_kwargs, dict):
                    kwargs_dict.update(extra_kwargs)

        context['kwargs'] = kwargs_dict

        # self (for methods)
        if arg_count > 0 and var_names[0] == 'self':
            context['self'] = frame_locals.get('self')

        # All locals (excluding special variables)
        context['locals'] = {
            k: v for k, v in frame_locals.items()
            if not k.startswith('__')
        }

        # All globals (excluding builtins and special variables)
        context['globals'] = {
            k: v for k, v in frame_globals.items()
            if not k.startswith('__') and k != '__builtins__'
        }

        # Return value (for exit probes)
        if self._retval is not None:
            context['retval'] = self._retval

        # Exception (for exit probes)
        context['exception'] = self._exception

        return context

    def get(self, name: str, default: Any = None) -> Any:
        """
        Get a variable from the context.

        Args:
            name: Variable name
            default: Default value if not found

        Returns:
            The variable value or default
        """
        return self._context.get(name, default)

    def has(self, name: str) -> bool:
        """Check if a variable exists in the context"""
        return name in self._context

    def __contains__(self, name: str) -> bool:
        """Support 'name in context' syntax"""
        return self.has(name)

    def __getitem__(self, name: str) -> Any:
        """Support context[name] syntax"""
        return self._context[name]

    def all(self) -> dict[str, Any]:
        """Get all variables in the context"""
        return self._context.copy()

    def __repr__(self) -> str:
        keys = list(self._context.keys())
        return f"FrameContext({keys})"


def extract_args_from_frame(frame: FrameType) -> tuple:
    """
    Extract positional arguments from a stack frame.

    Args:
        frame: Python stack frame

    Returns:
        Tuple of positional arguments
    """
    code = frame.f_code
    arg_count = code.co_argcount
    var_names = code.co_varnames
    frame_locals = frame.f_locals

    args = []
    for i in range(arg_count):
        arg_name = var_names[i]
        if arg_name in frame_locals:
            args.append(frame_locals[arg_name])

    return tuple(args)


def extract_kwargs_from_frame(frame: FrameType) -> dict:
    """
    Extract keyword arguments from a stack frame.

    Args:
        frame: Python stack frame

    Returns:
        Dict of keyword arguments
    """
    code = frame.f_code
    arg_count = code.co_argcount
    kwonly_arg_count = code.co_kwonlyargcount
    var_names = code.co_varnames
    frame_locals = frame.f_locals

    kwargs = {}

    # Keyword-only arguments
    for i in range(arg_count, arg_count + kwonly_arg_count):
        arg_name = var_names[i]
        if arg_name in frame_locals:
            kwargs[arg_name] = frame_locals[arg_name]

    # Variable keyword arguments (**kwargs)
    if code.co_flags & 0x08:  # CO_VARKEYWORDS
        offset = 1 if code.co_flags & 0x04 else 0  # *args offset
        kwarg_name = var_names[arg_count + kwonly_arg_count + offset]
        if kwarg_name in frame_locals:
            extra_kwargs = frame_locals[kwarg_name]
            if isinstance(extra_kwargs, dict):
                kwargs.update(extra_kwargs)

    return kwargs
