#![no_main]

use libfuzzer_sys::fuzz_target;
use vm::parser::lexer::Lexer;

fuzz_target!(|data: &[u8]| {
    // Try to convert bytes to UTF-8 string
    if let Ok(source) = std::str::from_utf8(data) {
        // Create lexer - this should never panic
        let mut lexer = Lexer::new(source);

        // Tokenize the entire input - should never panic
        // May produce error tokens, but should be graceful
        let mut tokens = Vec::new();
        let mut count = 0;
        loop {
            let token = lexer.next_token();
            tokens.push(token);

            count += 1;

            // Prevent infinite loops on malformed input
            if count > 100000 {
                break;
            }

            // Check for EOF by cloning and checking via Debug string (avoid E0223 ambiguity)
            let token_str = format!("{:?}", tokens.last().unwrap());
            if token_str.starts_with("Eof") {
                break;
            }
        }

        // Property: Should always produce at least one token
        assert!(!tokens.is_empty(), "Lexer should produce at least one token");
    }
});
