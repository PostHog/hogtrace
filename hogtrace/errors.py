"""
Exceptions for HogTrace VM.
"""


class HogTraceError(Exception):
    """Base exception for HogTrace VM errors"""
    pass


class EvaluationError(HogTraceError):
    """Error during expression evaluation"""
    pass


class TimeoutError(HogTraceError):
    """Probe execution exceeded time limit"""
    pass


class RecursionError(HogTraceError):
    """Expression recursion depth exceeded"""
    pass


class UnsafeOperationError(HogTraceError):
    """Attempted unsafe operation"""
    pass
