#![no_main]

use libfuzzer_sys::fuzz_target;
use s_expr_core::{parse, pretty_print, Expr, Node};

fn strip_spans(nodes: &[Node]) -> Vec<Expr> {
    nodes.iter().map(|n| n.expr.clone()).collect()
}

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let src = text.as_ref();

    // Only enforce roundtrip invariant for inputs that parse successfully.
    if let Ok(first) = parse(src) {
        let printed = pretty_print(&first);
        if let Ok(second) = parse(&printed) {
            assert_eq!(
                strip_spans(&first),
                strip_spans(&second),
                "roundtrip invariant violated for input: {:?}",
                src
            );
        }
    }
});
