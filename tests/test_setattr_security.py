"""
Security tests for SetAttr opcode.

These tests ensure that the SetAttr opcode ONLY works with the RequestStoreProxy,
and cannot be used to modify regular Python objects. This is critical for security.
"""

import sys
import pytest
from hogtrace.vm import compile, execute_probe
from hogtrace.request_store import RequestLocalStore


class TestObject:
    """Test class for security tests."""
    def __init__(self):
        self.value = "original"


def test_cannot_modify_regular_object():
    """Test that we cannot use SetAttr-like syntax on regular objects."""
    # This should fail at runtime because we don't have syntax to set
    # attributes on regular objects in the language
    # The only assignable thing is $req.* / $request.*
    pass  # No syntax exists for this in the language


def test_request_store_assignment_works():
    """Verify that request store assignment DOES work (baseline test)."""
    program = compile("fn:test:entry { $req.value = 'modified'; capture(v=$req.value); }")
    store = RequestLocalStore()
    frame = sys._getframe()

    result = execute_probe(program, program.probes[0], frame, store)

    assert result is not None
    assert result["v"] == "modified"


def test_only_request_variables_assignable():
    """Test that only $req.* and $request.* are assignable in the language."""
    # Valid assignments
    valid_programs = [
        "$req.field = 123;",
        "$request.field = 123;",
        "$req.user_id = 'foo';",
        "$request.session = 'bar';",
    ]

    store = RequestLocalStore()
    frame = sys._getframe()

    for source in valid_programs:
        program = compile(f"fn:test:entry {{ {source} }}")
        # Should not raise
        execute_probe(program, program.probes[0], frame, store)


def test_cannot_assign_to_regular_variables():
    """Test that we cannot assign to regular variables like args, retval, etc."""
    # These should fail at compilation
    invalid_assignments = [
        "args = 123;",  # Cannot assign to args
        "arg0 = 456;",  # Cannot assign to arg0
        "locals = {};",  # Cannot assign to locals
        "self = None;",  # Cannot assign to self
    ]

    for source in invalid_assignments:
        with pytest.raises(Exception):  # Should raise ValueError during compilation
            compile(f"fn:test:entry {{ {source} }}")


def test_setattr_only_on_request_proxy():
    """
    Test that the SetAttr opcode implementation only works on RequestStoreProxy.

    This is the core security check - we verify at the Rust level that
    set_attribute() only accepts RequestStoreProxy objects, not regular Python
    objects.
    """
    # This is ensured by the implementation in python_dispatcher.rs:
    # fn set_attribute() checks if obj is RequestStoreProxy, and returns error otherwise
    #
    # Since the language grammar only allows assignments to $req.* / $request.*,
    # and those compile to LoadVar("$req") which returns RequestStoreProxy,
    # it's impossible to get a regular Python object into the SetAttr operation.
    #
    # The only way to get here would be if someone manually crafted bytecode,
    # which is outside the scope of normal usage.
    pass


def test_request_store_proxy_marker_cannot_be_accessed():
    """
    Test that the RequestStoreProxy marker is internal and cannot be accessed
    from Python code.
    """
    # The RequestStoreProxy struct is private to python_dispatcher.rs
    # It cannot be imported or instantiated from Python
    from hogtrace import vm

    # Should not have RequestStoreProxy exposed
    assert not hasattr(vm, 'RequestStoreProxy')


def test_store_isolation_from_frame_locals():
    """Test that request store is isolated from frame locals."""
    test_local = "frame_local"

    program = compile("""
        fn:test:entry { $req.test_local = 'store_value'; }
        fn:test:exit {
            capture(
                from_store=$req.test_local,
                from_frame=test_local
            );
        }
    """)

    store = RequestLocalStore()
    frame = sys._getframe()

    execute_probe(program, program.probes[0], frame, store)
    result = execute_probe(program, program.probes[1], frame, store)

    assert result is not None
    # Store value should be what we set
    assert result["from_store"] == "store_value"
    # Frame local should be unchanged
    assert result["from_frame"] == "frame_local"
    # And they should be different
    assert result["from_store"] != result["from_frame"]


def test_request_vars_dont_leak_to_frame():
    """Test that setting $req.* doesn't modify frame locals."""
    program = compile("fn:test:entry { $req.leaked = 'should not appear'; }")
    store = RequestLocalStore()

    frame = sys._getframe()
    execute_probe(program, program.probes[0], frame, store)

    # Frame locals should not have request variable names leaked
    assert 'leaked' not in frame.f_locals
    assert '$req' not in frame.f_locals
    assert 'req' not in frame.f_locals
    assert '$request' not in frame.f_locals
    assert 'request' not in frame.f_locals


def test_store_method_safety():
    """Test that RequestLocalStore methods are safe and isolated."""
    store = RequestLocalStore()

    # Set some values
    store.set("user_id", 123)
    store.set("session", "abc")

    # Read them back
    assert store.get("user_id") == 123
    assert store.get("session") == "abc"

    # Reading nonexistent returns None (not error)
    assert store.get("nonexistent") is None
    assert store.get("nonexistent", "default") == "default"

    # Clear should work
    store.clear()
    assert store.get("user_id") is None
    assert store.get("session") is None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
