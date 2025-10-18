# HogTrace API Reference

Complete reference for the HogTrace Python API.

## Overview

The HogTrace API provides a clean, Pythonic interface for parsing and working with HogTrace programs. All ANTLR complexity is hidden behind simple functions and dataclasses.

## Functions

### `hogtrace.parse(code: str) -> Program`

Parse HogTrace code and return a Program object.

**Parameters:**
- `code` (str): HogTrace source code

**Returns:**
- `Program`: Parsed program with probes

**Raises:**
- `ParseError`: If code has syntax errors

**Example:**
```python
import hogtrace

program = hogtrace.parse("""
fn:myapp.test:entry
{ capture(args); }
""")
```

---

### `hogtrace.parse_file(file_path: str | Path) -> Program`

Parse a HogTrace file.

**Parameters:**
- `file_path` (str | Path): Path to .hogtrace file

**Returns:**
- `Program`: Parsed program with probes

**Raises:**
- `ParseError`: If code has syntax errors
- `FileNotFoundError`: If file doesn't exist

**Example:**
```python
program = hogtrace.parse_file("traces.hogtrace")
```

---

## Classes

### `Program`

A complete HogTrace program (collection of probes).

**Attributes:**
- `probes: list[Probe]` - List of probes in the program

**Methods:**
- `__len__()` - Number of probes
- `__iter__()` - Iterate over probes
- `__getitem__(index)` - Index into probes
- `__str__()` - String representation

**Example:**
```python
program = hogtrace.parse(code)

# Iterate
for probe in program:
    print(probe)

# Index
first = program[0]
last = program[-1]

# Length
count = len(program)
```

---

### `Probe`

A single probe definition.

**Attributes:**
- `spec: ProbeSpec` - Probe specification
- `predicate: Optional[Predicate]` - Guard condition (may be None)
- `actions: list[Action]` - List of actions to execute

**Example:**
```python
probe = program.probes[0]
print(probe.spec.module_function)  # "myapp.users.create"
print(probe.spec.probe_point)      # "entry"

if probe.predicate:
    print(probe.predicate.expression)

for action in probe.actions:
    print(action)
```

---

### `ProbeSpec`

Probe specification (provider:module.function:probe_point).

**Attributes:**
- `provider: str` - Provider name ("fn", "py")
- `module_function: str` - Module and function path
- `probe_point: str` - Probe point ("entry", "exit", "entry+10")
- `full_spec: str` - Complete specification string

**Example:**
```python
spec = probe.spec
print(spec.provider)           # "fn"
print(spec.module_function)    # "myapp.users.create_user"
print(spec.probe_point)        # "entry"
print(spec.full_spec)          # "fn:myapp.users.create_user:entry"
```

---

### `Predicate`

Predicate (guard condition).

**Attributes:**
- `expression: Expression` - The predicate expression

**Example:**
```python
if probe.predicate:
    print(probe.predicate.expression)  # "arg0 == 'admin'"
```

---

### `Action` (Base Class)

Base class for all actions.

**Attributes:**
- `type: ActionType` - Type of action (CAPTURE, ASSIGNMENT, SAMPLE)

**Subclasses:**
- `CaptureAction`
- `AssignmentAction`
- `SampleAction`

---

### `CaptureAction`

Capture/send action.

**Attributes:**
- `type: ActionType` - Always `ActionType.CAPTURE`
- `function: str` - Function name ("capture" or "send")
- `arguments: list[Expression]` - Positional arguments
- `named_arguments: dict[str, Expression]` - Named arguments

**Example:**
```python
for action in probe.actions:
    if action.type == ActionType.CAPTURE:
        print(f"Function: {action.function}")
        print(f"Positional: {action.arguments}")
        print(f"Named: {action.named_arguments}")
```

---

### `AssignmentAction`

Assignment to request-scoped variable.

**Attributes:**
- `type: ActionType` - Always `ActionType.ASSIGNMENT`
- `variable: RequestVar` - Variable being assigned to
- `value: Expression` - Value being assigned

**Example:**
```python
for action in probe.actions:
    if action.type == ActionType.ASSIGNMENT:
        print(f"{action.variable} = {action.value}")
```

---

### `SampleAction`

Sampling directive.

**Attributes:**
- `type: ActionType` - Always `ActionType.SAMPLE`
- `spec: str` - Sampling spec ("10%", "1/100")
- `is_percentage: bool` - True if percentage, False if ratio
- `value: Optional[float]` - Sampling rate (0.0-1.0)
- `numerator: Optional[int]` - For ratio sampling
- `denominator: Optional[int]` - For ratio sampling

**Example:**
```python
for action in probe.actions:
    if action.type == ActionType.SAMPLE:
        print(f"Sample rate: {action.value}")  # 0.1 for 10%
```

---

### `Expression` (Base Class)

Base class for all expressions.

**Attributes:**
- `type: ExpressionType` - Type of expression
- `raw: str` - Raw text representation

**Subclasses:**
- `Literal` - int, float, string, bool, None
- `Identifier` - Variable names
- `FieldAccess` - obj.field
- `IndexAccess` - obj[index]
- `FunctionCall` - func(args)
- `RequestVar` - $req.var
- `BinaryOp` - a + b, a == b, etc.
- `UnaryOp` - !expr

**Example:**
```python
expr = probe.predicate.expression
print(expr.type)  # ExpressionType.BINARY_OP
print(expr.raw)   # "arg0 == 'admin'"
```

---

## Enums

### `ActionType`

Type of action.

**Values:**
- `CAPTURE` - capture() or send()
- `ASSIGNMENT` - $req.var = value
- `SAMPLE` - sample directive

**Example:**
```python
from hogtrace.ast import ActionType

if action.type == ActionType.CAPTURE:
    print("This is a capture action")
```

---

### `ExpressionType`

Type of expression.

**Values:**
- `LITERAL` - Literal value
- `IDENTIFIER` - Variable name
- `FIELD_ACCESS` - Object field access
- `INDEX_ACCESS` - Array/dict index
- `FUNCTION_CALL` - Function call
- `REQUEST_VAR` - Request variable
- `BINARY_OP` - Binary operation
- `UNARY_OP` - Unary operation

**Example:**
```python
from hogtrace.ast import ExpressionType

if expr.type == ExpressionType.BINARY_OP:
    print(f"Binary op: {expr.operator}")
```

---

## Exceptions

### `ParseError`

Exception raised when parsing fails.

**Attributes:**
- Inherits from `Exception`
- Contains error message with line/column information

**Example:**
```python
try:
    program = hogtrace.parse(code)
except hogtrace.ParseError as e:
    print(f"Parse error: {e}")
```

---

## Complete Example

```python
import hogtrace
from hogtrace.ast import ActionType, ExpressionType

# Parse code
code = """
fn:django.core.handlers.wsgi.WSGIHandler:entry
{
    $req.request_id = arg0.META["REQUEST_ID"];
    $req.start_time = timestamp();
}

fn:myapp.db.execute_query:entry
/ $req.request_id != None /
{
    sample 10%;
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

# Analyze program
print(f"Total probes: {len(program)}")

for i, probe in enumerate(program, 1):
    print(f"\nProbe #{i}")
    print(f"  Module: {probe.spec.module_function}")
    print(f"  Point: {probe.spec.probe_point}")

    if probe.predicate:
        print(f"  Predicate: {probe.predicate.expression}")

    # Count actions by type
    captures = sum(1 for a in probe.actions if a.type == ActionType.CAPTURE)
    assignments = sum(1 for a in probe.actions if a.type == ActionType.ASSIGNMENT)
    samples = sum(1 for a in probe.actions if a.type == ActionType.SAMPLE)

    print(f"  Actions: {captures} captures, {assignments} assignments, {samples} samples")

    # Show capture details
    for action in probe.actions:
        if action.type == ActionType.CAPTURE:
            print(f"    Capture: {action.function}()")
            if action.named_arguments:
                print(f"      Named args: {list(action.named_arguments.keys())}")
```

---

## Type Hints

All classes have complete type hints for IDE autocomplete and type checking:

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
    Literal,
    Identifier,
    FieldAccess,
    IndexAccess,
    FunctionCall,
    RequestVar,
    BinaryOp,
    UnaryOp,
    ActionType,
    ExpressionType,
)
```

---

## Tips

1. **Check action types**: Always check `action.type` before accessing type-specific attributes

2. **Predicates can be None**: Check `if probe.predicate:` before accessing

3. **Programs are iterable**: Use `for probe in program` instead of `for probe in program.probes`

4. **Expressions preserve raw text**: Use `expression.raw` to get original syntax

5. **Error messages are helpful**: `ParseError` includes line and column numbers
