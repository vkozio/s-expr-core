#![no_main]

use libfuzzer_sys::fuzz_target;
use s_expr_core::{parse_recovering, DialectConfig};

fuzz_target!(|data: &[u8]| {
    // Interpret arbitrary bytes as UTF-8 with replacement for invalid sequences.
    let text = String::from_utf8_lossy(data);
    let src = text.as_ref();
    let len = src.len() as u32;

    let rec = parse_recovering(src, &DialectConfig::default());

    // Node spans must be monotone non-decreasing and inside the input.
    let mut prev_start = 0u32;
    for node in s_expr_core::ExprIter::new(&rec.nodes) {
        let span = node.span;
        assert!(
            span.start.byte <= span.end.byte,
            "node span decreasing: {:?}",
            span
        );
        assert!(
            span.end.byte <= len,
            "node span out of bounds: {:?} / len={}",
            span,
            len
        );
        assert!(
            prev_start <= span.start.byte,
            "node spans not monotone: {:?}",
            span
        );
        prev_start = span.start.byte;
    }

    // Error spans must be monotone non-decreasing and inside the input.
    let mut prev_err_start = 0u32;
    for err in &rec.errors {
        let span = err.span;
        assert!(
            span.start.byte <= span.end.byte,
            "error span decreasing: {:?}",
            span
        );
        assert!(
            span.end.byte <= len,
            "error span out of bounds: {:?} / len={}",
            span,
            len
        );
        assert!(
            prev_err_start <= span.start.byte,
            "error spans not monotone: {:?}",
            span
        );
        prev_err_start = span.start.byte;
    }
});
