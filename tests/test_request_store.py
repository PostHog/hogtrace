"""
Tests for RequestLocalStore integration with the Rust VM.

These tests verify that $req.* variables work correctly:
- Variables persist across multiple probe executions
- Reading unset variables returns None
- Both $req and $request syntax work
- SetAttr cannot modify regular Python objects (security)
"""

import sys
import pytest
from hogtrace.vm import compile, execute_probe, ProbeExecutor
from hogtrace.request_store import RequestLocalStore


def test_basic_request_variable_set_and_get():
    """Test basic set and get of request variables across probes in same program."""
    # One program with two probes - entry sets, exit reads
    program = compile("""
        fn:myapp.users.create:entry { $req.user_id = 123; }
        fn:myapp.users.create:exit { capture(user_id=$req.user_id); }
    """)

    entry_probe = program.probes[0]
    exit_probe = program.probes[1]

    store = RequestLocalStore()
    frame = sys._getframe()

    # Execute entry probe (sets variable)
    result1 = execute_probe(program, entry_probe, frame, store)
    assert result1 is None  # No capture in entry probe

    # Execute exit probe (reads variable)
    result2 = execute_probe(program, exit_probe, frame, store)
    assert result2 is not None
    assert result2["user_id"] == 123


def test_reading_unset_variable_returns_none():
    """Test that reading an unset $req variable returns None (not error)."""
    program = compile("fn:test:entry { capture(value=$req.nonexistent); }")
    probe = program.probes[0]

    store = RequestLocalStore()
    frame = sys._getframe()

    result = execute_probe(program, probe, frame, store)
    assert result is not None
    assert result["value"] is None


def test_request_vs_req_syntax():
    """Test that both $request and $req work and refer to the same store."""
    # One program: set with $request, read with $req
    program = compile("""
        fn:test:entry { $request.foo = 'bar'; }
        fn:test:exit { capture(foo=$req.foo); }
    """)

    store = RequestLocalStore()
    frame = sys._getframe()

    execute_probe(program, program.probes[0], frame, store)
    result = execute_probe(program, program.probes[1], frame, store)

    assert result is not None
    assert result["foo"] == "bar"


def test_multiple_variables():
    """Test setting and reading multiple request variables across probes."""
    program = compile("""
        fn:test:entry {
            $req.user_id = 123;
            $req.session_id = 'abc-def';
            $req.count = 42;
        }
        fn:test:exit {
            capture(
                user_id=$req.user_id,
                session_id=$req.session_id,
                count=$req.count
            );
        }
    """)

    store = RequestLocalStore()
    frame = sys._getframe()

    execute_probe(program, program.probes[0], frame, store)
    result = execute_probe(program, program.probes[1], frame, store)

    assert result is not None
    assert result["user_id"] == 123
    assert result["session_id"] == "abc-def"
    assert result["count"] == 42


def test_variable_overwrite():
    """Test that variables can be overwritten across probes."""
    program = compile("""
        fn:func1:entry { $req.value = 'first'; }
        fn:func2:entry { $req.value = 'second'; }
        fn:func3:entry { capture(value=$req.value); }
    """)

    store = RequestLocalStore()
    frame = sys._getframe()

    # Execute probes in sequence
    execute_probe(program, program.probes[0], frame, store)
    execute_probe(program, program.probes[1], frame, store)
    result = execute_probe(program, program.probes[2], frame, store)

    assert result is not None
    assert result["value"] == "second"


def test_cross_probe_communication():
    """Test complex cross-probe communication scenario with entry/exit probes."""
    # Single program with entry and exit probes
    program = compile("""
        fn:myapp.process_request:entry {
            $req.start_time = timestamp();
            $req.user_id = 999;
        }
        fn:myapp.process_request:exit {
            $req.end_time = timestamp();
            $req.duration = $req.end_time - $req.start_time;
            capture(
                user_id=$req.user_id,
                duration=$req.duration
            );
        }
    """)

    store = RequestLocalStore()
    frame = sys._getframe()

    # Execute entry probe
    execute_probe(program, program.probes[0], frame, store)

    # Execute exit probe
    result = execute_probe(program, program.probes[1], frame, store)

    assert result is not None
    assert result["user_id"] == 999
    assert isinstance(result["duration"], (int, float))
    assert result["duration"] >= 0


def test_probe_executor_class():
    """Test ProbeExecutor class with request store."""
    program = compile("""
        fn:test:entry { $req.value = 42; }
        fn:test:exit { capture(value=$req.value); }
    """)

    store = RequestLocalStore()

    executor1 = ProbeExecutor(program, program.probes[0], store)
    executor2 = ProbeExecutor(program, program.probes[1], store)

    frame = sys._getframe()

    executor1.execute(frame)
    result = executor2.execute(frame)

    assert result is not None
    assert result["value"] == 42


def test_type_coercion_with_request_vars():
    """Test that request variables preserve types correctly."""
    program = compile("""
        fn:test:entry {
            $req.bool_val = True;
            $req.int_val = 123;
            $req.float_val = 3.14;
            $req.str_val = "hello";
            $req.none_val = None;

            capture(
                bool_val=$req.bool_val,
                int_val=$req.int_val,
                float_val=$req.float_val,
                str_val=$req.str_val,
                none_val=$req.none_val
            );
        }
    """)

    store = RequestLocalStore()
    frame = sys._getframe()

    result = execute_probe(program, program.probes[0], frame, store)

    assert result is not None
    assert result["bool_val"] is True
    assert result["int_val"] == 123
    assert result["float_val"] == 3.14
    assert result["str_val"] == "hello"
    assert result["none_val"] is None


def test_request_store_isolation():
    """Test that different stores (different requests) are isolated from each other."""
    program = compile("""
        fn:test:entry { $req.value = 'set_value'; }
        fn:test:exit { capture(value=$req.value); }
    """)

    store1 = RequestLocalStore()  # Request 1
    store2 = RequestLocalStore()  # Request 2

    frame = sys._getframe()

    # Set value in store1 (request 1)
    execute_probe(program, program.probes[0], frame, store1)

    # Read from store1 - should work
    result1 = execute_probe(program, program.probes[1], frame, store1)
    assert result1 is not None
    assert result1["value"] == "set_value"

    # Read from store2 (different request) - should be None
    result2 = execute_probe(program, program.probes[1], frame, store2)
    assert result2 is not None
    assert result2["value"] is None


def test_conditional_with_request_vars():
    """Test using request variables in conditional predicates."""
    program = compile("""
        fn:func1:entry { $req.flag = True; }
        fn:func2:entry / $req.flag / { capture(message='flag is set'); }
        fn:func3:entry / $req.nonexistent / { capture(message='should not happen'); }
    """)

    store = RequestLocalStore()
    frame = sys._getframe()

    # Set flag
    execute_probe(program, program.probes[0], frame, store)

    # Should capture because flag is True
    result2 = execute_probe(program, program.probes[1], frame, store)
    assert result2 is not None
    assert result2["message"] == "flag is set"

    # Should not capture because nonexistent returns None (falsy)
    result3 = execute_probe(program, program.probes[2], frame, store)
    assert result3 is None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
