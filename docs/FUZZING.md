# Fuzzing HogTrace Parser

This document describes how to use AFL (American Fuzzy Lop) to fuzz test the HogTrace parser for bugs and edge cases.

## Overview

We use [python-afl](https://github.com/jwilk/python-afl) to perform coverage-guided fuzzing of the HogTrace parser. Fuzzing helps discover:

- Parser crashes on malformed input
- Unhandled edge cases
- Performance issues (infinite loops, stack overflows)
- Unicode handling bugs
- Memory safety issues

## Prerequisites

### Install AFL

**macOS:**
```bash
brew install afl-fuzz
```

**Linux (Debian/Ubuntu):**
```bash
sudo apt-get install afl afl++
```

### Install Python Dependencies

```bash
uv add --dev python-afl cython
```

### macOS-Specific Setup

On macOS, you **must** disable the crash reporter before fuzzing, or AFL will abort with an error. The crash reporter intercepts crashes and prevents AFL from detecting them properly.

**Disable crash reporter (required before fuzzing):**
```bash
SL=/System/Library; PL=com.apple.ReportCrash
launchctl unload -w ${SL}/LaunchAgents/${PL}.plist
sudo launchctl unload -w ${SL}/LaunchDaemons/${PL}.Root.plist
```

**Re-enable crash reporter (after fuzzing):**
```bash
SL=/System/Library; PL=com.apple.ReportCrash
launchctl load -w ${SL}/LaunchAgents/${PL}.plist
sudo launchctl load -w ${SL}/LaunchDaemons/${PL}.Root.plist
```

The `fuzz_start.sh` script will check if the crash reporter is enabled and provide these instructions if needed.

## Fuzzing Targets

We provide two fuzzing targets:

1. **`fuzz/fuzz_parser.py`**: Basic mode - spawns new process per input (slower, simpler)
2. **`fuzz/fuzz_parser_persistent.py`**: Persistent mode - processes 1000 inputs per spawn (10-100x faster)

**Persistent mode is recommended** for faster fuzzing campaigns.

## Seed Corpus

The `fuzz/corpus/` directory contains seed inputs that cover different HogTrace features:

- `01_basic.hogtrace` - Simple probe with capture
- `02_predicate.hogtrace` - Probe with predicate guard
- `03_request_vars.hogtrace` - Request-scoped variables
- `04_sampling.hogtrace` - Sampling directive
- `05_complex.hogtrace` - Complex nested expressions
- `06_multi_probe.hogtrace` - Multiple probes with state

These seeds guide AFL toward interesting code paths.

## Running Fuzzing Campaigns

### Start a New Campaign

```bash
./scripts/fuzz_start.sh
```

This will:
- Check that AFL and python-afl are installed
- Verify the seed corpus exists
- Start fuzzing with persistent mode
- Save findings to `fuzz/findings/`

**Important**: The fuzzer will run indefinitely until you press `Ctrl+C`. Let it run for at least several hours for meaningful results.

### Resume an Existing Campaign

If you stopped fuzzing and want to continue:

```bash
./scripts/fuzz_resume.sh
```

This resumes from the existing `fuzz/findings/` directory, preserving all discovered test cases.

### Reproduce a Crash

When AFL finds a crash, it saves the input in `fuzz/findings/crashes/`:

```bash
./scripts/fuzz_reproduce.sh fuzz/findings/crashes/id:000000,sig:06,src:000000,op:havoc,rep:4
```

This will:
- Show the crashing input (first 500 bytes)
- Run the parser without AFL instrumentation
- Display the actual exception/error

## Understanding AFL Output

When fuzzing runs, you'll see the AFL status screen:

```
┌─ process timing ────────────────────────┬─ overall results ─────┐
│        run time : 0 days, 1 hrs, 23 min │  cycles done : 12     │
│   last new path : 0 days, 0 hrs, 2 min  │  total paths : 847    │
│ last uniq crash : none seen yet         │ uniq crashes : 0      │
│  last uniq hang : none seen yet         │   uniq hangs : 0      │
├─ cycle progress ────────────────────────┴───────────────────────┤
│  now processing : 412 (48.6%)                                   │
│ path favorites : 89 (10.5%)                                     │
├─ map coverage ──────────────────────────────────────────────────┤
│    map density : 2.41% / 4.13%                                  │
│ count coverage : 1.89 bits/tuple                                │
├─ stage progress ────────────────────────────────────────────────┤
│  now trying : havoc                                             │
│ stage execs : 1847/2048 (90.19%)                                │
│ total execs : 287k                                              │
│  exec speed : 412/sec                                           │
└─────────────────────────────────────────────────────────────────┘
```

**Key metrics:**

- **total paths**: Unique code paths discovered (higher is better)
- **uniq crashes**: Number of unique crashes found (0 is ideal for a stable parser)
- **uniq hangs**: Inputs that caused timeouts
- **exec speed**: Executions per second (persistent mode gives 10-100x speedup)
- **cycles done**: Number of full queue passes (more cycles = more thorough)

## What the Fuzzer Tests

The fuzzing targets exercise:

1. **Parsing**: Call `hogtrace.parse()` on mutated inputs
2. **AST traversal**: Access program structure (`program.probes`, etc.)
3. **Repr methods**: Call `repr()` on all probe objects

**Expected errors** (caught and ignored):
- `ParseError` - Invalid syntax
- `UnicodeDecodeError` - Invalid UTF-8

**Unexpected errors** (will trigger crash detection):
- Assertion failures
- Segmentation faults
- Uncaught exceptions
- Infinite loops (detected as hangs)

## Performance Tips

### Use Persistent Mode

Persistent mode (`fuzz_parser_persistent.py`) processes 1000 inputs before restarting:

```python
while afl.loop(1000):
    # Process input
    ...
```

This gives **10-100x speedup** over basic mode.

### Run Multiple Instances

AFL supports parallel fuzzing. Run multiple instances to use all CPU cores:

**Master instance:**
```bash
.venv/bin/py-afl-fuzz -i fuzz/corpus -o fuzz/findings -M fuzzer1 -m none -- .venv/bin/python fuzz/fuzz_parser_persistent.py
```

**Secondary instances** (in separate terminals):
```bash
.venv/bin/py-afl-fuzz -i fuzz/corpus -o fuzz/findings -S fuzzer2 -m none -- .venv/bin/python fuzz/fuzz_parser_persistent.py
.venv/bin/py-afl-fuzz -i fuzz/corpus -o fuzz/findings -S fuzzer3 -m none -- .venv/bin/python fuzz/fuzz_parser_persistent.py
```

### Minimize Corpus

After fuzzing, minimize the corpus to remove redundant test cases:

```bash
afl-cmin -i fuzz/findings/default/queue -o fuzz/corpus_minimized -- .venv/bin/python fuzz/fuzz_parser.py
```

## Interpreting Results

### No Crashes (Expected)

If AFL runs for hours without finding crashes:
- **Good news**: The parser is robust
- Keep running longer for edge cases
- Consider adding more complex seeds

### Crashes Found

If crashes are found:

1. **Reproduce** with `./scripts/fuzz_reproduce.sh`
2. **Analyze** the exception and input
3. **Fix** the parser bug
4. **Add test case** to prevent regression
5. **Resume fuzzing** to find more issues

### Hangs Found

Hangs indicate infinite loops or extreme slowness:

1. Reproduce with the hang input
2. Profile with a debugger
3. Add recursion limits or complexity bounds
4. Consider adding timeouts to the parser

## Continuous Fuzzing

For ongoing security:

1. **Run regularly**: Fuzz after significant parser changes
2. **Integrate with CI**: Run short fuzzing sessions in CI/CD
3. **Expand corpus**: Add new seed files for new features
4. **Track coverage**: Monitor code coverage improvements

## Troubleshooting

### "afl-fuzz not found"

Install AFL via your package manager (see Prerequisites).

### "python-afl not installed"

Run: `uv add --dev python-afl cython`

### "Crash reporter detected" (macOS)

You must disable macOS's crash reporter before fuzzing. See the **macOS-Specific Setup** section above.

### "No instrumentation detected"

Make sure you're using `.venv/bin/py-afl-fuzz` (not `afl-fuzz` directly, and not through `uv run`). The wrapper sets up Python instrumentation.

### "WARNING: Target binary called without a prefixed path"

This warning can be ignored if you're using the provided scripts. It appears when AFL sees `uv` instead of `python` as the target. Our scripts now call `.venv/bin/python` directly to avoid this.

### Very slow execution speed

- Ensure you're using persistent mode (`fuzz_parser_persistent.py`)
- Check if AFL is running in QEMU mode (slower)
- Reduce timeout with `-t` flag if inputs are slow
- On macOS, AFL may be slower than on Linux due to platform differences

### AFL complains about CPU frequency scaling (Linux only)

On Linux, fix with:
```bash
cd /sys/devices/system/cpu
echo performance | sudo tee cpu*/cpufreq/scaling_governor
```

This warning does not apply to macOS and is automatically suppressed in the fuzzing scripts.

## References

- [AFL Documentation](https://github.com/google/AFL)
- [python-afl GitHub](https://github.com/jwilk/python-afl)
- [AFL persistent mode](https://github.com/google/AFL/blob/master/llvm_mode/README.persistent_mode.md)
- [Fuzzing strategies](https://github.com/google/fuzzing/blob/master/tutorial/structure-aware-fuzzing.md)

## Next Steps

Consider extending fuzzing to:

1. **VM executor**: Fuzz `ProbeExecutor.execute()` with random frames
2. **Expression evaluator**: Test edge cases in expression evaluation
3. **Structure-aware fuzzing**: Use grammar-based input generation
4. **Differential fuzzing**: Compare with other parsers
