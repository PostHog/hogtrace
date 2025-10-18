#!/usr/bin/env python3
"""
HogTrace CLI - Parse and validate HogTrace programs.
"""

import sys
import argparse
from pathlib import Path
import hogtrace
from hogtrace.ast import ActionType


def print_program(program: hogtrace.Program, verbose: bool = False):
    """Pretty-print a parsed program"""
    print(f"Found {len(program.probes)} probe(s)\n")

    for i, probe in enumerate(program.probes, 1):
        print(f"Probe #{i}: {probe.spec}")

        if probe.predicate:
            print(f"  Predicate: {probe.predicate.expression}")

        if probe.actions:
            print(f"  Actions ({len(probe.actions)}):")
            for action in probe.actions:
                if action.type == ActionType.CAPTURE:
                    print(f"    - {action}")
                elif action.type == ActionType.ASSIGNMENT:
                    print(f"    - {action}")
                elif action.type == ActionType.SAMPLE:
                    print(f"    - {action}")

        if verbose:
            print(f"\n  Raw probe:")
            for line in str(probe).split('\n'):
                print(f"    {line}")

        print()


def cmd_parse(args):
    """Parse a HogTrace file"""
    try:
        program = hogtrace.parse_file(args.file)
        print_program(program, verbose=args.verbose)
        return 0
    except FileNotFoundError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1
    except hogtrace.ParseError as e:
        print(f"Parse error: {e}", file=sys.stderr)
        return 1


def cmd_validate(args):
    """Validate HogTrace syntax"""
    try:
        program = hogtrace.parse_file(args.file)
        print(f"✓ Valid HogTrace syntax ({len(program.probes)} probes)")
        return 0
    except FileNotFoundError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1
    except hogtrace.ParseError as e:
        print(f"✗ Invalid syntax:\n{e}", file=sys.stderr)
        return 1


def cmd_eval(args):
    """Parse and evaluate HogTrace code from command line"""
    try:
        program = hogtrace.parse(args.code)
        print_program(program, verbose=args.verbose)
        return 0
    except hogtrace.ParseError as e:
        print(f"Parse error: {e}", file=sys.stderr)
        return 1


def main():
    parser = argparse.ArgumentParser(
        description="HogTrace - DTrace-inspired instrumentation for Python",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  hogtrace parse traces.hogtrace          Parse and display probes
  hogtrace validate traces.hogtrace       Validate syntax
  hogtrace eval "fn:app.test:entry { capture(args); }"   Evaluate code

For more information, see the documentation at:
  SPEC.md - Language specification
  TESTING.md - Testing guide
        """
    )

    subparsers = parser.add_subparsers(dest='command', help='Command to run')

    # Parse command
    parse_parser = subparsers.add_parser('parse', help='Parse a HogTrace file')
    parse_parser.add_argument('file', type=str, help='HogTrace file to parse')
    parse_parser.add_argument('-v', '--verbose', action='store_true',
                            help='Show detailed output')
    parse_parser.set_defaults(func=cmd_parse)

    # Validate command
    validate_parser = subparsers.add_parser('validate', help='Validate HogTrace syntax')
    validate_parser.add_argument('file', type=str, help='HogTrace file to validate')
    validate_parser.set_defaults(func=cmd_validate)

    # Eval command
    eval_parser = subparsers.add_parser('eval', help='Evaluate HogTrace code')
    eval_parser.add_argument('code', type=str, help='HogTrace code to evaluate')
    eval_parser.add_argument('-v', '--verbose', action='store_true',
                           help='Show detailed output')
    eval_parser.set_defaults(func=cmd_eval)

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        return 1

    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
