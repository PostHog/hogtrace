#![no_main]

use libfuzzer_sys::fuzz_target;
use vm::parser::{Lexer, Parser};

fuzz_target!(|data: &[u8]| {
    // Try to convert bytes to UTF-8 string
    if let Ok(source) = std::str::from_utf8(data) {
        // Skip very large inputs to avoid timeouts
        if source.len() > 10000 {
            return;
        }

        // Create parser - should never panic
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);

        // Attempt to parse - may fail with ParseError, but should never panic
        let result = parser.parse_program();

        match result {
            Ok(program) => {
                // Property: Valid program should have well-formed structure
                // Verify basic invariants
                for probe in &program.probes {
                    // Each probe must have a spec
                    assert!(!probe.spec.module_function.to_string().is_empty());

                    // If predicate exists, it should be a valid expression
                    if let Some(_pred) = &probe.predicate {
                        // Predicate exists and parsed successfully
                    }

                    // Body should be a valid list of statements
                    for _stmt in &probe.body {
                        // Statements exist and parsed successfully
                    }
                }
            }
            Err(_parse_error) => {
                // Parse errors are expected for malformed input
                // The important thing is that we don't panic
            }
        }
    }
});
