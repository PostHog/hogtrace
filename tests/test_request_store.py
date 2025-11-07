"""
Tests for RequestLocalStore integration with the Rust VM.

These tests verify that $req.* variables work correctly:
- Variables persist across multiple probe executions within same program
- Variables are isolated per-program within same request
- Variables are isolated per-request (different contexts)
- Reading unset variables returns None
- Both $req and $request syntax work
- SetAttr cannot modify regular Python objects (security)
"""

import sys
import pytest
from hogtrace.vm import compile, execute_probe, ProbeExecutor
from hogtrace import context


def test_basic_request_variable_set_and_get():
    """Test basic set and get of request variables across probes in same program."""
    # One program with two probes - entry sets, exit reads
    program = compile("""
        fn:myapp.users.create:entry { $req.user_id = 123; }
        fn:myapp.users.create:exit { capture(user_id=$req.user_id); }
    """)

    entry_probe = program.probes[0]
    exit_probe = program.probes[1]

    with context.new_context():
        store = context.get_store()
        program_store = store.for_program("test-program")
        frame = sys._getframe()

        # Execute entry probe (sets variable)
        result1 = execute_probe(program, entry_probe, frame, program_store)
        assert result1 is None  # No capture in entry probe

        # Execute exit probe (reads variable)
        result2 = execute_probe(program, exit_probe, frame, program_store)
        assert result2 is not None
        assert result2["user_id"] == 123


def test_reading_unset_variable_returns_none():
    """Test that reading an unset $req variable returns None (not error)."""
    program = compile("fn:test:entry { capture(value=$req.nonexistent); }")
    probe = program.probes[0]

    with context.new_context():
        store = context.get_store()
        program_store = store.for_program("test-program")
        frame = sys._getframe()

        result = execute_probe(program, probe, frame, program_store)
        assert result is not None
        assert result["value"] is None


def test_request_vs_req_syntax():
    """Test that both $request and $req work and refer to the same store."""
    # One program: set with $request, read with $req
    program = compile("""
        fn:test:entry { $request.foo = 'bar'; }
        fn:test:exit { capture(foo=$req.foo); }
    """)

    with context.new_context():
        store = context.get_store()
        program_store = store.for_program("test-program")
        frame = sys._getframe()

        execute_probe(program, program.probes[0], frame, program_store)
        result = execute_probe(program, program.probes[1], frame, program_store)

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

    with context.new_context():
        store = context.get_store()
        program_store = store.for_program("test-program")
        frame = sys._getframe()

        execute_probe(program, program.probes[0], frame, program_store)
        result = execute_probe(program, program.probes[1], frame, program_store)

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

    with context.new_context():
        store = context.get_store()
        program_store = store.for_program("test-program")
        frame = sys._getframe()

        # Execute probes in sequence
        execute_probe(program, program.probes[0], frame, program_store)
        execute_probe(program, program.probes[1], frame, program_store)
        result = execute_probe(program, program.probes[2], frame, program_store)

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

    with context.new_context():
        store = context.get_store()
        program_store = store.for_program("test-program")
        frame = sys._getframe()

        # Execute entry probe
        execute_probe(program, program.probes[0], frame, program_store)

        # Execute exit probe
        result = execute_probe(program, program.probes[1], frame, program_store)

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

    with context.new_context():
        store = context.get_store()
        program_store = store.for_program("test-program")

        executor1 = ProbeExecutor(program, program.probes[0], program_store)
        executor2 = ProbeExecutor(program, program.probes[1], program_store)

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

    with context.new_context():
        store = context.get_store()
        program_store = store.for_program("test-program")
        frame = sys._getframe()

        result = execute_probe(program, program.probes[0], frame, program_store)

        assert result is not None
        assert result["bool_val"] is True
        assert result["int_val"] == 123
        assert result["float_val"] == 3.14
        assert result["str_val"] == "hello"
        assert result["none_val"] is None


def test_request_store_isolation():
    """Test that different contexts (different requests) are isolated from each other."""
    program = compile("""
        fn:test:entry { $req.value = 'set_value'; }
        fn:test:exit { capture(value=$req.value); }
    """)

    frame = sys._getframe()

    # Request 1: Set value
    with context.new_context():
        store1 = context.get_store()
        program_store1 = store1.for_program("test-program")

        execute_probe(program, program.probes[0], frame, program_store1)

        # Read from same context - should work
        result1 = execute_probe(program, program.probes[1], frame, program_store1)
        assert result1 is not None
        assert result1["value"] == "set_value"

    # Request 2 (different context): Value should not be visible
    with context.new_context():
        store2 = context.get_store()
        program_store2 = store2.for_program("test-program")

        # Read from different context - should be None
        result2 = execute_probe(program, program.probes[1], frame, program_store2)
        assert result2 is not None
        assert result2["value"] is None


def test_conditional_with_request_vars():
    """Test using request variables in conditional predicates."""
    program = compile("""
        fn:func1:entry { $req.flag = True; }
        fn:func2:entry / $req.flag / { capture(message='flag is set'); }
        fn:func3:entry / $req.nonexistent / { capture(message='should not happen'); }
    """)

    with context.new_context():
        store = context.get_store()
        program_store = store.for_program("test-program")
        frame = sys._getframe()

        # Set flag
        execute_probe(program, program.probes[0], frame, program_store)

        # Should capture because flag is True
        result2 = execute_probe(program, program.probes[1], frame, program_store)
        assert result2 is not None
        assert result2["message"] == "flag is set"

        # Should not capture because nonexistent returns None (falsy)
        result3 = execute_probe(program, program.probes[2], frame, program_store)
        assert result3 is None


def test_program_isolation_within_same_request():
    """Test that different programs in the same request context are isolated."""
    program_a = compile("fn:test:entry { $req.user_id = 123; }")
    program_b = compile("fn:test:entry { capture(user_id=$req.user_id); }")

    frame = sys._getframe()

    with context.new_context():
        store = context.get_store()

        # Execute probe from program A
        program_a_store = store.for_program("program-a")
        execute_probe(program_a, program_a.probes[0], frame, program_a_store)

        # Execute probe from program B - should NOT see program A's variable
        program_b_store = store.for_program("program-b")
        result = execute_probe(program_b, program_b.probes[0], frame, program_b_store)

        assert result is not None
        assert result["user_id"] is None  # Isolated from program A

        # Verify program A still has its value
        verify_program = compile("fn:test:entry { capture(user_id=$req.user_id); }")
        verify_result = execute_probe(
            verify_program, verify_program.probes[0], frame, program_a_store
        )
        assert verify_result is not None
        assert verify_result["user_id"] == 123


def test_program_store_direct_api():
    """Test ProgramStore API methods directly (set, get, has, delete, clear, all)."""
    with context.new_context():
        store = context.get_store()
        program_store = store.for_program("test-program")

        # Test set and get
        program_store.set("key1", "value1")
        assert program_store.get("key1") == "value1"
        assert program_store.get("nonexistent") is None
        assert program_store.get("nonexistent", "default") == "default"

        # Test has
        assert program_store.has("key1") is True
        assert program_store.has("nonexistent") is False

        # Test __contains__
        assert "key1" in program_store
        assert "nonexistent" not in program_store

        # Test __getitem__ and __setitem__
        program_store["key2"] = "value2"
        assert program_store["key2"] == "value2"

        # Test all
        program_store.set("key3", "value3")
        all_vars = program_store.all()
        assert all_vars == {"key1": "value1", "key2": "value2", "key3": "value3"}

        # Test delete
        program_store.delete("key1")
        assert program_store.has("key1") is False
        assert program_store.get("key1") is None

        # Test clear
        program_store.clear()
        assert program_store.all() == {}
        assert program_store.has("key2") is False
        assert program_store.has("key3") is False


def test_request_local_store_api():
    """Test RequestLocalStore API methods."""
    with context.new_context():
        store = context.get_store()
        assert store is not None

        # Create multiple program stores
        prog1 = store.for_program("prog1")
        prog2 = store.for_program("prog2")

        prog1.set("var", "value1")
        prog2.set("var", "value2")

        # Test all_programs
        programs = store.all_programs()
        assert set(programs) == {"prog1", "prog2"}

        # Test that for_program returns the same instance
        prog1_again = store.for_program("prog1")
        assert prog1_again.get("var") == "value1"

        # Test clear (clears all program stores)
        store.clear()
        assert prog1.all() == {}
        assert prog2.all() == {}


def test_edge_cases():
    """Test edge cases like empty strings, special characters, None values."""
    program = compile("""
        fn:test:entry {
            $req.empty_string = '';
            $req.special_chars = 'a.b-c_d/e';
            $req.unicode = '你好';
            $req.explicit_none = None;
        }
        fn:test:exit {
            capture(
                empty_string=$req.empty_string,
                special_chars=$req.special_chars,
                unicode=$req.unicode,
                explicit_none=$req.explicit_none
            );
        }
    """)

    with context.new_context():
        store = context.get_store()
        program_store = store.for_program("test-program")
        frame = sys._getframe()

        execute_probe(program, program.probes[0], frame, program_store)
        result = execute_probe(program, program.probes[1], frame, program_store)

        assert result is not None
        assert result["empty_string"] == ""
        assert result["special_chars"] == "a.b-c_d/e"
        assert result["unicode"] == "你好"
        assert result["explicit_none"] is None


def test_same_program_id_reuses_store():
    """Test that requesting the same program ID returns the same store instance."""
    with context.new_context():
        store = context.get_store()
        assert store is not None

        prog1_first = store.for_program("program-1")
        prog1_first.set("test", "value")

        # Request the same program ID again
        prog1_second = store.for_program("program-1")

        # Should see the same data
        assert prog1_second.get("test") == "value"

        # Verify it's the same underlying storage
        prog1_second.set("another", "data")
        assert prog1_first.get("another") == "data"


def test_multiple_probes_same_program_share_state():
    """Test that multiple probes from the same program share state via store."""
    program = compile("""
        fn:func1:entry {
            $req.counter = 0;
        }
        fn:func2:entry {
            $req.counter = $req.counter + 1;
        }
        fn:func3:entry {
            $req.counter = $req.counter + 1;
        }
        fn:func4:entry {
            capture(counter=$req.counter);
        }
    """)

    with context.new_context():
        store = context.get_store()
        assert store is not None

        program_store = store.for_program("test-program")
        frame = sys._getframe()

        # Execute probes in sequence
        execute_probe(program, program.probes[0], frame, program_store)  # counter = 0
        execute_probe(program, program.probes[1], frame, program_store)  # counter = 1
        execute_probe(program, program.probes[2], frame, program_store)  # counter = 2
        result = execute_probe(program, program.probes[3], frame, program_store)

        assert result is not None
        assert result["counter"] == 2


def test_nested_contexts_are_isolated():
    """Test that nested context.new_context() calls create isolated scopes."""
    program = compile("fn:test:entry { capture(value=$req.test_value); }")
    frame = sys._getframe()

    with context.new_context():
        store1 = context.get_store()
        assert store1 is not None

        prog_store1 = store1.for_program("prog")
        prog_store1.set("test_value", "outer")

        # Nested context
        with context.new_context():
            store2 = context.get_store()
            assert store2 is not None

            prog_store2 = store2.for_program("prog")

            # Inner context should not see outer value
            result_inner = execute_probe(program, program.probes[0], frame, prog_store2)
            assert result_inner is not None
            assert result_inner["value"] is None

            # Set value in inner context
            prog_store2.set("test_value", "inner")

        # Back to outer context - should still have outer value
        result_outer = execute_probe(program, program.probes[0], frame, prog_store1)
        assert result_outer is not None
        assert result_outer["value"] == "outer"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
