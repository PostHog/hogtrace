"""
Tests for JSON serialization/deserialization of HogTrace AST.
"""

import json
import pytest

import hogtrace
from hogtrace.serialization import (
    serialize_expression, deserialize_expression,
    serialize_action, deserialize_action,
    serialize_probe, deserialize_probe,
    serialize_program, deserialize_program,
    program_to_json, program_from_json
)


def test_serialize_simple_program():
    """Test serializing a simple program."""
    code = """
    fn:myapp.users.create:entry
    {
        capture(arg0, arg1);
    }
    """
    program = hogtrace.parse(code)

    # Serialize to dict
    data = serialize_program(program)

    assert data["version"] == "0.1.0"
    assert len(data["probes"]) == 1

    probe_data = data["probes"][0]
    assert probe_data["spec"]["provider"] == "fn"
    assert probe_data["spec"]["module_function"] == "myapp.users.create"
    assert probe_data["spec"]["probe_point"] == "entry"
    assert probe_data["predicate"] is None
    assert len(probe_data["actions"]) == 1

    action_data = probe_data["actions"][0]
    assert action_data["type"] == "capture"
    assert action_data["function"] == "capture"
    assert len(action_data["arguments"]) == 2


def test_deserialize_simple_program():
    """Test deserializing a simple program."""
    code = """
    fn:myapp.users.create:entry
    {
        capture(arg0, arg1);
    }
    """
    original = hogtrace.parse(code)

    # Serialize and deserialize
    data = serialize_program(original)
    restored = deserialize_program(data)

    # Check structure matches
    assert len(restored.probes) == 1
    assert restored.probes[0].spec.provider == "fn"
    assert restored.probes[0].spec.module_function == "myapp.users.create"
    assert restored.probes[0].spec.probe_point == "entry"
    assert restored.probes[0].predicate is None
    assert len(restored.probes[0].actions) == 1


def test_roundtrip_with_predicate():
    """Test roundtrip with a predicate."""
    code = """
    fn:test:entry
    / arg0 > 10 && arg1 == "test" /
    {
        capture(arg0, arg1);
    }
    """
    original = hogtrace.parse(code)

    # Roundtrip
    data = serialize_program(original)
    restored = deserialize_program(data)

    assert restored.probes[0].predicate is not None
    # Check the predicate expression structure
    pred_expr = restored.probes[0].predicate.expression
    assert pred_expr.type.value == "binary_op"
    assert pred_expr.operator == "&&"


def test_roundtrip_with_request_variables():
    """Test roundtrip with request-scoped variables."""
    code = """
    fn:test:entry
    {
        $req.user_id = arg0;
        $req.count = $req.count + 1;
        capture(user_id=$req.user_id, count=$req.count);
    }
    """
    original = hogtrace.parse(code)

    # Roundtrip
    data = serialize_program(original)
    restored = deserialize_program(data)

    assert len(restored.probes[0].actions) == 3

    # Check assignment actions
    action1 = restored.probes[0].actions[0]
    assert action1.type.value == "assignment"
    assert action1.variable.name == "user_id"
    assert action1.variable.prefix == "req"

    # Check capture with named arguments
    capture_action = restored.probes[0].actions[2]
    assert capture_action.type.value == "capture"
    assert "user_id" in capture_action.named_arguments
    assert "count" in capture_action.named_arguments


def test_roundtrip_with_field_access():
    """Test roundtrip with field access expressions."""
    code = """
    fn:test:entry
    {
        capture(
            name=arg0.user.name,
            email=arg0.user.email
        );
    }
    """
    original = hogtrace.parse(code)

    # Roundtrip
    data = serialize_program(original)
    restored = deserialize_program(data)

    capture_action = restored.probes[0].actions[0]
    assert "name" in capture_action.named_arguments

    # Check field access structure
    name_expr = capture_action.named_arguments["name"]
    assert name_expr.type.value == "field_access"
    assert name_expr.field == "name"


def test_roundtrip_with_index_access():
    """Test roundtrip with index access expressions."""
    code = """
    fn:test:entry
    {
        capture(
            first=arg0[0],
            key=arg1["key"]
        );
    }
    """
    original = hogtrace.parse(code)

    # Roundtrip
    data = serialize_program(original)
    restored = deserialize_program(data)

    capture_action = restored.probes[0].actions[0]

    # Check index access with numeric index
    first_expr = capture_action.named_arguments["first"]
    assert first_expr.type.value == "index_access"

    # Check index access with string key
    key_expr = capture_action.named_arguments["key"]
    assert key_expr.type.value == "index_access"


def test_roundtrip_with_function_calls():
    """Test roundtrip with function calls."""
    code = """
    fn:test:entry
    {
        capture(
            length=len(arg0),
            time=timestamp(),
            str_val=str(arg1)
        );
    }
    """
    original = hogtrace.parse(code)

    # Roundtrip
    data = serialize_program(original)
    restored = deserialize_program(data)

    capture_action = restored.probes[0].actions[0]

    # Check function calls
    length_expr = capture_action.named_arguments["length"]
    assert length_expr.type.value == "function_call"
    assert length_expr.function == "len"
    assert len(length_expr.arguments) == 1

    time_expr = capture_action.named_arguments["time"]
    assert time_expr.type.value == "function_call"
    assert time_expr.function == "timestamp"
    assert len(time_expr.arguments) == 0


def test_roundtrip_with_sampling():
    """Test roundtrip with sampling."""
    code = """
    fn:test:entry
    {
        sample 50%;
        capture(arg0);
    }
    """
    original = hogtrace.parse(code)

    # Roundtrip
    data = serialize_program(original)
    restored = deserialize_program(data)

    # Check sampling action
    sample_action = restored.probes[0].actions[0]
    assert sample_action.type.value == "sample"
    assert sample_action.is_percentage is True
    assert sample_action.value == 0.5


def test_roundtrip_with_complex_expressions():
    """Test roundtrip with complex nested expressions."""
    code = """
    fn:test:entry
    / len(arg0.items) > 2 && arg0.user.active == true /
    {
        capture(
            count=len(arg0.items),
            total=arg0.items[0].price + arg0.items[1].price
        );
    }
    """
    original = hogtrace.parse(code)

    # Roundtrip
    data = serialize_program(original)
    restored = deserialize_program(data)

    # Check predicate
    assert restored.probes[0].predicate is not None
    pred_expr = restored.probes[0].predicate.expression
    assert pred_expr.type.value == "binary_op"

    # Check capture expressions
    capture_action = restored.probes[0].actions[0]
    count_expr = capture_action.named_arguments["count"]
    assert count_expr.type.value == "function_call"


def test_roundtrip_multiple_probes():
    """Test roundtrip with multiple probes."""
    code = """
    fn:test:entry
    {
        $req.start = timestamp();
    }

    fn:test:exit
    {
        capture(duration=timestamp() - $req.start);
    }
    """
    original = hogtrace.parse(code)

    # Roundtrip
    data = serialize_program(original)
    restored = deserialize_program(data)

    assert len(restored.probes) == 2
    assert restored.probes[0].spec.probe_point == "entry"
    assert restored.probes[1].spec.probe_point == "exit"


def test_program_to_json_string():
    """Test converting program to JSON string."""
    code = """
    fn:test:entry
    {
        capture(arg0);
    }
    """
    program = hogtrace.parse(code)

    # Convert to JSON string
    json_str = program_to_json(program)

    # Should be valid JSON
    data = json.loads(json_str)
    assert data["version"] == "0.1.0"
    assert len(data["probes"]) == 1


def test_program_from_json_string():
    """Test converting JSON string back to program."""
    code = """
    fn:test:entry
    / arg0 > 10 /
    {
        capture(arg0);
    }
    """
    original = hogtrace.parse(code)

    # Convert to JSON string and back
    json_str = program_to_json(original)
    restored = program_from_json(json_str)

    # Check structure
    assert len(restored.probes) == 1
    assert restored.probes[0].predicate is not None
    assert len(restored.probes[0].actions) == 1


def test_json_compact_vs_pretty():
    """Test compact vs pretty JSON formatting."""
    code = """
    fn:test:entry
    {
        capture(arg0);
    }
    """
    program = hogtrace.parse(code)

    # Compact JSON
    compact = program_to_json(program, indent=None)
    assert "\n" not in compact  # No newlines in compact

    # Pretty JSON
    pretty = program_to_json(program, indent=2)
    assert "\n" in pretty  # Has newlines
    assert len(pretty) > len(compact)  # Pretty is longer

    # Both should deserialize to the same thing
    from_compact = program_from_json(compact)
    from_pretty = program_from_json(pretty)

    assert len(from_compact.probes) == len(from_pretty.probes)


def test_roundtrip_preserves_execution():
    """Test that roundtrip preserves execution behavior."""
    import inspect
    from hogtrace import ProbeExecutor, RequestLocalStore

    code = """
    fn:test:entry
    / arg0 > 10 /
    {
        capture(doubled=arg0 * 2);
    }
    """
    original = hogtrace.parse(code)

    # Roundtrip through JSON
    json_str = program_to_json(original)
    restored = program_from_json(json_str)

    # Execute both versions
    store = RequestLocalStore()

    original_executor = ProbeExecutor(original.probes[0], store)
    restored_executor = ProbeExecutor(restored.probes[0], store)

    def test_func(value):
        frame = inspect.currentframe()
        return original_executor.execute(frame), restored_executor.execute(frame)

    # Both should fire with same result
    result_orig, result_restored = test_func(15)

    assert result_orig is not None
    assert result_restored is not None
    assert result_orig["doubled"] == result_restored["doubled"]
    assert result_orig["doubled"] == 30

    # Both should not fire
    result_orig, result_restored = test_func(5)
    assert result_orig is None
    assert result_restored is None


def test_roundtrip_all_literal_types():
    """Test roundtrip with all literal value types."""
    code = """
    fn:test:entry
    {
        capture(
            int_val=42,
            float_val=3.14,
            str_val="hello"
        );
    }
    """
    original = hogtrace.parse(code)

    # Roundtrip
    json_str = program_to_json(original)
    restored = program_from_json(json_str)

    # Check all literals preserved
    capture_action = restored.probes[0].actions[0]

    int_expr = capture_action.named_arguments["int_val"]
    assert int_expr.value == 42

    float_expr = capture_action.named_arguments["float_val"]
    assert float_expr.value == 3.14

    str_expr = capture_action.named_arguments["str_val"]
    assert str_expr.value == "hello"


def test_roundtrip_unary_operators():
    """Test roundtrip with unary operators."""
    code = """
    fn:test:entry
    / !arg0 /
    {
        capture(value=arg1);
    }
    """
    original = hogtrace.parse(code)

    # Roundtrip
    data = serialize_program(original)
    restored = deserialize_program(data)

    # Check predicate has unary operator
    pred_expr = restored.probes[0].predicate.expression
    assert pred_expr.type.value == "unary_op"
    assert pred_expr.operator == "!"


def test_version_check():
    """Test that version checking works."""
    data = {
        "version": "99.99.99",
        "probes": []
    }

    # Should raise error for unknown version
    with pytest.raises(ValueError, match="Unsupported program version"):
        deserialize_program(data)


def test_serialize_empty_program():
    """Test serializing a program with no probes."""
    from hogtrace.ast import Program

    # Create empty program directly (parsing empty string fails)
    program = Program([])

    data = serialize_program(program)
    assert data["version"] == "0.1.0"
    assert data["probes"] == []

    # Roundtrip should work
    restored = deserialize_program(data)
    assert len(restored.probes) == 0


def test_storage_workflow():
    """Test the typical storage workflow."""
    # Step 1: User writes code in UI
    user_code = """
    fn:myapp.users.create:entry
    / arg0.role == "admin" /
    {
        capture(user_id=arg0.id, role=arg0.role);
    }
    """

    # Step 2: Backend parses and validates
    program = hogtrace.parse(user_code)

    # Step 3: Backend serializes to JSON for storage
    json_definition = program_to_json(program)

    # Step 4: Store in database
    # db.save_probe_definition(debug_session_id="uuid-123", definition=json_definition)

    # Step 5: Later, instrumentation library fetches from DB
    # json_definition = db.fetch_probe_definition("uuid-123")

    # Step 6: Deserialize and execute
    restored_program = program_from_json(json_definition)

    # Step 7: Create executor
    from hogtrace import ProgramExecutor, RequestLocalStore
    store = RequestLocalStore()
    executor = ProgramExecutor(restored_program, store)

    # Verify it works
    assert len(executor.executors) == 1
    assert executor.program.probes[0].spec.module_function == "myapp.users.create"
