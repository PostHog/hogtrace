#!/usr/bin/env python3
"""
AFL fuzz target for HogTrace parser (persistent mode).

Persistent mode processes multiple inputs per process spawn,
which is 10-100x faster than regular mode.

Usage:
    py-afl-fuzz -i corpus -o findings -- python fuzz/fuzz_parser_persistent.py

Note: Requires afl-fuzz ≥ 1.82b and PYTHON_AFL_PERSISTENT environment variable
      (py-afl-fuzz sets this automatically)
"""

import sys
import os

# Add parent directory to path
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import afl
import hogtrace

# Persistent mode loop - process N inputs before restarting
# 1000 is a good balance between speed and stability
while afl.loop(1000):
    # Rewind stdin for next input
    sys.stdin.seek(0)

    # Read input from stdin
    input_data = sys.stdin.buffer.read()

    try:
        # Attempt to decode and parse
        code = input_data.decode('utf-8', errors='ignore')
        program = hogtrace.parse(code)

        # Exercise the parsed program
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
        # Unexpected exception - AFL will catch this
        raise

# Normal exit after loop completes
sys.exit(0)
