use s_expr_core::{
    parse_recovering, parse_with_trivia, AtomKind, DialectConfig, Expr, Node, ParseErrorCode,
};

fn check_spans(nodes: &[Node], errors: &[s_expr_core::ParseError], len: u32) {
    let mut prev = 0u32;
    for node in s_expr_core::ExprIter::new(nodes) {
        let span = node.span;
        assert!(
            span.start.byte <= span.end.byte,
            "node span decreasing {span:?}"
        );
        assert!(span.end.byte <= len, "node span {span:?} beyond len {len}");
        assert!(
            prev <= span.start.byte,
            "node span start not monotone {span:?}"
        );
        prev = span.start.byte;
    }
    let mut prev_err = 0u32;
    for err in errors {
        let span = err.span;
        assert!(
            span.start.byte <= span.end.byte,
            "error span decreasing {span:?}"
        );
        assert!(span.end.byte <= len, "error span {span:?} beyond len {len}");
        assert!(
            prev_err <= span.start.byte,
            "error span start not monotone {span:?}"
        );
        prev_err = span.start.byte;
    }
}

#[test]
fn unclosed_list_at_eof_one_level() {
    let src = "(a b";
    let rec = parse_recovering(src, &DialectConfig::default());
    assert_eq!(rec.nodes.len(), 1);
    assert!(matches!(&rec.nodes[0].expr, Expr::List(_)));
    assert_eq!(rec.errors.len(), 1);
    assert_eq!(rec.errors[0].code, ParseErrorCode::UnbalancedParen);
    // error span is the '('
    assert_eq!(rec.errors[0].span.start.byte, 0);
    assert_eq!(rec.errors[0].span.end.byte, 1);
    check_spans(&rec.nodes, &rec.errors, src.len() as u32);
}

#[test]
fn unclosed_lists_nested() {
    let src = "(a (b c";
    let rec = parse_recovering(src, &DialectConfig::default());
    assert_eq!(rec.errors.len(), 2);
    assert!(rec
        .errors
        .iter()
        .all(|e| e.code == ParseErrorCode::UnbalancedParen));
    // errors reported in source order: '(' at 0, then '(' at 3
    assert_eq!(rec.errors[0].span.start.byte, 0);
    assert_eq!(rec.errors[1].span.start.byte, 3);
    assert_eq!(rec.nodes.len(), 1);
    match &rec.nodes[0].expr {
        Expr::List(children) => {
            assert_eq!(children.len(), 2);
            assert!(matches!(&children[0].expr, Expr::Atom(AtomKind::Symbol(s)) if s == "a"));
            assert!(matches!(&children[1].expr, Expr::List(_)));
        }
        _ => panic!("expected root list"),
    }
    check_spans(&rec.nodes, &rec.errors, src.len() as u32);
}

#[test]
fn extra_close_paren_is_skipped() {
    let src = "(a))";
    let rec = parse_recovering(src, &DialectConfig::default());
    assert_eq!(rec.nodes.len(), 1);
    assert!(matches!(&rec.nodes[0].expr, Expr::List(_)));
    assert_eq!(rec.errors.len(), 1);
    assert_eq!(rec.errors[0].code, ParseErrorCode::UnbalancedParen);
    assert_eq!(rec.errors[0].span.start.byte, 3);
    check_spans(&rec.nodes, &rec.errors, src.len() as u32);
}

#[test]
fn unterminated_string_ends_at_eol_and_parsing_continues() {
    let src = "\"abc\n(foo)";
    let rec = parse_recovering(src, &DialectConfig::default());
    assert_eq!(rec.nodes.len(), 2);
    assert!(matches!(&rec.nodes[0].expr, Expr::Atom(AtomKind::String(s)) if s == "abc"));
    assert!(matches!(&rec.nodes[1].expr, Expr::List(_)));
    let unterminated = rec
        .errors
        .iter()
        .filter(|e| e.code == ParseErrorCode::UnterminatedString)
        .count();
    assert_eq!(unterminated, 1);
    check_spans(&rec.nodes, &rec.errors, src.len() as u32);
}

#[test]
fn bad_atom_becomes_error_node() {
    let src = "\"a\\q";
    let rec = parse_recovering(src, &DialectConfig::default());
    assert_eq!(rec.nodes.len(), 1);
    match &rec.nodes[0].expr {
        Expr::Error(raw) => assert_eq!(raw, "\"a\\q"),
        _ => panic!("expected error node"),
    }
    assert!(rec
        .errors
        .iter()
        .any(|e| e.code == ParseErrorCode::InvalidAtom));
    check_spans(&rec.nodes, &rec.errors, src.len() as u32);
}

#[test]
fn depth_limit_hit_replaces_subtree_with_error_node() {
    let mut dialect = DialectConfig::default();
    dialect.limits.max_depth = Some(1);
    let src = "(a (b c))";
    let rec = parse_recovering(src, &dialect);
    assert_eq!(rec.nodes.len(), 1);
    match &rec.nodes[0].expr {
        Expr::List(children) => {
            assert_eq!(children.len(), 2);
            assert!(matches!(&children[0].expr, Expr::Atom(AtomKind::Symbol(s)) if s == "a"));
            match &children[1].expr {
                Expr::Error(raw) => assert_eq!(raw, "(b c)"),
                _ => panic!("expected error subtree"),
            }
        }
        _ => panic!("expected root list"),
    }
    assert!(rec
        .errors
        .iter()
        .any(|e| matches!(&e.code, ParseErrorCode::LimitExceeded(_))));
    check_spans(&rec.nodes, &rec.errors, src.len() as u32);
}

#[test]
fn valid_input_equals_parse_with_trivia() {
    let src = "(a ; note\n (b c) d)";
    let dialect = DialectConfig::default();
    let (expect_nodes, expect_comments) = parse_with_trivia(src, &dialect).unwrap();
    let rec = parse_recovering(src, &dialect);
    assert_eq!(rec.nodes, expect_nodes);
    assert_eq!(rec.comments, expect_comments);
    assert!(rec.errors.is_empty());
    check_spans(&rec.nodes, &rec.errors, src.len() as u32);
}

#[test]
fn hostile_inputs_never_panic_and_keep_spans() {
    let deep = "(".repeat(5000);
    let inputs = [
        "",
        ")",
        "))((",
        "((((",
        "\"abc",
        "(a \"b\n c)",
        "(;x",
        "(a b",
        "( ) ) (",
        "(привет \"мир",
        "😀 (😀",
        "(a\r\n\"x\r\n)",
        "\"\\",
        "(:",
        "1.5.5 -",
        &deep,
    ];
    let dialect = DialectConfig::default();
    for input in inputs {
        let rec = parse_recovering(input, &dialect);
        check_spans(&rec.nodes, &rec.errors, input.len() as u32);
    }
}
