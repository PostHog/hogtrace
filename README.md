# HogTrace

A DTrace-inspired language for runtime instrumentation of Python web applications.

## Quick Start

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

for probe in program.probes:
    print(f"Probe: {probe.spec}")
```

See **[docs/QUICKSTART.md](docs/QUICKSTART.md)** for complete guide.

## CLI Usage

```bash
# Parse and display
uv run python main.py parse tests/test_examples.hogtrace

# Validate syntax
uv run python main.py validate tests/test_examples.hogtrace

# Evaluate code
uv run python main.py eval "fn:app.test:entry { capture(args); }"
```

## Installation

```bash
uv sync
```

## Project Structure

```
hogtrace/
├── hogtrace/          # Main package
│   ├── __init__.py    # Public API (parse, parse_file)
│   ├── ast.py         # AST data classes
│   └── builder.py     # AST builder
├── generated/         # ANTLR-generated parser files
├── grammar/           # ANTLR4 grammar definitions
│   └── HogTrace.g4    # Main grammar
├── tests/             # Test suite
│   ├── test_api.py    # API tests
│   ├── test_parser.py # Parser tests
│   └── test_examples.hogtrace  # Test examples
├── examples/          # Example scripts
│   ├── demo_api.py    # API demos
│   └── example_usage.py  # Low-level ANTLR usage
├── docs/              # Documentation
│   ├── QUICKSTART.md  # Quick start guide
│   ├── API.md         # Complete API reference
│   ├── SPEC.md        # Language specification
│   ├── TESTING.md     # Testing guide
│   └── README.md      # Full documentation
├── main.py            # CLI tool
└── pyproject.toml     # Package configuration
```

## Documentation

- **[docs/QUICKSTART.md](docs/QUICKSTART.md)** - Quick start guide ⭐
- **[docs/API.md](docs/API.md)** - Complete API reference
- **[docs/SPEC.md](docs/SPEC.md)** - Language specification
- **[docs/README.md](docs/README.md)** - Full documentation
- **[docs/TESTING.md](docs/TESTING.md)** - Testing guide

## Examples

Run the demos:

```bash
# API demo (6 examples)
uv run python examples/demo_api.py

# Low-level ANTLR usage
uv run python examples/example_usage.py
```

## Testing

Run all tests with pytest:

```bash
# Run all tests
uv run pytest

# Run with verbose output
uv run pytest -v

# Run specific test file
uv run pytest tests/test_vm.py

# Run specific test
uv run pytest tests/test_vm.py::test_basic_capture
```

**34 tests** - All passing ✅

## Features

- ✅ Clean Python API (`hogtrace.parse()`)
- ✅ Full type hints for IDE support
- ✅ DTrace-inspired syntax
- ✅ Request-scoped variables (`$req.*`)
- ✅ Sampling support
- ✅ Predicates/guards
- ✅ CLI tool
- ✅ Comprehensive documentation
- ✅ Test suite (100% passing)

## Language Example

```dtrace
# Track request lifecycle
fn:django.core.handlers.wsgi.WSGIHandler:entry
{
    $req.request_id = arg0.META["REQUEST_ID"];
    $req.start_time = timestamp();
}

# Sample database queries
fn:myapp.db.execute_query:entry
/ rand() < 0.1 /  # 10% sampling
{
    capture(query=$req.request_id, sql=arg0);
}

# Track completion
fn:django.core.handlers.wsgi.WSGIHandler:exit
{
    capture(
        request_id=$req.request_id,
        duration=timestamp() - $req.start_time,
        status=retval.status_code
    );
}
```

## Development Status

**Phase 1: Language Design & Parsing** ✅ COMPLETE

- [x] Language specification
- [x] ANTLR4 grammar
- [x] Parser implementation
- [x] Clean API
- [x] Comprehensive test suite
- [x] Documentation

**Phase 2: Runtime Implementation** 🚧 TODO

- [ ] AST builder
- [ ] Bytecode injection
- [ ] Probe management
- [ ] Request context tracking
- [ ] PostHog integration

## License

Part of the PostHog ecosystem.
