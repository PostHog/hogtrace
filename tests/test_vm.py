"""
Test suite for HogTrace VM.
"""

import inspect

import hogtrace
from hogtrace.request_store import RequestLocalStore, RequestContext
from hogtrace.vm import ProbeExecutor
from hogtrace.frame import FrameContext


def test_basic_capture():
    """Test basic variable capture"""
    code = """
    fn:test:entry
    {
        capture(arg0, arg1);
    }
    """

    program = hogtrace.parse(code)
    probe = program.probes[0]
    store = RequestLocalStore()
    executor = ProbeExecutor(probe, store)

    # Create a test function
    def test_func(a, b):
        frame = inspect.currentframe()
        return executor.execute(frame)

    result = test_func(10, 20)
    assert result is not None
    assert result['arg0'] == 10
    assert result['arg1'] == 20


def test_predicate():
    """Test predicate evaluation"""
    code = """
    fn:test:entry
    / arg0 > 10 /
    {
        capture(arg0);
    }
    """

    program = hogtrace.parse(code)
    probe = program.probes[0]
    store = RequestLocalStore()
    executor = ProbeExecutor(probe, store)

    def test_func(x):
        frame = inspect.currentframe()
        return executor.execute(frame)

    # Should fire
    result = test_func(15)
    assert result is not None
    assert result['arg0'] == 15

    # Should not fire
    result = test_func(5)
    assert result is None



def test_request_variables():
    """Test request-scoped variables"""
    code = """
    fn:test:entry
    {
        $req.count = $req.count + 1;
        capture(count=$req.count);
    }
    """

    program = hogtrace.parse(code)
    probe = program.probes[0]
    store = RequestLocalStore()
    store.set("count", 0)
    executor = ProbeExecutor(probe, store)

    def test_func():
        frame = inspect.currentframe()
        return executor.execute(frame)

    # First call
    result = test_func()
    assert result is not None
    assert result['count'] == 1

    # Second call
    result = test_func()
    assert result is not None
    assert result['count'] == 2



def test_field_access():
    """Test object field access"""
    code = """
    fn:test:entry
    {
        capture(name=arg0.name, value=arg0.value);
    }
    """

    program = hogtrace.parse(code)
    probe = program.probes[0]
    store = RequestLocalStore()
    executor = ProbeExecutor(probe, store)

    class TestObject:
        def __init__(self):
            self.name = "test"
            self.value = 42

    def test_func(obj):
        frame = inspect.currentframe()
        return executor.execute(frame)

    result = test_func(TestObject())
    assert result is not None
    assert result['name'] == "test"
    assert result['value'] == 42



def test_index_access():
    """Test dict/list indexing"""
    code = """
    fn:test:entry
    {
        capture(
            first=arg0[0],
            key=arg1["key"]
        );
    }
    """

    program = hogtrace.parse(code)
    probe = program.probes[0]
    store = RequestLocalStore()
    executor = ProbeExecutor(probe, store)

    def test_func(lst, dct):
        frame = inspect.currentframe()
        return executor.execute(frame)

    result = test_func([1, 2, 3], {"key": "value"})
    assert result is not None
    assert result['first'] == 1
    assert result['key'] == "value"



def test_sampling():
    """Test sampling directives"""
    code = """
    fn:test:entry
    {
        sample 50%;
        capture(arg0);
    }
    """

    program = hogtrace.parse(code)
    probe = program.probes[0]
    store = RequestLocalStore()
    executor = ProbeExecutor(probe, store)

    def test_func(x):
        frame = inspect.currentframe()
        return executor.execute(frame)

    # Run multiple times, some should fire, some shouldn't
    fired = 0
    trials = 100
    for i in range(trials):
        result = test_func(i)
        if result is not None:
            fired += 1

    # With 50% sampling, expect around 50 fires (with some variance)
    assert 30 < fired < 70, f"Expected ~50 fires, got {fired}"



def test_builtin_functions():
    """Test built-in functions"""
    code = """
    fn:test:entry
    {
        capture(
            length=len(arg0),
            str_val=str(arg1),
            int_val=int(arg2)
        );
    }
    """

    program = hogtrace.parse(code)
    probe = program.probes[0]
    store = RequestLocalStore()
    executor = ProbeExecutor(probe, store)

    def test_func(lst, num, text):
        frame = inspect.currentframe()
        return executor.execute(frame)

    result = test_func([1, 2, 3], 42, "314")
    assert result is not None
    assert result['length'] == 3
    assert result['str_val'] == "42"
    assert result['int_val'] == 314



def test_exit_probe():
    """Test exit probe with return value"""
    code = """
    fn:test:exit
    {
        capture(result=retval);
    }
    """

    program = hogtrace.parse(code)
    probe = program.probes[0]
    store = RequestLocalStore()
    executor = ProbeExecutor(probe, store)

    def test_func():
        return 42

    # Get frame and return value
    result_value = test_func()
    frame = inspect.currentframe()
    result = executor.execute(frame, retval=result_value)

    assert result is not None
    assert result['result'] == 42



def test_exception_probe():
    """Test exit probe with exception"""
    code = """
    fn:test:exit
    / exception != None /
    {
        capture(error=exception);
    }
    """

    program = hogtrace.parse(code)
    probe = program.probes[0]
    store = RequestLocalStore()
    executor = ProbeExecutor(probe, store)

    frame = inspect.currentframe()

    # With exception
    exc = ValueError("test error")
    result = executor.execute(frame, exception=exc)
    assert result is not None
    assert result['error'] == exc

    # Without exception
    result = executor.execute(frame, exception=None)
    assert result is None



def test_request_context():
    """Test RequestContext manager"""
    store = RequestLocalStore()

    # Set value outside context
    store.set("outside", 1)

    with RequestContext(store):
        # Set value inside context
        store.set("inside", 2)
        assert store.get("inside") == 2

    # Inside value should be cleared
    assert store.get("inside") is None



def test_complex_expression():
    """Test complex nested expressions"""
    code = """
    fn:test:entry
    / len(arg0) > 2 && arg0[0]["value"] >= 100 /
    {
        capture(
            first_value=arg0[0]["value"],
            count=len(arg0)
        );
    }
    """

    program = hogtrace.parse(code)
    probe = program.probes[0]
    store = RequestLocalStore()
    executor = ProbeExecutor(probe, store)

    def test_func(data):
        frame = inspect.currentframe()
        return executor.execute(frame)

    # Should fire
    data = [{"value": 100}, {"value": 200}, {"value": 300}]
    result = test_func(data)
    assert result is not None
    assert result['first_value'] == 100
    assert result['count'] == 3

    # Should not fire (too few items)
    data = [{"value": 100}]
    result = test_func(data)
    assert result is None

    # Should not fire (value too low)
    data = [{"value": 50}, {"value": 60}, {"value": 70}]
    result = test_func(data)
    assert result is None



def test_special_captures():
    """Test special capture names (args, locals, etc)"""
    code = """
    fn:test:entry
    {
        capture(args, locals);
    }
    """

    program = hogtrace.parse(code)
    probe = program.probes[0]
    store = RequestLocalStore()
    executor = ProbeExecutor(probe, store)

    def test_func(a, b, c):
        x = 10
        y = 20
        frame = inspect.currentframe()
        return executor.execute(frame)

    result = test_func(1, 2, 3)
    assert result is not None
    assert result['args'] == (1, 2, 3)
    assert 'a' in result['locals']
    assert 'x' in result['locals']
    assert 'y' in result['locals']
