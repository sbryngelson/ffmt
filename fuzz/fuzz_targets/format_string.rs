#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Only fuzz valid UTF-8
    if let Ok(s) = std::str::from_utf8(data) {
        // Should never panic
        let result = ffmt::format_string(s);
        
        // Idempotency: formatting twice should produce the same output
        let result2 = ffmt::format_string(&result);
        assert_eq!(result, result2, "Not idempotent on input: {:?}", s);
    }
});
