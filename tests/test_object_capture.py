"""
Tests for capturing complex Python objects.

Verifies that objects are serialized to JSON-like structures on a best-effort basis.
"""

import sys
import pytest
from hogtrace.vm import compile, execute_probe
from hogtrace import context


class SomeObj:
    """Test class with attributes."""

    def __init__(self, name: str, age: int):
        self.name = name
        self.age = age


def test_capture_simple_object():
    """Test capturing a simple object with attributes."""
    program = compile("fn:test:entry { capture(obj=obj); }")

    with context.new_context():
        store = context.get_store()
        assert store is not None
        program_store = store.for_program("test-program")

        obj = SomeObj("Alice", 30)  # noqa
        frame = sys._getframe()

        result = execute_probe(program, program.probes[0], frame, program_store)

        assert result is not None
        assert "obj" in result
        # Object should be serialized to a JSON-like string
        obj_str = result["obj"]
        assert isinstance(obj_str, str)
        # Should contain the attribute values
        assert "Alice" in obj_str or "name" in obj_str
        assert "30" in obj_str or "age" in obj_str


def test_capture_object_attribute():
    """Test capturing specific object attributes."""
    program = compile("fn:test:entry { capture(name=obj.name, age=obj.age); }")

    with context.new_context():
        store = context.get_store()
        assert store is not None
        program_store = store.for_program("test-program")

        obj = SomeObj("Bob", 25)  # noqa
        frame = sys._getframe()

        result = execute_probe(program, program.probes[0], frame, program_store)

        assert result is not None
        assert result["name"] == "Bob"
        assert result["age"] == 25


def test_capture_multiple_objects():
    """Test capturing multiple objects."""
    program = compile("fn:test:entry { capture(obj1=obj1, obj2=obj2); }")

    with context.new_context():
        store = context.get_store()
        assert store is not None
        program_store = store.for_program("test-program")

        obj1 = SomeObj("Charlie", 35)  # noqa
        obj2 = SomeObj("Diana", 28)  # noqa
        frame = sys._getframe()

        result = execute_probe(program, program.probes[0], frame, program_store)

        assert result is not None
        assert "obj1" in result
        assert "obj2" in result
        # Both should be serialized
        assert isinstance(result["obj1"], str)
        assert isinstance(result["obj2"], str)


def test_store_and_retrieve_object():
    """Test storing object in request store and retrieving attributes."""
    program = compile("""
        fn:test:entry { $req.stored_obj = obj; }
        fn:test:exit { capture(name=$req.stored_obj.name); }
    """)

    with context.new_context():
        store = context.get_store()
        assert store is not None
        program_store = store.for_program("test-program")

        obj = SomeObj("Eve", 40)  # noqa
        frame = sys._getframe()

        # Store the object
        execute_probe(program, program.probes[0], frame, program_store)

        # Retrieve attribute from stored object
        result = execute_probe(program, program.probes[1], frame, program_store)

        assert result is not None
        assert result["name"] == "Eve"


def test_nested_objects():
    """Test capturing nested objects."""

    class Address:
        def __init__(self, city: str, country: str):
            self.city = city
            self.country = country

    class Person:
        def __init__(self, name: str, address: Address):
            self.name = name
            self.address = address

    program = compile("fn:test:entry { capture(person=person); }")

    with context.new_context():
        store = context.get_store()
        assert store is not None
        program_store = store.for_program("test-program")

        addr = Address("Paris", "France")
        person = Person("Frank", addr)  # noqa
        frame = sys._getframe()

        result = execute_probe(program, program.probes[0], frame, program_store)

        assert result is not None
        assert "person" in result
        # Should be serialized
        person_str = result["person"]
        assert isinstance(person_str, str)
        # Should contain nested data (best-effort)
        assert "Frank" in person_str or "name" in person_str


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
