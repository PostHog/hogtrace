# HogTrace Language Specification v0.1

## Overview

HogTrace is a DTrace-inspired language for runtime instrumentation of Python web applications. Unlike DTrace's focus on C programs and system-level tracing, HogTrace is designed for web application observability with features like sampling, request-scoped variables, and integration with PostHog for trace collection.

## Design Principles

1. **Web-first**: Designed for web applications with request/response lifecycles
2. **Lightweight**: Support for sampling to minimize performance impact
3. **Python-native**: First-class support for Python objects, dicts, lists, and attributes
4. **Observable**: All trace data is sent to PostHog, not stdout
5. **DTrace-inspired**: Familiar syntax for those who know DTrace

---

## 1. Probe Definition Syntax

### Basic Structure

```
provider:module.function:probe-point
/ predicate /
{
    action;
    action;
}
```

### Providers

- **`fn`**: Function-level probes (entry, exit, line offsets)
- **`py`**: General Python probes (future: exceptions, allocations, etc.)

### Probe Points

#### Function Entry
```
fn:myapp.users.create_user:entry
```

#### Function Exit
```
fn:myapp.users.create_user:exit
```

#### Line Offsets (relative to function start)
```
fn:myapp.users.create_user:entry+5
```
Probes at line 5 after function entry (bytecode offset, not source line).

#### Wildcard Matching
```
fn:myapp.users.*:entry        # All functions in myapp.users module
fn:myapp.*.create_*:entry     # Pattern matching across modules
```

---

## 2. Predicates (Guards)

Predicates filter when probe actions execute. They appear between `/` delimiters.

### Syntax
```
/ condition /
```

### Available Variables in Predicates

#### Entry Probes
- `args` - tuple of all positional arguments
- `arg0, arg1, arg2, ...` - individual positional arguments
- `kwargs` - dict of keyword arguments
- `self` - for method calls
- `$req.*` - request-scoped variables (set in previous probes)

#### Exit Probes
- All entry variables (args still accessible)
- `retval` - the return value
- `exception` - exception object if function raised (None otherwise)

### Predicate Examples

```dtrace
# Only trace when first argument is "admin"
fn:myapp.auth.check_permission:entry
/ arg0 == "admin" /
{
    capture(args);
}

# Only trace successful returns (no exception)
fn:myapp.db.query:exit
/ exception == None /
{
    capture(retval);
}

# Guard based on return value
fn:myapp.payments.process:exit
/ retval["status"] == "success" /
{
    capture(retval);
}

# Guard based on request-scoped variable
fn:myapp.api.handler:entry
/ $req.user_id != None /
{
    capture(arg0, $req.user_id);
}
```

---

## 3. Sampling

Sampling reduces overhead by only executing probes for a subset of invocations.

### Directive-Based Sampling

Use `sample` directive in action block:

```dtrace
fn:myapp.api.high_traffic_endpoint:entry
{
    sample 10%;  # Only trace 10% of invocations
    capture(args);
}
```

Or ratio-based:

```dtrace
fn:myapp.api.endpoint:entry
{
    sample 1/100;  # Sample 1 in 100 invocations
    capture(args);
}
```

### Predicate-Based Sampling

More flexible, uses `rand()` function (returns float 0.0-1.0):

```dtrace
fn:myapp.api.endpoint:entry
/ rand() < 0.1 /  # 10% sampling
{
    capture(args);
}
```

**Recommendation**: Use predicate-based sampling for flexibility and composability with other conditions.

---

## 4. Data Access

### Object Field Access

#### Dictionary Access
```dtrace
obj["key"]
obj["nested"]["field"]
```

#### Attribute Access (Python objects)
```dtrace
obj.attribute
obj.nested.field
```

#### List/Tuple Indexing
```dtrace
args[0]
retval[1]
```

#### Chained Access
```dtrace
arg0.user["email"]
retval.data[0]["id"]
```

### Type Coercion

All values are Python objects. HogTrace preserves Python typing:

```dtrace
/ arg0.count > 100 /           # Numeric comparison
/ arg0.name == "admin" /       # String comparison
/ arg0.enabled /               # Boolean check
```

---

## 5. Request-Scoped Variables

Request-scoped variables persist across probes within a single request context.

### Syntax

Variables use `$req.` or `$request.` prefix:

```dtrace
$req.varname
$request.varname
```

Both forms are equivalent. Use shorter `$req.` for brevity.

### Assignment

```dtrace
fn:myapp.middleware.auth:entry
{
    $req.user_id = arg0.user.id;
    $req.start_time = timestamp();
}
```

### Reading

```dtrace
fn:myapp.api.handler:exit
/ $req.user_id != None /
{
    capture($req.user_id, $req.start_time, retval);
}
```

### Scope and Lifetime

- Variables are scoped to the current request/transaction
- Automatically cleaned up when request completes
- Undefined variables evaluate to `None`
- Web frameworks must integrate to provide request context

---

## 6. Action Primitives

All data collection uses `capture()` - data is sent to PostHog, not printed.

### `capture()` - Send Data to PostHog

#### Capture All Arguments
```dtrace
capture(args)
```

#### Capture All Local Variables
```dtrace
capture(locals)
```

#### Capture All Global Variables (use sparingly)
```dtrace
capture(globals)
```

#### Capture Specific Variables
```dtrace
capture(arg0, arg1, retval)
capture(arg0, $req.user_id, locals)
```

#### Named Capture (explicit field names)
```dtrace
capture(user_id=arg0, email=arg1, result=retval)
```

### `send()` - Alias for `capture()`

Some may prefer `send()`:

```dtrace
send(args, retval)
```

Both are equivalent. Use `capture()` for consistency with PostHog terminology.

---

## 7. Built-in Functions

### `timestamp()`
Returns current Unix timestamp (float, seconds since epoch).

```dtrace
$req.start_time = timestamp();
```

### `rand()`
Returns random float between 0.0 and 1.0.

```dtrace
/ rand() < 0.1 /  # 10% sampling
```

### `len(obj)`
Returns length of object (works on lists, dicts, strings, etc.).

```dtrace
/ len(args) > 2 /
```

### `str(obj)`, `int(obj)`, `float(obj)`
Type conversion functions.

```dtrace
capture(str(arg0))
```

---

## 8. Complete Examples

### Example 1: Basic Function Tracing

```dtrace
# Trace all calls to create_user
fn:myapp.users.create_user:entry
{
    capture(args);
}

fn:myapp.users.create_user:exit
{
    capture(retval);
}
```

### Example 2: Tracing with Guards

```dtrace
# Only trace admin user creation
fn:myapp.users.create_user:entry
/ arg0.role == "admin" /
{
    capture(args);
}
```

### Example 3: Request-Level Tracing

```dtrace
# Track request start
fn:django.core.handlers.wsgi.WSGIHandler:entry
{
    $req.request_id = arg0.META["REQUEST_ID"];
    $req.start_time = timestamp();
}

# Track database queries in this request
fn:myapp.db.execute_query:entry
/ $req.request_id != None /
{
    capture(query=$req.request_id, sql=arg0);
}

# Track request completion
fn:django.core.handlers.wsgi.WSGIHandler:exit
{
    $req.duration = timestamp() - $req.start_time;
    capture(
        request_id=$req.request_id,
        duration=$req.duration,
        status=retval.status_code
    );
}
```

### Example 4: Sampling High-Traffic Endpoints

```dtrace
# Sample 1% of high-traffic endpoint calls
fn:myapp.api.list_products:entry
/ rand() < 0.01 /
{
    capture(args);
}
```

### Example 5: Exception Tracking

```dtrace
# Capture when functions exit with exceptions
fn:myapp.payments.process_payment:exit
/ exception != None /
{
    capture(
        args=args,
        exception=exception,
        user_id=$req.user_id
    );
}
```

### Example 6: Line-Level Probing

```dtrace
# Probe specific line offset in function
fn:myapp.complex_algorithm.process:entry+10
{
    capture(locals);
}
```

### Example 7: Wildcard Probing

```dtrace
# Trace all API handlers
fn:myapp.api.*:entry
{
    sample 5%;
    capture(args, $req.user_id);
}
```

---

## 9. Grammar Summary (ANTLR4 Sketch)

```antlr4
grammar HogTrace;

// Top-level: one or more probe definitions
program: probe+ EOF;

probe: probeSpec predicate? action;

// Probe specification
probeSpec: PROVIDER ':' moduleFunction ':' probePoint;

PROVIDER: 'fn' | 'py';

moduleFunction: IDENTIFIER ('.' IDENTIFIER)* ;

probePoint: 'entry'
          | 'exit'
          | 'entry' '+' INT
          | 'exit' '+' INT
          ;

// Predicate (guard)
predicate: '/' expression '/';

// Action block
action: '{' statement* '}';

statement: assignment
         | sampleDirective
         | captureStatement
         ;

assignment: requestVar '=' expression ';';

requestVar: '$req' '.' IDENTIFIER
          | '$request' '.' IDENTIFIER
          ;

sampleDirective: 'sample' (percentage | ratio) ';';
percentage: INT '%';
ratio: INT '/' INT;

captureStatement: ('capture' | 'send') '(' captureArgs? ')' ';';

captureArgs: expression (',' expression)*
           | namedArg (',' namedArg)*
           ;

namedArg: IDENTIFIER '=' expression;

// Expressions
expression: expression '.' IDENTIFIER              # FieldAccess
          | expression '[' expression ']'          # IndexAccess
          | IDENTIFIER '(' expressionList? ')'     # FunctionCall
          | requestVar                             # RequestVarExpr
          | IDENTIFIER                             # Identifier
          | literal                                # Literal
          | expression op=('*'|'/'|'%') expression # MulDiv
          | expression op=('+'|'-') expression     # AddSub
          | expression op=('=='|'!='|'<'|'>'|'<='|'>=') expression # Comparison
          | expression op=('&&'|'||') expression   # Logical
          | '!' expression                         # Not
          | '(' expression ')'                     # Parens
          ;

expressionList: expression (',' expression)*;

literal: INT | FLOAT | STRING | BOOL | NONE;

BOOL: 'True' | 'False';
NONE: 'None';
INT: [0-9]+;
FLOAT: [0-9]+ '.' [0-9]+;
STRING: '"' (~["\r\n])* '"' | '\'' (~['\r\n])* '\'';
IDENTIFIER: [a-zA-Z_][a-zA-Z0-9_]*;

WS: [ \t\r\n]+ -> skip;
COMMENT: '#' ~[\r\n]* -> skip;
```

---

## 10. Semantics and Implementation Notes

### Request Context Tracking

HogTrace requires integration with web frameworks to track request boundaries:

- **Django**: Use middleware to set request context
- **Flask**: Use request context stack
- **FastAPI**: Use context vars and dependency injection

The runtime must:
1. Create a new request-scope when a request starts
2. Propagate request-scope through async calls
3. Clean up request-scope when request completes

### Bytecode Injection

The parser outputs an AST that the runtime uses to:

1. **Locate probe points**: Resolve module.function to actual code objects
2. **Install probes**: Inject bytecode at entry, exit, or offset points
3. **Evaluate predicates**: Generate bytecode for guard conditions
4. **Execute actions**: Generate calls to capture/send primitives

### Performance Considerations

- **Sampling**: Essential for high-traffic functions
- **Predicate evaluation**: Should short-circuit to minimize overhead
- **Request-scope storage**: Use thread-locals or context vars (lightweight)
- **Data serialization**: Lazy serialization only when probe fires

### Error Handling

- Probes should **never** crash the application
- Failed predicates default to `False` (skip action)
- Failed captures log error but don't raise
- Malformed probes rejected at parse time

---

## 11. Future Extensions

### Additional Providers

- **`ex:module.ExceptionType`**: Probe when specific exception raised
- **`http:method:path`**: HTTP request/response probes
- **`db:query`**: Database query probes
- **`cache:get/set`**: Cache operation probes

### Advanced Features

- **Aggregations**: `count()`, `avg()`, `min()`, `max()` over time windows
- **Histograms**: Latency distributions
- **Stack traces**: `capture(stacktrace())`
- **Conditional compilation**: Enable/disable probes without redeploying

### Multi-Language Support

Extend beyond Python to JavaScript, Go, Ruby for polyglot environments.

---

## 12. Reference

### Reserved Keywords

```
fn, py, entry, exit, sample, capture, send, rand, timestamp, len, str, int, float
True, False, None
```

### Predefined Variables

#### Entry Probes
- `args`, `arg0`, `arg1`, ...
- `kwargs`
- `self`

#### Exit Probes
- `retval`
- `exception`

#### Global
- `$req.*` / `$request.*`

### Operators (precedence, high to low)

1. `[]`, `.` (field/index access)
2. Function calls `()`
3. `!` (logical not)
4. `*`, `/`, `%`
5. `+`, `-`
6. `<`, `>`, `<=`, `>=`
7. `==`, `!=`
8. `&&` (logical and)
9. `||` (logical or)

---

## Appendix: Design Decisions

### Why `capture()` instead of `trace()` or `printf()`?

Aligns with PostHog's `capture()` API for events. Makes it clear data goes to observability backend, not console.

### Why `$req.` instead of `self->`?

- More explicit than DTrace's `self->`
- Clearer for web developers (request is the unit of work)
- Avoids confusion with Python's `self`

### Why predicate-based sampling over directives?

Predicates compose better with other conditions:

```dtrace
/ rand() < 0.1 && arg0.user_type == "premium" /
```

But directives are simpler for basic cases. Supporting both gives flexibility.

### Why DTrace-style syntax?

- Proven in production systems for decades
- Familiar to SREs and performance engineers
- Clear separation of probe point, predicate, and action
- Extensible syntax for future probe types

---

## License

This specification is part of the HogTrace project.
