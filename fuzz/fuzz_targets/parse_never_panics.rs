#![no_main]

use libfuzzer_sys::fuzz_target;
use s_expr_core::{parse, DialectConfig};

fuzz_target!(|data: &[u8]| {
    // Interpret arbitrary bytes as UTF-8 with replacement for invalid sequences.
    let text = String::from_utf8_lossy(data);
    let src = text.as_ref();

    // Default parse must not panic on any input.
    for result in [
        parse(src),
        s_expr_core::parse_with_dialect(src, &DialectConfig::default()),
    ] {
        if let Err(err) = result {
            let len = src.len() as u32;
            let span = err.span;
            assert!(
                span.start.byte <= span.end.byte,
                "error span must be non-decreasing: {:?}",
                span
            );
            assert!(
                span.end.byte <= len,
                "error span must lie within input bounds: {:?} / len={}",
                span,
                len
            );
        }
    }
});
