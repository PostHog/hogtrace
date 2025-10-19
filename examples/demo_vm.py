#!/usr/bin/env python3
"""
Demo of the HogTrace VM.

Shows how to execute probes against running Python code.
"""

import sys
from pathlib import Path
import inspect
import time

# Add parent directory to path
sys.path.insert(0, str(Path(__file__).parent.parent))

import hogtrace
from hogtrace.vm import ProbeExecutor, ProgramExecutor
from hogtrace.request_store import RequestLocalStore, RequestContext


def demo_basic_execution():
    """Demo 1: Basic probe execution"""
    print("=" * 60)
    print("Demo 1: Basic Probe Execution")
    print("=" * 60)

    code = """
    fn:demo:entry
    / arg0 > 10 /
    {
        capture(value=arg0, doubled=arg0 * 2);
    }
    """

    program = hogtrace.parse(code)
    probe = program.probes[0]
    store = RequestLocalStore()
    executor = ProbeExecutor(probe, store)

    def test_function(x):
        frame = inspect.currentframe()
        result = executor.execute(frame)
        return result

    # Should fire
    result = test_function(15)
    print(f"test_function(15): {result}")

    # Should not fire (predicate fails)
    result = test_function(5)
    print(f"test_function(5): {result}")

    print()


def demo_request_tracking():
    """Demo 2: Request-level variable tracking"""
    print("=" * 60)
    print("Demo 2: Request Tracking")
    print("=" * 60)

    code = """
    fn:request_start:entry
    {
        $req.request_id = arg0;
        $req.start_time = timestamp();
        $req.call_count = 0;
        capture(request_id=$req.request_id);
    }

    fn:db_query:entry
    {
        $req.call_count = $req.call_count + 1;
        capture(
            request_id=$req.request_id,
            query=arg0,
            call_num=$req.call_count
        );
    }

    fn:request_end:exit
    {
        capture(
            request_id=$req.request_id,
            duration=timestamp() - $req.start_time,
            total_queries=$req.call_count,
            status=retval
        );
    }
    """

    program = hogtrace.parse(code)
    store = RequestLocalStore()

    # Create executors for each probe
    start_executor = ProbeExecutor(program.probes[0], store)
    query_executor = ProbeExecutor(program.probes[1], store)
    end_executor = ProbeExecutor(program.probes[2], store)

    # Simulate a request
    def request_start(request_id):
        frame = inspect.currentframe()
        result = start_executor.execute(frame)
        print(f"Request started: {result}")

    def db_query(sql):
        frame = inspect.currentframe()
        result = query_executor.execute(frame)
        print(f"DB query: {result}")

    def request_end():
        frame = inspect.currentframe()
        result = end_executor.execute(frame, retval="200 OK")
        print(f"Request ended: {result}")
        return "200 OK"

    # Simulate request lifecycle
    with RequestContext(store):
        request_start("req-123")
        db_query("SELECT * FROM users")
        db_query("SELECT * FROM orders")
        db_query("SELECT * FROM products")
        request_end()

    print()


def demo_sampling():
    """Demo 3: Probabilistic sampling"""
    print("=" * 60)
    print("Demo 3: Sampling")
    print("=" * 60)

    code = """
    fn:high_traffic:entry
    / rand() < 0.2 /
    {
        capture(arg0);
    }
    """

    program = hogtrace.parse(code)
    probe = program.probes[0]
    store = RequestLocalStore()
    executor = ProbeExecutor(probe, store)

    def high_traffic_function(request_id):
        frame = inspect.currentframe()
        return executor.execute(frame)

    # Call many times
    fired = 0
    total = 100
    for i in range(total):
        result = high_traffic_function(f"req-{i}")
        if result:
            fired += 1

    print(f"Fired {fired}/{total} times (~20% expected)")
    print()


def demo_object_introspection():
    """Demo 4: Inspecting complex objects"""
    print("=" * 60)
    print("Demo 4: Object Introspection")
    print("=" * 60)

    code = """
    fn:process_user:entry
    / arg0.role == "admin" && len(arg0.permissions) > 5 /
    {
        capture(
            username=arg0.username,
            role=arg0.role,
            perm_count=len(arg0.permissions),
            first_perm=arg0.permissions[0]
        );
    }
    """

    program = hogtrace.parse(code)
    probe = program.probes[0]
    store = RequestLocalStore()
    executor = ProbeExecutor(probe, store)

    class User:
        def __init__(self, username, role, permissions):
            self.username = username
            self.role = role
            self.permissions = permissions

    def process_user(user):
        frame = inspect.currentframe()
        return executor.execute(frame)

    # Should fire
    admin = User("alice", "admin", ["read", "write", "delete", "admin", "manage", "create"])
    result = process_user(admin)
    print(f"Admin user: {result}")

    # Should not fire (not admin)
    regular = User("bob", "user", ["read", "write"])
    result = process_user(regular)
    print(f"Regular user: {result}")

    print()


def demo_exception_tracking():
    """Demo 5: Exception tracking"""
    print("=" * 60)
    print("Demo 5: Exception Tracking")
    print("=" * 60)

    code = """
    fn:risky_operation:exit
    / exception != None /
    {
        capture(
            error_type=str(exception),
            args=args
        );
    }
    """

    program = hogtrace.parse(code)
    probe = program.probes[0]
    store = RequestLocalStore()
    executor = ProbeExecutor(probe, store)

    def risky_operation(value):
        try:
            if value < 0:
                raise ValueError("Negative value not allowed")
            return value * 2
        except Exception as e:
            frame = inspect.currentframe()
            result = executor.execute(frame, exception=e)
            print(f"Exception captured: {result}")
            raise

    # Normal execution
    print(f"risky_operation(10) = {risky_operation(10)}")

    # Exception
    try:
        risky_operation(-5)
    except ValueError:
        pass

    print()


def demo_program_executor():
    """Demo 6: Executing multiple probes"""
    print("=" * 60)
    print("Demo 6: Program Executor (Multiple Probes)")
    print("=" * 60)

    code = """
    fn:calculate:entry
    {
        capture(input_a=arg0, input_b=arg1);
    }

    fn:calculate:entry
    / arg0 > 100 /
    {
        capture(warning="Large input detected");
    }

    fn:calculate:exit
    {
        capture(result=retval);
    }
    """

    program = hogtrace.parse(code)
    store = RequestLocalStore()
    program_executor = ProgramExecutor(program, store)

    def calculate(a, b):
        # Entry probes
        frame = inspect.currentframe()
        entry_results = program_executor.execute_all(frame)
        for probe_spec, data in entry_results:
            print(f"  Entry probe fired: {data}")

        # Do calculation
        result = a + b

        # Exit probes
        exit_results = program_executor.execute_all(frame, retval=result)
        for probe_spec, data in exit_results:
            print(f"  Exit probe fired: {data}")

        return result

    print("calculate(10, 20):")
    calculate(10, 20)

    print("\ncalculate(150, 50):")
    calculate(150, 50)

    print()


def main():
    """Run all demos"""
    print("\n" + "=" * 60)
    print("HogTrace VM Demo")
    print("=" * 60)
    print()

    demo_basic_execution()
    demo_request_tracking()
    demo_sampling()
    demo_object_introspection()
    demo_exception_tracking()
    demo_program_executor()

    print("=" * 60)
    print("Demo Complete!")
    print("=" * 60)


if __name__ == "__main__":
    main()
