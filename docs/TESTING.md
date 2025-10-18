# HogTrace Grammar Testing

This document describes the testing performed on the HogTrace grammar.

## Test Results

All grammar tests **PASSED** ✓

## Files

- **`HogTrace.g4`** - The ANTLR4 grammar definition
- **`test_examples.hogtrace`** - 20 comprehensive test cases covering all language features
- **`test_parser.py`** - Automated Python test suite (10 tests)
- **Generated files**:
  - `HogTraceLexer.py` - Lexer implementation
  - `HogTraceParser.py` - Parser implementation
  - `HogTraceListener.py` - Parse tree listener (for AST traversal)

## Running Tests

### Using antlr4-parse (command-line)

Test a single example:
```bash
uv run antlr4-parse HogTrace.g4 program -tree <<'EOF'
fn:myapp.test:entry
{
    capture(args);
}
EOF
```

Test all examples:
```bash
uv run antlr4-parse HogTrace.g4 program -tree < test_examples.hogtrace
```

### Using Python test suite

```bash
uv run python test_parser.py
```

Expected output:
```
Running HogTrace parser tests...

✓ PASS: Basic entry probe
✓ PASS: Exit probe with predicate
✓ PASS: Request-scoped variables
✓ PASS: Sampling with percentage
✓ PASS: Predicate-based sampling
✓ PASS: Wildcard probing
✓ PASS: Line offset probe
✓ PASS: Complex nested access
✓ PASS: Multiple probes
✓ PASS: Send alias

==================================================
Results: 10 passed, 0 failed
==================================================
```

## Test Coverage

The test suite validates:

### 1. Probe Definitions
- ✓ Basic entry probes: `fn:module.function:entry`
- ✓ Exit probes: `fn:module.function:exit`
- ✓ Line offsets: `fn:module.function:entry+10`
- ✓ Wildcard matching: `fn:module.*:entry`

### 2. Predicates (Guards)
- ✓ Simple conditions: `/ arg0 == "test" /`
- ✓ Compound conditions: `/ exception == None && retval["status"] == "success" /`
- ✓ Logical operators: `&&`, `||`, `!`
- ✓ Comparison operators: `==`, `!=`, `<`, `>`, `<=`, `>=`
- ✓ Arithmetic operators: `+`, `-`, `*`, `/`, `%`

### 3. Data Access
- ✓ Field access: `arg0.field`
- ✓ Dictionary access: `arg0["key"]`
- ✓ List indexing: `args[0]`
- ✓ Nested access: `arg0.data[0]["value"]`

### 4. Request-Scoped Variables
- ✓ Assignment: `$req.user_id = arg0.id`
- ✓ Reading: `capture($req.user_id)`
- ✓ Both forms: `$req.var` and `$request.var`

### 5. Sampling
- ✓ Percentage: `sample 10%`
- ✓ Ratio: `sample 1/100`
- ✓ Predicate-based: `/ rand() < 0.1 /`

### 6. Capture Statements
- ✓ Positional args: `capture(args, kwargs)`
- ✓ Named args: `capture(user=arg0, id=$req.user_id)`
- ✓ Mixed: `capture(args, user_id=$req.user_id)`
- ✓ Predefined: `capture(args)`, `capture(locals)`, `capture(globals)`
- ✓ Send alias: `send(args)`

### 7. Built-in Functions
- ✓ `timestamp()`
- ✓ `rand()`
- ✓ `len(obj)`

### 8. Literals
- ✓ Integers: `42`, `100`
- ✓ Floats: `0.1`, `3.14`
- ✓ Strings: `"admin"`, `'test'`
- ✓ Booleans: `True`, `False`
- ✓ None: `None`

### 9. Multi-probe Programs
- ✓ Multiple probe definitions in a single file
- ✓ Request tracking across probes

### 10. Comments
- ✓ Line comments: `# comment`
- ✓ Block comments: `/* comment */`

## Example Test Cases

### Basic Probe
```dtrace
fn:myapp.users.create_user:entry
{
    capture(args);
}
```

### With Predicate
```dtrace
fn:myapp.auth.check:entry
/ arg0 == "admin" /
{
    capture(args);
}
```

### Request Tracking
```dtrace
fn:myapp.start:entry
{
    $req.start_time = timestamp();
}

fn:myapp.end:exit
{
    capture(duration=timestamp() - $req.start_time);
}
```

### Complex Nested Access
```dtrace
fn:myapp.process:entry
/ len(args) > 2 && arg0.data[0]["value"] >= 100 /
{
    capture(
        count=len(args),
        first_value=arg0.data[0]["value"]
    );
}
```

## Grammar Features Validated

✓ All lexer tokens recognized correctly
✓ All parser rules parse without errors
✓ Operator precedence working as expected
✓ Expression nesting handled properly
✓ Both `capture()` and `send()` primitives work
✓ Both `$req.*` and `$request.*` forms work
✓ Comments properly ignored
✓ Whitespace properly handled

## Next Steps

The grammar is production-ready. To use it:

1. **Generate parsers**: `uv run antlr4 -Dlanguage=Python3 HogTrace.g4`
2. **Implement AST visitor**: Create a visitor class to walk the parse tree
3. **Build runtime**: Implement bytecode injection and probe management
4. **Integrate with PostHog**: Wire up the `capture()` primitive

## Regenerating Parser Files

If you modify `HogTrace.g4`, regenerate the parser with:

```bash
uv run antlr4 -Dlanguage=Python3 HogTrace.g4
```

This will update:
- `HogTraceLexer.py`
- `HogTraceParser.py`
- `HogTraceListener.py`
- Token files (`.tokens`, `.interp`)
