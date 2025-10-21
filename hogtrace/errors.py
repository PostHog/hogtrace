"""
Exceptions for HogTrace VM.

All exceptions inherit from HogTraceError for easy catching.
"""


class HogTraceError(Exception):
    """
    Base exception for HogTrace VM errors.

    All HogTrace-specific exceptions inherit from this.
    """
    pass


class ParseError(HogTraceError):
    """
    Error during parsing of HogTrace code.

    Raised when the input code has syntax errors or invalid structure.
    """
    pass


class EvaluationError(HogTraceError):
    """
    Error during expression evaluation.

    Raised when an expression cannot be evaluated, such as:
    - Unknown operators or functions
    - Type errors in operations
    - Missing required attributes
    """

    def __init__(self, message: str, suggestion: str = None):
        super().__init__(message)
        self.suggestion = suggestion

    def __str__(self):
        msg = super().__str__()
        if self.suggestion:
            msg += f"\n  Suggestion: {self.suggestion}"
        return msg


class TimeoutError(HogTraceError):
    """
    Probe execution exceeded time limit.

    Raised when predicate evaluation or action execution takes too long.
    This prevents slow probes from impacting application performance.
    """
    pass


class RecursionError(HogTraceError):
    """
    Expression recursion depth exceeded.

    Raised when evaluating deeply nested expressions to prevent stack overflow.
    """
    pass


class UnsafeOperationError(HogTraceError):
    """
    Attempted unsafe operation.

    Raised when trying to access private attributes, call unsafe functions,
    or perform other operations that violate security constraints.
    """
    pass


class CaptureSizeError(HogTraceError):
    """
    Captured data exceeded size limit.

    Raised when capture() tries to collect more data than the configured limit.
    This prevents memory exhaustion from capturing large objects.
    """
    pass


class RateLimitError(HogTraceError):
    """
    Probe fired too frequently.

    Raised when a probe exceeds its configured rate limit.
    This prevents hot paths from generating excessive telemetry.
    """
    pass
