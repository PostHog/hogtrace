# HogTrace

HogTrace is a DTrace-inspired instrumentation language for Python web applications. It provides a safe, efficient way to inject observability probes into running Python code without modifying application source.

## Overview

HogTrace allows you to write declarative probe specifications that capture data from function calls at runtime. Probes are compiled to bytecode and executed by a Rust-based virtual machine, providing performance and safety guarantees while maintaining the expressiveness needed for production debugging and monitoring.

The language supports conditional predicates, request-scoped variables, and structured data capture. All probe code is validated at compile time and executed in a sandboxed environment with strict resource limits.

## Installation

Install the package using pip:

```bash
pip install hogtrace
```

For development:

```bash
git clone https://github.com/posthog/hogtrace
cd hogtrace
pip install -e .
```

The package includes a Rust extension that provides the bytecode compiler and virtual machine. The extension is built automatically during installation using maturin.

## Basic Usage

A typical HogTrace workflow involves compiling probe definitions and executing them against Python stack frames:

```python
import sys
from hogtrace import compile, execute_probe
from hogtrace.request_store import RequestLocalStore

# Compile probe definition
program = compile("""
    fn:myapp.users.create_user:entry {
        capture(user_id=arg0, timestamp=timestamp());
    }
""")

# Create request-local storage
store = RequestLocalStore()

# Execute probe against current frame
probe = program.probes[0]
frame = sys._getframe()
result = execute_probe(program, probe, frame, store)

if result:
    print(result)  # {'user_id': ..., 'timestamp': ...}
```

## Language Syntax

HogTrace probe definitions follow a structured format:

```
fn:module.path.function:probe_point [/ predicate /] { actions }
```

The probe point specifies where to instrument (`entry` or `exit`). The optional predicate determines whether the probe should fire. Actions define what data to capture or how to modify request-scoped state.

### Probe Points

Entry probes fire when a function is called. They have access to function arguments and the call context:

```python
fn:myapp.api.handler:entry {
    $req.start_time = timestamp();
    capture(method=arg0, path=arg1);
}
```

Exit probes fire when a function returns. They have access to the return value and can detect exceptions:

```python
fn:myapp.api.handler:exit {
    $req.duration = timestamp() - $req.start_time;
    capture(duration=$req.duration, status=retval.status_code);
}
```

### Predicates

Predicates are boolean expressions that control probe execution. Only probes whose predicates evaluate to `True` will execute their action body:

```python
fn:myapp.db.query:entry / len(args) > 2 / {
    capture(query=arg0, params=args[1:]);
}
```

Predicates can reference arguments, request-scoped variables, and call built-in functions. Any non-boolean predicate result is treated as false.

### Request-Scoped Variables

Request-scoped variables persist across probe executions within the same request context. They are prefixed with `$req.` or `$request.`:

```python
fn:web.middleware.start:entry {
    $req.request_id = arg0.headers["X-Request-ID"];
    $req.user_id = None;  # Initialize
}

fn:auth.authenticate:exit {
    $req.user_id = retval.user.id;
}

fn:web.middleware.end:exit {
    capture(
        request_id=$req.request_id,
        user_id=$req.user_id,
        duration=timestamp() - $req.start_time
    );
}
```

Request-scoped variables are stored in a thread-local `RequestLocalStore` instance that must be passed to probe execution. Reading an unset variable returns `None` rather than raising an error.

### Built-in Functions

The language provides several built-in functions:

- `timestamp()` - Returns current Unix timestamp
- `rand()` - Returns random float between 0.0 and 1.0
- `len(obj)` - Returns length of object
- `capture(**kwargs)` - Captures named data for export

### Data Capture

The `capture()` function records data from probe execution. It accepts named arguments:

```python
fn:myapp.process:entry {
    capture(
        func_name="process",
        arg_count=len(args),
        first_arg=arg0
    );
}
```

Captured data is returned as a dictionary from `execute_probe()`. Multiple capture calls in the same probe are supported, though typically only the first result is returned.

## Architecture

HogTrace uses a multi-stage compilation and execution pipeline:

### Compilation

Source code is parsed into an abstract syntax tree, validated for correctness, and compiled to bytecode. The bytecode is a stack-based instruction set designed for efficient execution and serialization.

The compiler performs constant folding, validates variable references, and ensures all operations are safe. The resulting bytecode can be serialized to Protocol Buffers for storage and network transmission.

### Execution

The bytecode is executed by a Rust virtual machine using a dispatcher pattern. The dispatcher provides language-specific operations like variable access and function calls while the core executor handles stack manipulation and control flow.

For Python integration, the dispatcher accesses frame locals, evaluates attribute access on Python objects, and marshals data between Rust and Python representations. Request-scoped variables are stored in a thread-local store that is passed explicitly to each probe execution.

### Security Model

All probe execution is sandboxed. Probes cannot modify frame locals, call arbitrary Python functions, or access the file system. The only side effects are writing to the request-local store and accumulating capture events.

The bytecode instruction set includes explicit bounds checking, type validation, and resource limits. Stack depth is limited, execution time is bounded, and all memory allocation is controlled.

## Testing

The project includes comprehensive test coverage:

```bash
# Run Rust tests (bytecode compiler, VM, parser)
cargo test

# Run Python integration tests
pytest tests/

# Run specific test suite
pytest tests/test_request_store.py -v
```

The test suite validates correct bytecode generation, request-scoped variable isolation, security constraints, and end-to-end probe execution.

## Performance Characteristics

The Rust-based virtual machine provides predictable performance:

- Compilation is fast enough for dynamic probe definition
- Bytecode execution overhead is minimal (microseconds per probe)
- Request store access is backed by thread-local storage
- No global interpreter lock contention during VM execution

Probe execution is synchronous and occurs inline with the instrumented code. For high-throughput scenarios, consider using predicates or sampling to reduce probe overhead.

## Development

Build the Rust extension in development mode:

```bash
# Install development dependencies
pip install maturin

# Build and install extension
maturin develop

# Build with optimizations
maturin develop --release
```

The extension source is in `src/` and includes:

- Lexer and parser (`src/parser/`)
- Bytecode compiler (`src/parser/compiler.rs`)
- Virtual machine executor (`src/executor.rs`)
- Python dispatcher (`src/python_dispatcher.rs`)
- Python bindings (`src/python_bindings.rs`)

## License

This software is part of the PostHog ecosystem and is released under the MIT License.
