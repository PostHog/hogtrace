#!/usr/bin/env python3
"""
Demo of the clean HogTrace API.

This shows how easy it is to use HogTrace programmatically.
"""

import sys
from pathlib import Path

# Add parent directory to path so we can import hogtrace
sys.path.insert(0, str(Path(__file__).parent.parent))

import hogtrace
from hogtrace.ast import ActionType, ExpressionType


def demo_basic_usage():
    """Basic usage of the parse API"""
    print("=" * 60)
    print("Demo 1: Basic Usage")
    print("=" * 60)

    code = """
    fn:myapp.users.create_user:entry
    / arg0.role == "admin" /
    {
        capture(args);
    }
    """

    program = hogtrace.parse(code)

    print(f"✓ Parsed {len(program.probes)} probe(s)\n")

    for probe in program.probes:
        print(f"Probe: {probe.spec.module_function}:{probe.spec.probe_point}")
        if probe.predicate:
            print(f"  Has predicate: {probe.predicate.expression}")
        print(f"  Actions: {len(probe.actions)}")
        for action in probe.actions:
            print(f"    - {action}")


def demo_request_tracking():
    """Request-level tracking example"""
    print("\n" + "=" * 60)
    print("Demo 2: Request Tracking")
    print("=" * 60)

    code = """
    # Track request start
    fn:django.core.handlers.wsgi.WSGIHandler:entry
    {
        $req.request_id = arg0.META["REQUEST_ID"];
        $req.start_time = timestamp();
    }

    # Track DB queries
    fn:myapp.db.execute_query:entry
    / $req.request_id != None /
    {
        capture(query=$req.request_id, sql=arg0);
    }

    # Track completion
    fn:django.core.handlers.wsgi.WSGIHandler:exit
    {
        capture(
            request_id=$req.request_id,
            duration=timestamp() - $req.start_time,
            status=retval.status_code
        );
    }
    """

    program = hogtrace.parse(code)

    print(f"✓ Parsed {len(program.probes)} probe(s)\n")

    for i, probe in enumerate(program.probes, 1):
        print(f"\nProbe #{i}: {probe.spec}")

        # Count different action types
        captures = sum(1 for a in probe.actions if a.type == ActionType.CAPTURE)
        assignments = sum(1 for a in probe.actions if a.type == ActionType.ASSIGNMENT)
        samples = sum(1 for a in probe.actions if a.type == ActionType.SAMPLE)

        print(f"  Actions: {captures} captures, {assignments} assignments, {samples} samples")

        if probe.predicate:
            print(f"  Predicate: {probe.predicate.expression}")


def demo_file_parsing():
    """Parse from file"""
    print("\n" + "=" * 60)
    print("Demo 3: File Parsing")
    print("=" * 60)

    try:
        program = hogtrace.parse_file("tests/test_examples.hogtrace")
        print(f"✓ Loaded {len(program.probes)} probes from tests/test_examples.hogtrace\n")

        # Show stats
        with_predicates = sum(1 for p in program.probes if p.predicate)
        with_sampling = sum(1 for p in program.probes
                          if any(a.type == ActionType.SAMPLE for a in p.actions))

        print(f"Statistics:")
        print(f"  Total probes: {len(program.probes)}")
        print(f"  With predicates: {with_predicates}")
        print(f"  With sampling: {with_sampling}")

        # Show probe types
        entry_probes = sum(1 for p in program.probes if 'entry' in p.spec.probe_point)
        exit_probes = sum(1 for p in program.probes if 'exit' in p.spec.probe_point)

        print(f"\nProbe points:")
        print(f"  Entry probes: {entry_probes}")
        print(f"  Exit probes: {exit_probes}")

    except FileNotFoundError:
        print("  (tests/test_examples.hogtrace not found - skipping)")


def demo_programmatic_access():
    """Access probe details programmatically"""
    print("\n" + "=" * 60)
    print("Demo 4: Programmatic Access")
    print("=" * 60)

    code = """
    fn:myapp.api.*:entry
    / rand() < 0.1 /
    {
        sample 10%;
        capture(
            user=$req.user_id,
            endpoint=arg0,
            timestamp=timestamp()
        );
    }
    """

    program = hogtrace.parse(code)
    probe = program.probes[0]

    print("Probe details:")
    print(f"  Provider: {probe.spec.provider}")
    print(f"  Module/Function: {probe.spec.module_function}")
    print(f"  Probe point: {probe.spec.probe_point}")
    print(f"  Full spec: {probe.spec.full_spec}")

    if probe.predicate:
        expr = probe.predicate.expression
        print(f"\nPredicate:")
        print(f"  Expression type: {expr.type}")
        print(f"  Raw: {expr.raw}")

    print(f"\nActions:")
    for action in probe.actions:
        print(f"  Type: {action.type.value}")

        if action.type == ActionType.CAPTURE:
            print(f"    Function: {action.function}")
            print(f"    Positional args: {len(action.arguments)}")
            print(f"    Named args: {list(action.named_arguments.keys())}")

        elif action.type == ActionType.SAMPLE:
            print(f"    Spec: {action.spec}")
            print(f"    Is percentage: {action.is_percentage}")
            print(f"    Value: {action.value}")


def demo_iteration():
    """Iterate over program elements"""
    print("\n" + "=" * 60)
    print("Demo 5: Iteration")
    print("=" * 60)

    code = """
    fn:myapp.a:entry { capture(args); }
    fn:myapp.b:entry { capture(args); }
    fn:myapp.c:entry { capture(args); }
    """

    program = hogtrace.parse(code)

    # Programs are iterable
    print("Iterating over probes:")
    for probe in program:
        print(f"  - {probe.spec}")

    # Can use indexing
    print(f"\nFirst probe: {program[0].spec}")
    print(f"Last probe: {program[-1].spec}")

    # Can get length
    print(f"Total: {len(program)} probes")


def demo_error_handling():
    """Error handling"""
    print("\n" + "=" * 60)
    print("Demo 6: Error Handling")
    print("=" * 60)

    bad_code = """
    fn:myapp.test:entry
    / this is invalid syntax /
    {
        invalid_function(args)
    }
    """

    print("Attempting to parse invalid code...")
    try:
        program = hogtrace.parse(bad_code)
    except hogtrace.ParseError as e:
        print(f"✓ Caught ParseError:\n  {e}")


def main():
    """Run all demos"""
    print("\n" + "=" * 60)
    print("HogTrace API Demo")
    print("=" * 60)

    demo_basic_usage()
    demo_request_tracking()
    demo_file_parsing()
    demo_programmatic_access()
    demo_iteration()
    demo_error_handling()

    print("\n" + "=" * 60)
    print("Demo Complete!")
    print("=" * 60)


if __name__ == "__main__":
    main()
