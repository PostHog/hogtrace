# HogTrace

A DTrace-inspired language for runtime instrumentation of Python web applications. HogTrace enables lightweight, production-safe observability by injecting probes into running applications with minimal overhead.

## Overview

HogTrace is designed for web application observability with features like:
- **Sampling** - Control overhead with percentage or predicate-based sampling
- **Request-scoped variables** - Track state across function calls within a request
- **Python-native** - First-class support for dicts, objects, lists, and attributes
- **PostHog integration** - All trace data sent to PostHog for analysis

## Quick Example

```dtrace
# Track request lifecycle
fn:django.core.handlers.wsgi.WSGIHandler:entry
{
    $req.request_id = arg0.META["REQUEST_ID"];
    $req.start_time = timestamp();
}

# Sample 10% of database queries
fn:myapp.db.execute_query:entry
/ rand() < 0.1 /
{
    capture(query=$req.request_id, sql=arg0);
}

# Capture request completion
fn:django.core.handlers.wsgi.WSGIHandler:exit
{
    capture(
        request_id=$req.request_id,
        duration=timestamp() - $req.start_time,
        status=retval.status_code
    );
}
```

## Project Status

**Phase 1: Language Design & Parsing**  COMPLETE

- [x] Language specification
- [x] ANTLR4 grammar
- [x] Parser implementation
- [x] Comprehensive test suite

**Phase 2: Runtime Implementation** =� TODO

- [ ] AST builder
- [ ] Bytecode injection
- [ ] Probe management
- [ ] Request context tracking
- [ ] PostHog integration

## Documentation

- **[QUICKSTART.md](QUICKSTART.md)** - Quick start guide for the API ⭐
- **[SPEC.md](SPEC.md)** - Complete language specification
- **[TESTING.md](TESTING.md)** - Grammar testing and validation
- **[test_examples.hogtrace](test_examples.hogtrace)** - 20+ example programs
- **[demo_api.py](demo_api.py)** - Comprehensive API examples

## Installation

```bash
# Install dependencies
uv sync

# Generate parser files (already done)
uv run antlr4 -Dlanguage=Python3 HogTrace.g4
```

## Usage

### Python API (Recommended)

```python
import hogtrace

# Parse HogTrace code
code = """
fn:myapp.users.create_user:entry
/ arg0.role == "admin" /
{
    capture(args);
}
"""

program = hogtrace.parse(code)

# Access probes
for probe in program.probes:
    print(f"Probe: {probe.spec}")
    print(f"Predicate: {probe.predicate}")
    print(f"Actions: {probe.actions}")

# Parse from file
program = hogtrace.parse_file("traces.hogtrace")
```

See **[QUICKSTART.md](QUICKSTART.md)** for complete API documentation.

### CLI Tool

```bash
# Parse and display probes
uv run python main.py parse traces.hogtrace

# Validate syntax
uv run python main.py validate traces.hogtrace

# Evaluate code
uv run python main.py eval "fn:app.test:entry { capture(args); }"
```

### Testing the Grammar

```bash
# Run automated tests
uv run python test_parser.py

# Run API demo
uv run python demo_api.py

# Test with antlr4-parse (low-level)
uv run antlr4-parse HogTrace.g4 program -tree < test_examples.hogtrace
```

## Language Features

### Probe Points

```dtrace
fn:module.function:entry          # Function entry
fn:module.function:exit            # Function exit
fn:module.function:entry+10        # Line offset (bytecode)
fn:module.*:entry                  # Wildcard matching
```

### Predicates (Guards)

```dtrace
/ arg0 == "admin" /                        # Simple condition
/ exception == None /                      # Exit probes
/ rand() < 0.1 /                           # Sampling
/ len(args) > 2 && arg0.count >= 100 /    # Complex conditions
```

### Data Access

```dtrace
arg0.field                    # Object attribute
arg0["key"]                   # Dictionary access
args[0]                       # List indexing
arg0.data[0]["value"]         # Nested access
```

### Request-Scoped Variables

```dtrace
$req.user_id = arg0.id                    # Assignment
$request.start_time = timestamp()         # Both forms work
capture(user=$req.user_id)                # Reading
```

### Sampling

```dtrace
sample 10%;                   # Percentage
sample 1/100;                 # Ratio
/ rand() < 0.1 /              # Predicate-based (recommended)
```

### Capture Data

```dtrace
capture(args)                             # All arguments
capture(locals)                           # All local variables
capture(globals)                          # All globals (use sparingly)
capture(arg0, arg1, retval)              # Specific variables
capture(user=arg0, status=retval.code)   # Named fields
send(args)                               # Alias for capture()
```

### Built-in Functions

- `timestamp()` - Current Unix timestamp
- `rand()` - Random float 0.0-1.0
- `len(obj)` - Length of object
- `str(obj)`, `int(obj)`, `float(obj)` - Type conversion

### Predefined Variables

**Entry probes:**
- `args` - All positional arguments (tuple)
- `arg0`, `arg1`, ... - Individual arguments
- `kwargs` - Keyword arguments (dict)
- `self` - For method calls

**Exit probes:**
- All entry variables (still accessible)
- `retval` - Return value
- `exception` - Exception object if raised, else None

## Example Programs

### Basic Function Tracing

```dtrace
fn:myapp.users.create_user:entry
{
    capture(args);
}

fn:myapp.users.create_user:exit
{
    capture(retval);
}
```

### Conditional Tracing

```dtrace
fn:myapp.auth.check_permission:entry
/ arg0 == "admin" /
{
    capture(args);
}
```

### Exception Tracking

```dtrace
fn:myapp.payments.process_payment:exit
/ exception != None /
{
    capture(args=args, exception=exception, user_id=$req.user_id);
}
```

### High-Traffic Sampling

```dtrace
fn:myapp.api.list_products:entry
/ rand() < 0.01 /  # Sample 1%
{
    capture(args);
}
```

## Testing

All tests pass 

```
Running HogTrace parser tests...

 PASS: Basic entry probe
 PASS: Exit probe with predicate
 PASS: Request-scoped variables
 PASS: Sampling with percentage
 PASS: Predicate-based sampling
 PASS: Wildcard probing
 PASS: Line offset probe
 PASS: Complex nested access
 PASS: Multiple probes
 PASS: Send alias

==================================================
Results: 10 passed, 0 failed
==================================================
```

## Grammar Files

- `HogTrace.g4` - ANTLR4 grammar definition
- `HogTraceLexer.py` - Generated lexer
- `HogTraceParser.py` - Generated parser
- `HogTraceListener.py` - Generated parse tree listener

## Next Steps

To complete HogTrace, the following components need implementation:

1. **AST Builder** - Convert parse tree to abstract syntax tree
2. **Bytecode Injection** - Inject probes into Python bytecode
3. **Probe Manager** - Install, enable, disable probes dynamically
4. **Request Context** - Track request-scoped variables (thread-locals or contextvars)
5. **PostHog Client** - Send captured data to PostHog
6. **CLI Tool** - Load and activate HogTrace programs
7. **Framework Integration** - Hooks for Django, Flask, FastAPI

## Contributing

This is the initial implementation of the HogTrace language parser. The grammar is stable and tested.

## License

Part of the PostHog ecosystem.

## References

- [DTrace](http://dtrace.org/) - Inspiration for probe syntax
- [ANTLR4](https://www.antlr.org/) - Parser generator
- [PostHog](https://posthog.com/) - Observability backend
