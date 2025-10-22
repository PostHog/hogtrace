#![no_main]

use libfuzzer_sys::fuzz_target;
use vm::parser::{Lexer, Parser, Compiler};

fuzz_target!(|data: &[u8]| {
    // Try to convert bytes to UTF-8 string
    if let Ok(source) = std::str::from_utf8(data) {
        // Skip very large inputs to avoid timeouts
        if source.len() > 5000 {
            return;
        }

        // Complete pipeline: source → tokens → AST → bytecode → protobuf
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);

        if let Ok(ast) = parser.parse_program() {
            let mut compiler = Compiler::new();

            if let Ok(program) = compiler.compile(ast) {
                // Test protobuf serialization
                if let Ok(bytes) = program.to_proto_bytes() {
                    // Test deserialization
                    if let Ok(decoded) = vm::program::Program::from_proto_bytes(&bytes) {
                        // Property: Round-trip should preserve structure
                        assert_eq!(decoded.version, program.version);
                        assert_eq!(decoded.probes.len(), program.probes.len());
                        assert_eq!(decoded.sampling, program.sampling);

                        // Property: Constant pool should be preserved
                        assert_eq!(decoded.constant_pool.len(), program.constant_pool.len());

                        // Property: Each probe should be preserved
                        for (i, (orig, dec)) in program.probes.iter().zip(decoded.probes.iter()).enumerate() {
                            assert_eq!(dec.id, orig.id, "Probe {} ID mismatch", i);
                            assert_eq!(dec.predicate.len(), orig.predicate.len(), "Probe {} predicate size mismatch", i);
                            assert_eq!(dec.body.len(), orig.body.len(), "Probe {} body size mismatch", i);

                            // Bytecode should be identical
                            assert_eq!(dec.predicate, orig.predicate, "Probe {} predicate bytecode mismatch", i);
                            assert_eq!(dec.body, orig.body, "Probe {} body bytecode mismatch", i);
                        }

                        // Property: Second round-trip should produce identical bytes
                        if let Ok(bytes2) = decoded.to_proto_bytes() {
                            assert_eq!(bytes, bytes2, "Protobuf encoding should be deterministic");
                        }
                    }
                }
            }
        }
    }
});
