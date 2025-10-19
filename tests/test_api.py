"""
Test suite for the HogTrace API.
"""

import hogtrace
from hogtrace.ast import (
    ActionType, ExpressionType,
    CaptureAction, AssignmentAction, SampleAction
)


def test_basic_parse():
    """Test basic parsing"""
    code = """
    fn:myapp.test:entry
    {
        capture(args);
    }
    """

    program = hogtrace.parse(code)
    assert len(program) == 1
    assert program.probes[0].spec.module_function == "myapp.test"
    assert program.probes[0].spec.probe_point == "entry"


def test_with_predicate():
    """Test probe with predicate"""
    code = """
    fn:myapp.test:entry
    / arg0 == "admin" /
    {
        capture(args);
    }
    """

    program = hogtrace.parse(code)
    assert program[0].predicate is not None
    assert "admin" in program[0].predicate.expression.raw


def test_multiple_probes():
    """Test multiple probes"""
    code = """
    fn:myapp.a:entry { capture(args); }
    fn:myapp.b:exit { capture(retval); }
    """

    program = hogtrace.parse(code)
    assert len(program) == 2
    assert program[0].spec.probe_point == "entry"
    assert program[1].spec.probe_point == "exit"


def test_request_variables():
    """Test request-scoped variables"""
    code = """
    fn:myapp.test:entry
    {
        $req.user_id = arg0.id;
        capture(user=$req.user_id);
    }
    """

    program = hogtrace.parse(code)
    actions = program[0].actions

    assert len(actions) == 2
    assert isinstance(actions[0], AssignmentAction)
    assert actions[0].variable.name == "user_id"

    assert isinstance(actions[1], CaptureAction)
    assert "user" in actions[1].named_arguments


def test_sampling():
    """Test sampling directives"""
    code = """
    fn:myapp.test:entry
    {
        sample 10%;
        capture(args);
    }
    """

    program = hogtrace.parse(code)
    actions = program[0].actions

    assert len(actions) == 2
    assert isinstance(actions[0], SampleAction)
    assert actions[0].is_percentage
    assert actions[0].value == 0.1


def test_capture_variants():
    """Test different capture styles"""
    code = """
    fn:myapp.test:entry
    {
        capture(args);
        capture(arg0, arg1);
        capture(user=arg0, id=arg1);
        send(retval);
    }
    """

    program = hogtrace.parse(code)
    actions = program[0].actions

    assert len(actions) == 4
    assert all(isinstance(a, CaptureAction) for a in actions)

    # Positional args
    assert len(actions[1].arguments) == 2

    # Named args
    assert "user" in actions[2].named_arguments
    assert "id" in actions[2].named_arguments

    # Send alias
    assert actions[3].function == "send"


def test_wildcards():
    """Test wildcard matching"""
    code = """
    fn:myapp.api.*:entry
    {
        capture(args);
    }
    """

    program = hogtrace.parse(code)
    assert "*" in program[0].spec.module_function


def test_line_offsets():
    """Test line offsets"""
    code = """
    fn:myapp.test:entry+10
    {
        capture(locals);
    }
    """

    program = hogtrace.parse(code)
    assert "+10" in program[0].spec.probe_point


def test_parse_file():
    """Test parsing from file"""
    try:
        program = hogtrace.parse_file("tests/test_examples.hogtrace")
        assert len(program) == 22
    except FileNotFoundError:
        pass  # Skip if file not found


def test_iteration():
    """Test program iteration"""
    code = """
    fn:a:entry { capture(args); }
    fn:b:entry { capture(args); }
    fn:c:entry { capture(args); }
    """

    program = hogtrace.parse(code)

    # Iterate
    count = 0
    for probe in program:
        count += 1

    assert count == 3

    # Indexing
    assert program[0].spec.module_function == "a"
    assert program[-1].spec.module_function == "c"

    # Length
    assert len(program) == 3



def test_error_handling():
    """Test error handling"""
    bad_code = "fn:test:entry { invalid }"

    try:
        hogtrace.parse(bad_code)
        assert False, "Should have raised ParseError"
    except hogtrace.ParseError:
        pass



def test_complex_expressions():
    """Test complex nested expressions"""
    code = """
    fn:myapp.test:entry
    / len(args) > 2 && arg0.data[0]["value"] >= 100 /
    {
        capture(count=len(args));
    }
    """

    program = hogtrace.parse(code)
    assert program[0].predicate is not None

    actions = program[0].actions
    assert len(actions) == 1
    assert "count" in actions[0].named_arguments
