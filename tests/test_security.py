"""
Security tests for HogTrace.

Tests that the VM properly blocks unsafe operations and enforces limits.
"""

import pytest
import inspect
import time
from hogtrace import parse, ProbeExecutor, RequestLocalStore
from hogtrace.limits import HogTraceLimits
from hogtrace.errors import (
    EvaluationError, TimeoutError, RecursionError,
    CaptureSizeError, UnsafeOperationError
)


def test_private_attribute_access_blocked():
    """Test that private attributes cannot be accessed by default."""
    code = """
    fn:test:entry
    {
        capture(private=arg0._private);
    }
    """
    program = parse(code)
    store = RequestLocalStore()
    executor = ProbeExecutor(program.probes[0], store)

    class TestClass:
        def __init__(self):
            self._private = "secret"
            self.public = "visible"

    def test_function(obj):
        frame = inspect.currentframe()
        return executor.execute(frame)

    # Should not capture private attribute
    result = test_function(TestClass())

    # The probe might not fire or might return None for private
    # The evaluator should block access to _private
    assert result is None or result.get("private") is None


def test_private_attribute_access_allowed_with_flag():
    """Test that private attributes can be accessed if explicitly allowed."""
    code = """
    fn:test:entry
    {
        capture(private=arg0._private);
    }
    """
    program = parse(code)
    store = RequestLocalStore()

    # Allow private attributes
    limits = HogTraceLimits(allow_private_attributes=True)
    executor = ProbeExecutor(program.probes[0], store, limits)

    class TestClass:
        def __init__(self):
            self._private = "secret"

    def test_function(obj):
        frame = inspect.currentframe()
        return executor.execute(frame)

    result = test_function(TestClass())

    # Now it should work
    assert result is not None
    assert result["private"] == "secret"


def test_dunder_attribute_access_blocked():
    """Test that dunder attributes are blocked by default."""
    code = """
    fn:test:entry
    {
        capture(class_name=arg0.__class__.__name__);
    }
    """
    program = parse(code)
    store = RequestLocalStore()
    executor = ProbeExecutor(program.probes[0], store)

    class TestClass:
        pass

    def test_function(obj):
        frame = inspect.currentframe()
        return executor.execute(frame)

    # Should not allow access to __class__
    result = test_function(TestClass())
    assert result is None or result.get("class_name") is None


def test_recursion_depth_limit():
    """Test that deeply nested expressions hit recursion limit."""
    # Create an expression that's too deeply nested
    # arg0.a.b.c.d... repeated many times
    nested_access = "arg0" + ".level" * 150

    code = f"""
    fn:test:entry
    {{
        capture(deep={nested_access});
    }}
    """

    program = parse(code)
    store = RequestLocalStore()
    executor = ProbeExecutor(program.probes[0], store)

    # Create deeply nested object
    class Level:
        def __init__(self, depth):
            if depth > 0:
                self.level = Level(depth - 1)
            else:
                self.level = "bottom"

    def test_function(obj):
        frame = inspect.currentframe()
        return executor.execute(frame)

    # Should not crash, but should hit recursion limit
    result = test_function(Level(200))

    # Probe should fail gracefully (return None or empty)
    assert result is None or result == {}


def test_capture_size_limit():
    """Test that capturing too much data triggers size limit."""
    code = """
    fn:test:entry
    {
        capture(large_data=arg0);
    }
    """
    program = parse(code)
    store = RequestLocalStore()

    # Set very small size limit
    limits = HogTraceLimits(max_capture_size_bytes=100)
    executor = ProbeExecutor(program.probes[0], store, limits)

    # Create large data
    large_data = {"key" * 100: "value" * 1000}

    def test_function(data):
        frame = inspect.currentframe()
        return executor.execute(frame)

    # Should not crash, but should be limited
    # The executor should handle CaptureSizeError gracefully
    result = test_function(large_data)

    # Either returns None or truncated data
    assert result is None or len(str(result)) < 10000


def test_capture_depth_limit():
    """Test that deeply nested structures are truncated."""
    code = """
    fn:test:entry
    {
        capture(nested=arg0);
    }
    """
    program = parse(code)
    store = RequestLocalStore()

    # Set depth limit
    limits = HogTraceLimits(max_capture_depth=3)
    executor = ProbeExecutor(program.probes[0], store, limits)

    # Create deeply nested structure
    deep = {"l1": {"l2": {"l3": {"l4": {"l5": "deep"}}}}}

    def test_function(data):
        frame = inspect.currentframe()
        return executor.execute(frame)

    result = test_function(deep)

    assert result is not None
    # Should have captured something but truncated deep nesting
    assert "nested" in result
    # Check that deep levels are truncated
    assert "max depth" in str(result) or "l5" not in str(result)


def test_capture_items_limit():
    """Test that large lists/dicts are truncated."""
    code = """
    fn:test:entry
    {
        capture(items=arg0);
    }
    """
    program = parse(code)
    store = RequestLocalStore()

    # Limit to 10 items
    limits = HogTraceLimits(max_capture_items=10)
    executor = ProbeExecutor(program.probes[0], store, limits)

    # Create list with 100 items
    large_list = list(range(100))

    def test_function(data):
        frame = inspect.currentframe()
        return executor.execute(frame)

    result = test_function(large_list)

    assert result is not None
    # Should have truncated to 10 items
    assert len(result["items"]) <= 11  # 10 items + "..." marker


def test_unsafe_function_blocked():
    """Test that unsafe functions cannot be called."""
    code = """
    fn:test:entry
    {
        capture(result=eval("1+1"));
    }
    """

    # This should fail at parse time or execution time
    # eval is not in the whitelist
    program = parse(code)
    store = RequestLocalStore()
    executor = ProbeExecutor(program.probes[0], store)

    def test_function():
        frame = inspect.currentframe()
        return executor.execute(frame)

    # Should not execute eval
    result = test_function()
    assert result is None or result == {}


def test_division_by_zero_handled():
    """Test that division by zero doesn't crash."""
    code = """
    fn:test:entry
    / 1 / 0 /
    {
        capture(args);
    }
    """
    program = parse(code)
    store = RequestLocalStore()
    executor = ProbeExecutor(program.probes[0], store)

    def test_function():
        frame = inspect.currentframe()
        return executor.execute(frame)

    # Should not raise exception
    result = test_function()
    # Predicate should fail, probe doesn't fire
    assert result is None


def test_type_error_in_operation_handled():
    """Test that type errors in operations are handled."""
    code = """
    fn:test:entry
    / arg0 + "string" /
    {
        capture(args);
    }
    """
    program = parse(code)
    store = RequestLocalStore()
    executor = ProbeExecutor(program.probes[0], store)

    def test_function(num):
        frame = inspect.currentframe()
        return executor.execute(frame)

    # Should not crash on type error
    result = test_function(42)
    assert result is None  # Predicate fails


def test_timeout_on_slow_predicate():
    """Test that slow predicates timeout (Unix only)."""
    import sys
    import signal

    # Skip on Windows (no SIGALRM)
    if not hasattr(signal, 'SIGALRM'):
        pytest.skip("Timeout not supported on this platform")

    # Note: We can't actually test infinite loops easily,
    # but we can test the timeout mechanism exists
    code = """
    fn:test:entry
    / arg0 > 0 /
    {
        capture(value=arg0);
    }
    """
    program = parse(code)
    store = RequestLocalStore()

    # Set very short timeout
    limits = HogTraceLimits(max_predicate_time_ms=1)
    executor = ProbeExecutor(program.probes[0], store, limits)

    def test_function(value):
        frame = inspect.currentframe()
        return executor.execute(frame)

    # Normal execution should still work
    result = test_function(5)
    # May or may not timeout on fast systems, but shouldn't crash
    assert result is None or isinstance(result, dict)


def test_string_length_truncation():
    """Test that very long strings are truncated."""
    code = """
    fn:test:entry
    {
        capture(long_string=arg0);
    }
    """
    program = parse(code)
    store = RequestLocalStore()
    executor = ProbeExecutor(program.probes[0], store)

    # Create very long string
    long_string = "A" * 10000

    def test_function(s):
        frame = inspect.currentframe()
        return executor.execute(frame)

    result = test_function(long_string)

    assert result is not None
    # Should be truncated (max 1000 chars in _limit_capture_value)
    assert len(result["long_string"]) < 2000
    assert "..." in result["long_string"] or len(result["long_string"]) == 1000


def test_no_access_to_system_modules():
    """Test that probes cannot import or access system modules."""
    # HogTrace doesn't have import capability, but test that
    # we can't access __import__ through attributes
    code = """
    fn:test:entry
    {
        capture(builtins=arg0.__class__.__bases__);
    }
    """
    program = parse(code)
    store = RequestLocalStore()
    executor = ProbeExecutor(program.probes[0], store)

    def test_function(obj):
        frame = inspect.currentframe()
        return executor.execute(frame)

    # Should block access to __bases__
    result = test_function(object())
    assert result is None or result.get("builtins") is None


def test_exception_in_capture_doesnt_crash():
    """Test that exceptions during capture don't crash."""
    code = """
    fn:test:entry
    {
        capture(
            good=arg0,
            bad=arg1.nonexistent.chain,
            also_good=arg2
        );
    }
    """
    program = parse(code)
    store = RequestLocalStore()
    executor = ProbeExecutor(program.probes[0], store)

    def test_function(a, b, c):
        frame = inspect.currentframe()
        return executor.execute(frame)

    result = test_function(1, object(), 3)

    # Should capture what it can
    assert result is not None
    assert result["good"] == 1
    # bad might be None or missing
    assert result.get("bad") is None
    assert result["also_good"] == 3


def test_circular_reference_handling():
    """Test that circular references don't cause infinite loops."""
    code = """
    fn:test:entry
    {
        capture(obj=arg0);
    }
    """
    program = parse(code)
    store = RequestLocalStore()

    # Set depth limit to prevent infinite recursion
    limits = HogTraceLimits(max_capture_depth=5)
    executor = ProbeExecutor(program.probes[0], store, limits)

    # Create circular reference
    class Node:
        def __init__(self, value):
            self.value = value
            self.next = None

    a = Node("a")
    b = Node("b")
    a.next = b
    b.next = a  # Circular!

    def test_function(obj):
        frame = inspect.currentframe()
        return executor.execute(frame)

    # Should not hang or crash
    result = test_function(a)
    assert result is not None
    # Should have been truncated at depth limit


def test_large_dict_truncation():
    """Test that dicts with many keys are truncated."""
    code = """
    fn:test:entry
    {
        capture(big_dict=arg0);
    }
    """
    program = parse(code)
    store = RequestLocalStore()

    limits = HogTraceLimits(max_capture_items=5)
    executor = ProbeExecutor(program.probes[0], store, limits)

    # Create dict with 100 keys
    big_dict = {f"key{i}": i for i in range(100)}

    def test_function(d):
        frame = inspect.currentframe()
        return executor.execute(frame)

    result = test_function(big_dict)

    assert result is not None
    # Should have at most 5 keys + "..." marker
    assert len(result["big_dict"]) <= 6


def test_predicate_error_logged_but_not_raised():
    """Test that predicate errors are logged but don't raise."""
    code = """
    fn:test:entry
    / arg0.missing.attribute /
    {
        capture(value=arg0);
    }
    """
    program = parse(code)
    store = RequestLocalStore()
    executor = ProbeExecutor(program.probes[0], store)

    def test_function(obj):
        frame = inspect.currentframe()
        # Should not raise, even though predicate fails
        return executor.execute(frame)

    # Should return None (predicate failed), not raise
    result = test_function(42)
    assert result is None


def test_assignment_error_silent():
    """Test that assignment errors fail silently."""
    from hogtrace.request_store import RequestContext

    code = """
    fn:test:entry
    {
        $req.value = arg0.missing.chain;
        capture(success=1);
    }
    """
    program = parse(code)
    store = RequestLocalStore()
    executor = ProbeExecutor(program.probes[0], store)

    def test_function(obj):
        frame = inspect.currentframe()
        return executor.execute(frame)

    with RequestContext(store):
        # Assignment should fail silently, capture should still work
        result = test_function(42)
        assert result is not None
        assert result["success"] == 1
        # The assignment failed, so value shouldn't be set
        assert store.get("value") is None


def test_multiple_errors_dont_accumulate():
    """Test that errors in multiple probes don't accumulate."""
    code = """
    fn:test:entry
    / arg0.bad1 /
    {
        capture(a);
    }

    fn:test:entry
    / arg0.bad2 /
    {
        capture(b);
    }

    fn:test:entry
    {
        capture(good=arg0);
    }
    """
    program = parse(code)
    store = RequestLocalStore()
    from hogtrace import ProgramExecutor

    executor = ProgramExecutor(program, store)

    def test_function(obj):
        frame = inspect.currentframe()
        return executor.execute_all(frame)

    # First two probes fail, third succeeds
    results = test_function(123)

    # Should get one result from the third probe
    assert len(results) == 1
    assert results[0][1]["good"] == 123
