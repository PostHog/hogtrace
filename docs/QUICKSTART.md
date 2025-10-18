# HogTrace Quick Start Guide

## Installation

```bash
uv sync
```

## Using the API

### Basic Usage

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
    if probe.predicate:
        print(f"  Predicate: {probe.predicate.expression}")
    for action in probe.actions:
        print(f"  Action: {action}")
```

### Parse from File

```python
import hogtrace

# Load from .hogtrace file
program = hogtrace.parse_file("traces.hogtrace")

print(f"Loaded {len(program)} probes")
```

### Access Probe Details

```python
program = hogtrace.parse(code)
probe = program.probes[0]

# Probe specification
print(probe.spec.provider)           # "fn"
print(probe.spec.module_function)    # "myapp.users.create_user"
print(probe.spec.probe_point)        # "entry"

# Predicate (optional)
if probe.predicate:
    print(probe.predicate.expression)

# Actions
for action in probe.actions:
    if action.type == ActionType.CAPTURE:
        print(f"Capture: {action.function}")
        print(f"Args: {action.arguments}")
        print(f"Named: {action.named_arguments}")
```

### Iterate Over Program

```python
# Programs are iterable
for probe in program:
    print(probe.spec)

# Use indexing
first_probe = program[0]
last_probe = program[-1]

# Get length
count = len(program)
```

### Error Handling

```python
try:
    program = hogtrace.parse(code)
except hogtrace.ParseError as e:
    print(f"Syntax error: {e}")
```

## Using the CLI

### Parse and Display

```bash
# Parse a file
uv run python main.py parse traces.hogtrace

# With verbose output
uv run python main.py parse traces.hogtrace -v
```

### Validate Syntax

```bash
uv run python main.py validate traces.hogtrace
```

### Evaluate Code

```bash
# Parse code from command line
uv run python main.py eval "fn:app.test:entry { capture(args); }"
```

## Program Structure

A `Program` contains:
- `probes` - List of `Probe` objects

Each `Probe` has:
- `spec` - `ProbeSpec` (provider, module_function, probe_point)
- `predicate` - Optional `Predicate` with expression
- `actions` - List of `Action` objects

Actions can be:
- `CaptureAction` - capture() or send()
- `AssignmentAction` - $req.var = value
- `SampleAction` - sample directive

## Example: Request Tracking

```python
import hogtrace
from hogtrace.ast import ActionType

code = """
fn:django.core.handlers.wsgi.WSGIHandler:entry
{
    $req.request_id = arg0.META["REQUEST_ID"];
    $req.start_time = timestamp();
}

fn:myapp.db.execute_query:entry
/ $req.request_id != None /
{
    capture(query=$req.request_id, sql=arg0);
}

fn:django.core.handlers.wsgi.WSGIHandler:exit
{
    capture(
        request_id=$req.request_id,
        duration=timestamp() - $req.start_time
    );
}
"""

program = hogtrace.parse(code)

print(f"Request tracking with {len(program)} probes")

for probe in program:
    # Count action types
    assignments = sum(1 for a in probe.actions if a.type == ActionType.ASSIGNMENT)
    captures = sum(1 for a in probe.actions if a.type == ActionType.CAPTURE)

    print(f"{probe.spec.module_function}:{probe.spec.probe_point}")
    print(f"  {assignments} assignments, {captures} captures")
```

## Type Hints

All classes have full type hints for IDE support:

```python
from hogtrace.ast import (
    Program,
    Probe,
    ProbeSpec,
    Predicate,
    Action,
    CaptureAction,
    AssignmentAction,
    SampleAction,
    Expression,
    ActionType,
    ExpressionType
)
```

## Next Steps

- **[SPEC.md](SPEC.md)** - Complete language specification
- **[README.md](README.md)** - Full documentation
- **[demo_api.py](demo_api.py)** - Comprehensive examples
- **[test_examples.hogtrace](test_examples.hogtrace)** - Example programs

## Tips

1. **Programs are iterable**: Use `for probe in program` instead of `for probe in program.probes`

2. **Error messages are helpful**: ParseError includes line/column information

3. **Check action types**: Use `action.type == ActionType.CAPTURE` to check action type

4. **Expressions preserve raw text**: Access `expression.raw` for original syntax

5. **Request vars have both forms**: Both `$req.*` and `$request.*` work

## Common Patterns

### Count Probe Types

```python
entry_probes = [p for p in program if 'entry' in p.spec.probe_point]
exit_probes = [p for p in program if 'exit' in p.spec.probe_point]
```

### Find Probes with Sampling

```python
sampled = [p for p in program
           if any(a.type == ActionType.SAMPLE for a in p.actions)]
```

### Extract All Module Names

```python
modules = {p.spec.module_function.split('.')[0] for p in program}
```

### Filter by Predicate

```python
guarded = [p for p in program if p.predicate is not None]
```
