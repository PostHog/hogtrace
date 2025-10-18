#!/usr/bin/env python3
"""Test script for HogTrace parser"""

from antlr4 import *
from HogTraceLexer import HogTraceLexer
from HogTraceParser import HogTraceParser


def parse_hogtrace(code: str) -> bool:
    """Parse HogTrace code and return True if successful"""
    try:
        input_stream = InputStream(code)
        lexer = HogTraceLexer(input_stream)
        stream = CommonTokenStream(lexer)
        parser = HogTraceParser(stream)

        # Disable error output for cleaner test results
        parser.removeErrorListeners()

        tree = parser.program()

        # Check if there were any syntax errors
        if parser.getNumberOfSyntaxErrors() > 0:
            return False

        return True
    except Exception as e:
        print(f"Parse error: {e}")
        return False


def test_examples():
    """Test various HogTrace examples"""

    tests = [
        (
            "Basic entry probe",
            """
            fn:myapp.users.create_user:entry
            {
                capture(args);
            }
            """
        ),
        (
            "Exit probe with predicate",
            """
            fn:myapp.users.create_user:exit
            / exception == None /
            {
                capture(retval);
            }
            """
        ),
        (
            "Request-scoped variables",
            """
            fn:myapp.api.handler:entry
            {
                $req.user_id = arg0.id;
                $req.start_time = timestamp();
                capture(user_id=$req.user_id);
            }
            """
        ),
        (
            "Sampling with percentage",
            """
            fn:myapp.api.endpoint:entry
            {
                sample 10%;
                capture(args);
            }
            """
        ),
        (
            "Predicate-based sampling",
            """
            fn:myapp.api.endpoint:entry
            / rand() < 0.1 /
            {
                capture(args);
            }
            """
        ),
        (
            "Wildcard probing",
            """
            fn:myapp.api.*:entry
            {
                capture(args);
            }
            """
        ),
        (
            "Line offset probe",
            """
            fn:myapp.function:entry+10
            {
                capture(locals);
            }
            """
        ),
        (
            "Complex nested access",
            """
            fn:myapp.process:entry
            / len(args) > 2 && arg0.data[0]["value"] >= 100 /
            {
                capture(
                    count=len(args),
                    first_value=arg0.data[0]["value"]
                );
            }
            """
        ),
        (
            "Multiple probes",
            """
            fn:myapp.start:entry
            {
                $req.start_time = timestamp();
            }

            fn:myapp.end:exit
            {
                capture(duration=timestamp() - $req.start_time);
            }
            """
        ),
        (
            "Send alias",
            """
            fn:myapp.track:entry
            {
                send(args, kwargs);
            }
            """
        ),
    ]

    passed = 0
    failed = 0

    print("Running HogTrace parser tests...\n")

    for name, code in tests:
        result = parse_hogtrace(code)
        status = "✓ PASS" if result else "✗ FAIL"
        print(f"{status}: {name}")

        if result:
            passed += 1
        else:
            failed += 1

    print(f"\n{'='*50}")
    print(f"Results: {passed} passed, {failed} failed")
    print(f"{'='*50}")

    return failed == 0


if __name__ == "__main__":
    import sys
    success = test_examples()
    sys.exit(0 if success else 1)
