# HogTrace Bytecode VM

A compact, language-agnostic bytecode virtual machine for executing HogTrace probes with minimal overhead.

## Features

- **Bytecode-based**: Compact instruction set optimized for small size
- **Constant Pool**: Shared pool of constants (literals, identifiers, field names, function names)
- **Language-Agnostic Core**: Generic VM with language-specific operations delegated to a `Dispatcher` trait
- **Protobuf Wire Format**: Efficient binary serialization for network transmission
- **No Control Flow**: Linear execution only (no loops, no conditionals) - keeps bytecode simple
- **Stack-Based**: Standard stack machine architecture

## Architecture

### Core Components

1. **Opcodes** (`opcodes.rs`): 22 instruction opcodes for:
   - Stack operations (PUSH_CONST, POP)
   - Variable access (LOAD_VAR, STORE_VAR)
   - Arithmetic (ADD, SUB, MUL, DIV, MOD)
   - Comparisons (EQ, NE, LT, GT, LE, GE)
   - Logical operations (AND, OR, NOT)
   - Field/item access (GET_ATTR, GET_ITEM)
   - Function calls (CALL_FUNC)

2. **Constant Pool** (`constant_pool.rs`): Stores all constants:
   - Primitives: Int, Float, String, Bool, None
   - Identifiers: Variable names (args, arg0, retval, etc.)
   - Field names: For attribute access
   - Function names: For built-in functions

3. **Value System** (`value.rs`): Runtime values:
   - Primitives: Bool, Int, Float, String, None
   - Object: Language-specific objects (e.g., Python PyObjects)

4. **Dispatcher Trait** (`dispatcher.rs`): Interface for language-specific operations:
   - `load_variable()`: Access frame variables
   - `store_variable()`: Store request-scoped variables
   - `get_attribute()`: Object attribute access
   - `get_item()`: Dictionary/list indexing
   - `call_function()`: Built-in function calls

5. **Executor** (`executor.rs`): Core VM loop that executes bytecode

6. **Program/Probe** (`program.rs`): Program structure with protobuf serialization

7. **Python Dispatcher** (`python_dispatcher.rs`): PyO3-based dispatcher for Python

### Wire Format (Protobuf)

```protobuf
message Program {
  uint32 version = 1;
  ConstantPool constant_pool = 2;
  repeated Probe probes = 3;
  float sampling = 4;
}

message Probe {
  string id = 1;
  ProbeSpec spec = 2;
  bytes predicate = 4;    // Predicate bytecode
  bytes body = 5;         // Action body bytecode
}
```

## Example Usage

### Creating a Simple Program

```rust
use vm::{Constant, ConstantPool, Executor, Program, Probe, ProbeSpec, FnTarget, Opcode, Value};

// Build constant pool
let mut pool = ConstantPool::new();
let const_10 = pool.add(Constant::Int(10));
let var_arg0 = pool.add(Constant::Identifier("arg0".to_string()));

// Build predicate: arg0 > 10
let predicate = vec![
    Opcode::LoadVar as u8, var_arg0 as u8, (var_arg0 >> 8) as u8,
    Opcode::PushConst as u8, const_10 as u8, (const_10 >> 8) as u8,
    Opcode::Gt as u8,
];

// Create probe
let probe = Probe {
    id: "my_probe".to_string(),
    spec: ProbeSpec::Fn {
        specifier: "myapp.users.create".to_string(),
        target: FnTarget::Entry,
    },
    predicate,
    body: vec![],
};

// Create program
let program = Program {
    version: 1,
    constant_pool: pool,
    probes: vec![probe],
    sampling: 1.0,
};

// Serialize to protobuf
let bytes = program.to_proto_bytes().unwrap();

// Send over network...
```

### Executing Bytecode

```rust
use vm::{Executor, Dispatcher, Value};

// Implement dispatcher for your language
struct MyDispatcher;
impl Dispatcher for MyDispatcher {
    fn load_variable(&mut self, name: &str) -> Result<Value, String> {
        // Return variable value from current execution context
        Ok(Value::Int(42))
    }
    // ... implement other methods
}

// Execute bytecode
let mut dispatcher = MyDispatcher;
let mut executor = Executor::new(&program.constant_pool, &mut dispatcher);
let result = executor.execute(&bytecode)?;
```

## Bytecode Format

- Opcodes: Single byte (u8)
- Operands:
  - Constant pool indices: u16 (little-endian)
  - Argument counts: u8
- No jumps or branches (no control flow)
- Execution proceeds linearly from start to end
- Final stack value is the result

## Size Optimization

The VM is designed for minimal bytecode size:
- Opcodes: 1 byte each
- Constant pool indices: 2 bytes (supports up to 65,536 constants)
- All constants shared across probes in a program
- No padding or alignment requirements

Example bytecode sizes:
- `arg0 > 10`: 7 bytes (LOAD_VAR, PUSH_CONST, GT)
- `capture(arg0)`: 7 bytes (LOAD_VAR, CALL_FUNC)

## Language Support

Currently implemented:
- **Python**: Full PyO3-based dispatcher with frame introspection

Easily extensible to:
- **JavaScript**: Via V8 or QuickJS bindings
- **Ruby**: Via ruby-sys
- **Go**: Via CGO
- Any language with FFI support

## Performance

- Stack-based execution
- No allocations during execution (pre-allocated stack)
- Minimal overhead per instruction (~10-20 CPU instructions)
- Suitable for high-frequency probes

## Testing

```bash
cargo test
cargo run  # Run example program
```

## License

Part of the HogTrace project.
