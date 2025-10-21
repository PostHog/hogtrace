"""
Expression evaluator for HogTrace VM.

Safely evaluates HogTrace expressions using frame context and request store.
"""

import operator
import signal
import sys
from typing import Any, Optional
from contextlib import contextmanager

from hogtrace.ast import (
    Expression, Literal, Identifier, FieldAccess, IndexAccess,
    FunctionCall, RequestVar, BinaryOp, UnaryOp
)
from hogtrace.frame import FrameContext
from hogtrace.request_store import RequestLocalStore
from hogtrace.builtins import call_builtin, is_safe_function
from hogtrace.errors import EvaluationError, RecursionError as HogTraceRecursionError, TimeoutError
from hogtrace.limits import HogTraceLimits, DEFAULT_LIMITS


class ExpressionEvaluator:
    """
    Evaluates HogTrace expressions safely.

    Handles all expression types and provides safety mechanisms.
    """

    # Binary operators mapping
    BINARY_OPS = {
        '+': operator.add,
        '-': operator.sub,
        '*': operator.mul,
        '/': operator.truediv,
        '%': operator.mod,
        '==': operator.eq,
        '!=': operator.ne,
        '<': operator.lt,
        '>': operator.gt,
        '<=': operator.le,
        '>=': operator.ge,
        '&&': lambda a, b: bool(a) and bool(b),
        '||': lambda a, b: bool(a) or bool(b),
    }

    def __init__(
        self,
        frame_context: FrameContext,
        request_store: Optional[RequestLocalStore] = None,
        limits: Optional[HogTraceLimits] = None
    ):
        self.frame_context = frame_context
        self.request_store = request_store or RequestLocalStore()
        self.limits = limits or DEFAULT_LIMITS
        self._depth = 0

    def evaluate(self, expr: Expression) -> Any:
        """
        Evaluate an expression.

        Args:
            expr: Expression AST node

        Returns:
            Evaluated value

        Raises:
            EvaluationError: If evaluation fails
            RecursionError: If recursion depth exceeded
            TimeoutError: If evaluation exceeds timeout
        """
        # Check recursion depth
        self._depth += 1
        if self._depth > self.limits.max_recursion_depth:
            raise HogTraceRecursionError(
                f"Expression recursion depth exceeded {self.limits.max_recursion_depth}"
            )

        try:
            result = self._evaluate_expr(expr)
            return result
        finally:
            self._depth -= 1

    def _evaluate_expr(self, expr: Expression) -> Any:
        """Internal expression evaluation"""

        # Literal
        if isinstance(expr, Literal):
            return expr.value

        # Identifier
        if isinstance(expr, Identifier):
            return self._evaluate_identifier(expr)

        # Request variable
        if isinstance(expr, RequestVar):
            return self.request_store.get(expr.name)

        # Field access (obj.field)
        if isinstance(expr, FieldAccess):
            obj = self.evaluate(expr.object)
            return self._safe_getattr(obj, expr.field)

        # Index access (obj[index])
        if isinstance(expr, IndexAccess):
            obj = self.evaluate(expr.object)
            index = self.evaluate(expr.index)
            return self._safe_getitem(obj, index)

        # Function call
        if isinstance(expr, FunctionCall):
            return self._evaluate_function_call(expr)

        # Binary operation
        if isinstance(expr, BinaryOp):
            return self._evaluate_binary_op(expr)

        # Unary operation
        if isinstance(expr, UnaryOp):
            return self._evaluate_unary_op(expr)

        raise EvaluationError(f"Unknown expression type: {type(expr)}")

    def _evaluate_identifier(self, expr: Identifier) -> Any:
        """Evaluate an identifier (variable lookup)"""
        name = expr.name

        # Check frame context
        if self.frame_context.has(name):
            return self.frame_context.get(name)

        # Not found - return None (DTrace-style behavior)
        return None

    def _evaluate_function_call(self, expr: FunctionCall) -> Any:
        """Evaluate a function call"""
        func_name = expr.function

        # Check if function is whitelisted
        if not is_safe_function(func_name):
            raise EvaluationError(f"Function '{func_name}' is not available")

        # Evaluate arguments
        args = [self.evaluate(arg) for arg in expr.arguments]

        # Call the function
        try:
            return call_builtin(func_name, *args)
        except Exception as e:
            raise EvaluationError(f"Error calling {func_name}(): {e}")

    def _evaluate_binary_op(self, expr: BinaryOp) -> Any:
        """Evaluate a binary operation"""
        op = expr.operator

        if op not in self.BINARY_OPS:
            raise EvaluationError(f"Unknown operator: {op}")

        # Evaluate operands
        left = self.evaluate(expr.left)
        right = self.evaluate(expr.right)

        # Apply operator
        try:
            op_func = self.BINARY_OPS[op]
            return op_func(left, right)
        except Exception as e:
            raise EvaluationError(f"Error in {op} operation: {e}")

    def _evaluate_unary_op(self, expr: UnaryOp) -> Any:
        """Evaluate a unary operation"""
        op = expr.operator

        # Evaluate operand
        operand = self.evaluate(expr.operand)

        if op == '!':
            return not bool(operand)
        elif op == '-':
            return -operand
        elif op == '+':
            return +operand
        else:
            raise EvaluationError(f"Unknown unary operator: {op}")

    def _safe_getattr(self, obj: Any, attr: str) -> Any:
        """
        Safely get an attribute from an object.

        Args:
            obj: Object to get attribute from
            attr: Attribute name

        Returns:
            Attribute value

        Raises:
            EvaluationError: If attribute access fails
        """
        # Check for private attributes (single underscore)
        if attr.startswith('_') and not attr.startswith('__'):
            if not self.limits.allow_private_attributes:
                raise EvaluationError(f"Access to private attribute '{attr}' is not allowed")

        # Check for dunder attributes (double underscore)
        if attr.startswith('__') and attr.endswith('__'):
            if not self.limits.allow_dunder_attributes:
                raise EvaluationError(f"Access to dunder attribute '{attr}' is not allowed")

        try:
            return getattr(obj, attr)
        except AttributeError:
            # Return None for missing attributes (DTrace-style)
            return None
        except Exception as e:
            raise EvaluationError(f"Error accessing {attr}: {e}")

    def _safe_getitem(self, obj: Any, index: Any) -> Any:
        """
        Safely get an item from an object.

        Args:
            obj: Object to index
            index: Index or key

        Returns:
            Item value

        Raises:
            EvaluationError: If index access fails
        """
        try:
            return obj[index]
        except (KeyError, IndexError, TypeError):
            # Return None for missing items (DTrace-style)
            return None
        except Exception as e:
            raise EvaluationError(f"Error accessing index {index}: {e}")


@contextmanager
def timeout_context(timeout_ms: Optional[int]):
    """
    Context manager for evaluation timeout using SIGALRM (Unix only).

    On non-Unix systems or when timeout is None, this is a no-op.

    Args:
        timeout_ms: Timeout in milliseconds, or None to disable

    Raises:
        TimeoutError: If evaluation exceeds timeout
    """
    if timeout_ms is None:
        # No timeout
        yield
        return

    # Check if platform supports SIGALRM
    if not hasattr(signal, 'SIGALRM'):
        # Windows or other platforms - no timeout support
        # For production on Windows, consider using threading.Timer
        yield
        return

    def timeout_handler(signum, frame):
        raise TimeoutError(f"Expression evaluation exceeded {timeout_ms}ms timeout")

    # Convert ms to seconds for alarm
    timeout_sec = timeout_ms / 1000.0

    # Set the signal handler and alarm
    old_handler = signal.signal(signal.SIGALRM, timeout_handler)
    signal.setitimer(signal.ITIMER_REAL, timeout_sec)

    try:
        yield
    finally:
        # Cancel the alarm and restore handler
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, old_handler)


def evaluate_expression(
    expr: Expression,
    frame_context: FrameContext,
    request_store: Optional[RequestLocalStore] = None,
    limits: Optional[HogTraceLimits] = None,
    timeout_ms: Optional[int] = None
) -> Any:
    """
    Convenience function to evaluate an expression with optional timeout.

    Args:
        expr: Expression to evaluate
        frame_context: Frame context
        request_store: Request-scoped storage
        limits: Resource limits (uses DEFAULT_LIMITS if not provided)
        timeout_ms: Timeout in milliseconds (overrides limits.max_predicate_time_ms if provided)

    Returns:
        Evaluated value

    Raises:
        TimeoutError: If evaluation exceeds timeout
    """
    effective_limits = limits or DEFAULT_LIMITS
    effective_timeout = timeout_ms if timeout_ms is not None else effective_limits.max_predicate_time_ms

    evaluator = ExpressionEvaluator(frame_context, request_store, effective_limits)

    with timeout_context(effective_timeout):
        return evaluator.evaluate(expr)
