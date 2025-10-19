"""
Test suite for HogTrace parser.

These tests validate the grammar and parser functionality.
"""

import hogtrace


def test_basic_entry_probe():
    """Test basic entry probe"""
    code = """
    fn:myapp.users.create_user:entry
    {
        capture(args);
    }
    """
    program = hogtrace.parse(code)
    assert len(program.probes) == 1


def test_exit_probe_with_predicate():
    """Test exit probe with predicate"""
    code = """
    fn:myapp.users.create_user:exit
    / exception == None /
    {
        capture(retval);
    }
    """
    program = hogtrace.parse(code)
    assert len(program.probes) == 1
    assert program.probes[0].predicate is not None


def test_request_scoped_variables():
    """Test request-scoped variables"""
    code = """
    fn:myapp.api.handler:entry
    {
        $req.user_id = arg0.id;
        $req.start_time = timestamp();
        capture(user_id=$req.user_id);
    }
    """
    program = hogtrace.parse(code)
    assert len(program.probes) == 1


def test_sampling_percentage():
    """Test sampling with percentage"""
    code = """
    fn:myapp.api.high_traffic:entry
    {
        sample 10%;
        capture(args);
    }
    """
    program = hogtrace.parse(code)
    assert len(program.probes) == 1


def test_predicate_based_sampling():
    """Test predicate-based sampling"""
    code = """
    fn:myapp.api.endpoint:entry
    / rand() < 0.1 /
    {
        capture(args);
    }
    """
    program = hogtrace.parse(code)
    assert len(program.probes) == 1


def test_wildcard_probing():
    """Test wildcard matching"""
    code = """
    fn:myapp.api.*:entry
    {
        capture(args);
    }
    """
    program = hogtrace.parse(code)
    assert len(program.probes) == 1


def test_line_offset_probe():
    """Test line offset probe"""
    code = """
    fn:myapp.function:entry+10
    {
        capture(locals);
    }
    """
    program = hogtrace.parse(code)
    assert len(program.probes) == 1


def test_complex_nested_access():
    """Test complex nested expressions"""
    code = """
    fn:myapp.process:entry
    / len(args) > 2 && arg0.data[0]["value"] >= 100 /
    {
        capture(
            count=len(args),
            first_value=arg0.data[0]["value"]
        );
    }
    """
    program = hogtrace.parse(code)
    assert len(program.probes) == 1


def test_multiple_probes():
    """Test multiple probes"""
    code = """
    fn:myapp.a:entry { capture(args); }
    fn:myapp.b:entry { capture(args); }
    fn:myapp.c:entry { capture(args); }
    """
    program = hogtrace.parse(code)
    assert len(program.probes) == 3


def test_send_alias():
    """Test send() alias for capture()"""
    code = """
    fn:myapp.track:entry
    {
        send(args, kwargs);
    }
    """
    program = hogtrace.parse(code)
    assert len(program.probes) == 1
