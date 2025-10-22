#![no_main]

use libfuzzer_sys::fuzz_target;
use vm::parser::{Lexer, Parser, Compiler};

fuzz_target!(|data: &[u8]| {
    // Try to convert bytes to UTF-8 string
    if let Ok(source) = std::str::from_utf8(data) {
        // Skip very large inputs to avoid timeouts
        if source.len() > 10000 {
            return;
        }

        // Parse first
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);

        if let Ok(ast) = parser.parse_program() {
            // Try to compile - should never panic
            let mut compiler = Compiler::new();
            let result = compiler.compile(ast);

            match result {
                Ok(program) => {
                    // Property: Valid program should have valid bytecode
                    assert_eq!(program.version, 1);

                    // Property: Constant pool should not be absurdly large
                    assert!(program.constant_pool.len() < 100000, "Constant pool too large");

                    // Property: Each probe should have valid bytecode
                    for probe in &program.probes {
                        // Probe ID should not be empty
                        assert!(!probe.id.is_empty());

                        // Bytecode should not be absurdly large
                        assert!(probe.predicate.len() < 1000000, "Predicate bytecode too large");
                        assert!(probe.body.len() < 1000000, "Body bytecode too large");

                        // If bytecode exists, first byte should be valid opcode
                        if !probe.predicate.is_empty() {
                            let opcode = probe.predicate[0];
                            assert!(opcode < 50, "Invalid opcode in predicate"); // Opcodes are < 50
                        }
                        if !probe.body.is_empty() {
                            let opcode = probe.body[0];
                            assert!(opcode < 50, "Invalid opcode in body");
                        }
                    }

                    // Property: Program should be serializable to protobuf
                    let proto_result = program.to_proto_bytes();
                    assert!(proto_result.is_ok(), "Program should be serializable");

                    // Property: Protobuf round-trip should work
                    if let Ok(bytes) = proto_result {
                        let decoded_result = vm::program::Program::from_proto_bytes(&bytes);
                        assert!(decoded_result.is_ok(), "Protobuf round-trip should work");

                        if let Ok(decoded) = decoded_result {
                            // Basic properties should match
                            assert_eq!(decoded.version, program.version);
                            assert_eq!(decoded.probes.len(), program.probes.len());
                        }
                    }
                }
                Err(_compile_error) => {
                    // Compile errors are expected for some valid AST (e.g., unsupported features)
                    // The important thing is that we don't panic
                }
            }
        }
    }
});
