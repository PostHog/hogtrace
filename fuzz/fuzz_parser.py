#!/usr/bin/env python3
"""
AFL fuzz target for HogTrace parser.

This script reads input from stdin and attempts to parse it.
AFL will mutate inputs to find crashes and edge cases.

Usage:
    py-afl-fuzz -i corpus -o findings -- python fuzz/fuzz_parser.py
"""

import sys
import os

# Add parent directory to path
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import afl
import hogtrace

# Initialize AFL (must be after all imports)
afl.init()

# Read input from stdin
input_data = sys.stdin.buffer.read()

try:
    # Attempt to decode and parse
    code = input_data.decode('utf-8', errors='ignore')
    program = hogtrace.parse(code)

    # Optional: Exercise the parsed program to find more bugs
    # This helps find bugs in the AST structure
    for probe in program.probes:
        _ = str(probe.spec)
        if probe.predicate:
            _ = str(probe.predicate)
        for action in probe.actions:
            _ = str(action)

except hogtrace.ParseError:
    # Expected - invalid syntax
    pass
except UnicodeDecodeError:
    # Expected - invalid UTF-8
    pass
except Exception as e:
    # Unexpected exception - AFL will catch this as a crash
    # Re-raise to let AFL detect it
    raise

# Fast exit (skip Python cleanup for speed)
os._exit(0)
