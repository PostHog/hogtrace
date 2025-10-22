//! HogTrace Bytecode VM
//!
//! A stack-based virtual machine for executing HogTrace probe bytecode.
//! The VM is language-agnostic at its core, with language-specific behavior
//! implemented through the `Dispatcher` trait.
//!
//! # Architecture
//!
//! - **Bytecode**: Compact instruction format with opcodes and operands
//! - **Constant Pool**: Shared pool of constants (literals, identifiers, names)
//! - **Stack-based execution**: No control flow, linear execution only
//! - **Dispatcher pattern**: Language-specific operations (Python, JavaScript, etc.)
//!
//! # Example Usage
//!
//! ```ignore
//! use vm::{Program, Executor, PythonDispatcher};
//! use pyo3::Python;
//!
//! // Deserialize a program from protobuf bytes (received over network)
//! let program_bytes = vec![/* ... */];
//! let program = Program::from_proto_bytes(&program_bytes).unwrap();
//!
//! // Execute a probe's predicate
//! Python::attach(|py| {
//!     let frame = /* get Python frame */;
//!     let mut dispatcher = PythonDispatcher::new_entry(py, frame);
//!     let mut executor = Executor::new(&program.constant_pool, &mut dispatcher);
//!
//!     let result = executor.execute(&program.probes[0].predicate).unwrap();
//!     // Check if predicate is true, then execute body...
//! });
//! ```

// Core modules
pub mod constant_pool;
pub mod dispatcher;
pub mod executor;
pub mod opcodes;
pub mod program;
pub mod value;

// Language-specific implementations
pub mod python_dispatcher;

// Parser
pub mod parser;

// Re-export main types for convenience
pub use constant_pool::{Constant, ConstantPool};
pub use dispatcher::{BinaryOp, ComparisonOp, Dispatcher};
pub use executor::Executor;
pub use opcodes::Opcode;
pub use program::{FnTarget, Probe, ProbeSpec, Program};
pub use python_dispatcher::PythonDispatcher;
pub use value::Value;

/// Current bytecode format version
pub const BYTECODE_VERSION: u32 = 1;

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        // Build a simple program: 40 + 2
        let mut pool = ConstantPool::new();
        let idx1 = pool.add(Constant::Int(40));
        let idx2 = pool.add(Constant::Int(2));

        let bytecode = vec![
            Opcode::PushConst as u8,
            (idx1 & 0xFF) as u8,
            ((idx1 >> 8) & 0xFF) as u8,
            Opcode::PushConst as u8,
            (idx2 & 0xFF) as u8,
            ((idx2 >> 8) & 0xFF) as u8,
            Opcode::Add as u8,
        ];

        struct TestDispatcher;
        impl Dispatcher for TestDispatcher {
            fn load_variable(&mut self, _name: &str) -> Result<Value, String> {
                Err("No variables".to_string())
            }
            fn store_variable(&mut self, _name: &str, _value: Value) -> Result<(), String> {
                Ok(())
            }
            fn get_attribute(&mut self, _obj: &Value, _attr: &str) -> Result<Value, String> {
                Err("Not implemented".to_string())
            }
            fn get_item(&mut self, _obj: &Value, _key: &Value) -> Result<Value, String> {
                Err("Not implemented".to_string())
            }
            fn call_function(&mut self, _name: &str, _args: Vec<Value>) -> Result<Value, String> {
                Err("Not implemented".to_string())
            }
        }

        let mut dispatcher = TestDispatcher;
        let mut executor = Executor::new(&pool, &mut dispatcher);
        let result = executor.execute(&bytecode).unwrap();

        match result {
            Value::Int(42) => (),
            other => panic!("Expected Int(42), got {:?}", other),
        }
    }
}
