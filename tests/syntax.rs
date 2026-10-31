use s_expr_core::{
    parse, parse_with_trivia, pretty_print_with_comments, AtomKind, Expr, Node, ParseErrorCode,
};

fn expect_single_list(src: &str) -> Vec<Node> {
    let nodes = parse(src).expect("parse ok");
    assert_eq!(nodes.len(), 1, "expected a single top-level form");
    nodes
}

#[test]
fn accepts_core_syntax_constructs() {
    let src = r#"
        ; program with all core atom categories and nesting
        (program
          (stage
            (scan :node-type "User")
            (where (= :country "US" 42 true false)))
          (stage
            (scan :node-type "Order")
            (where (> :total 100))
            (return :id :user-id :total)))
    "#;

    let nodes = expect_single_list(src);
    let root = &nodes[0];
    match &root.expr {
        Expr::List(children) => {
            // (program <stage> <stage>)
            assert_eq!(children.len(), 3);
            // First child is the symbol "program"
            match &children[0].expr {
                Expr::Atom(AtomKind::Symbol(s)) => assert_eq!(s, "program"),
                other => panic!("expected symbol atom, got {:?}", other),
            }
        }
        other => panic!("expected list at root, got {:?}", other),
    }
}

#[test]
fn accepts_comments_and_whitespace_as_ws() {
    let src = "  ; leading comment\n (a  ; inner comment\n  b)\n  ; trailing comment\n";
    let nodes = expect_single_list(src);

    match &nodes[0].expr {
        Expr::List(children) => {
            assert_eq!(children.len(), 2);
            for child in children {
                match &child.expr {
                    Expr::Atom(AtomKind::Symbol(s)) => assert!(s == "a" || s == "b"),
                    other => panic!("expected simple symbol atoms, got {:?}", other),
                }
            }
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn block_comments_are_skipped() {
    let src = "(a #| hi |# b)";
    let nodes = expect_single_list(src);
    match &nodes[0].expr {
        Expr::List(children) => {
            assert_eq!(children.len(), 2);
            for child in children {
                match &child.expr {
                    Expr::Atom(AtomKind::Symbol(s)) => assert!(s == "a" || s == "b"),
                    other => panic!("expected simple symbol atoms, got {:?}", other),
                }
            }
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn block_comments_nest() {
    let src = "(a #| x #| y |# z |# b)";
    let nodes = expect_single_list(src);
    match &nodes[0].expr {
        Expr::List(children) => {
            assert_eq!(children.len(), 2);
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn block_comments_can_span_lines() {
    let src = "(a #| line1\nline2\r\nline3 |# b)";
    let nodes = expect_single_list(src);
    match &nodes[0].expr {
        Expr::List(children) => assert_eq!(children.len(), 2),
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn block_comment_splits_an_atom() {
    // `foo#| … |#bar` lexes as two symbols, not a single atom.
    let src = "foo#| hidden |#bar";
    let nodes = parse(src).expect("parse ok");
    assert_eq!(nodes.len(), 2);
    let atom = |i: usize| match &nodes[i].expr {
        Expr::Atom(AtomKind::Symbol(s)) => s.clone(),
        other => panic!("expected symbol, got {:?}", other),
    };
    assert_eq!(atom(0), "foo");
    assert_eq!(atom(1), "bar");
}

#[test]
fn rejects_unterminated_block_comment() {
    let src = "(a #| never closed";
    let err = parse(src).expect_err("parse should fail on unterminated block comment");
    assert!(
        matches!(err.code, ParseErrorCode::UnterminatedBlockComment),
        "unexpected error code: {:?}",
        err.code
    );
}

#[test]
fn block_comments_roundtrip_through_pretty_with_comments() {
    let src = "(a #| x |# b)";
    let (nodes, comments) = parse_with_trivia(src, &s_expr_core::DialectConfig::default())
        .expect("parse with trivia ok");
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].kind, s_expr_core::CommentKind::Block);
    let printed = pretty_print_with_comments(&nodes, &comments);
    let (nodes2, comments2) =
        parse_with_trivia(&printed, &s_expr_core::DialectConfig::default()).expect("reparse ok");
    fn syms(ns: &[Node]) -> Vec<&str> {
        match &ns[0].expr {
            Expr::List(children) => children
                .iter()
                .filter_map(|n| match &n.expr {
                    Expr::Atom(AtomKind::Symbol(s)) => Some(s.as_str()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }
    assert_eq!(
        syms(&nodes),
        syms(&nodes2),
        "nodes differ; printed:\n{printed}"
    );
    let body = |c: &s_expr_core::Comment| (c.kind, c.text.clone());
    assert_eq!(
        comments.iter().map(body).collect::<Vec<_>>(),
        comments2.iter().map(body).collect::<Vec<_>>(),
        "comments differ; printed:\n{printed}"
    );
}

#[test]
fn rejects_unbalanced_parens() {
    let src = "(a (b c)";
    let err = parse(src).expect_err("parse should fail on unbalanced parens");
    assert!(
        matches!(err.code, s_expr_core::ParseErrorCode::UnbalancedParen),
        "unexpected error code: {:?}",
        err.code
    );
}

#[test]
fn rejects_unterminated_string_literal() {
    let src = "(\"foo";
    let err = parse(src).expect_err("parse should fail on unterminated string");
    assert!(
        matches!(err.code, s_expr_core::ParseErrorCode::UnterminatedString),
        "unexpected error code: {:?}",
        err.code
    );
}

#[test]
fn rejects_invalid_string_escape_sequence() {
    let src = "(\"foo\\q\")";
    let err = parse(src).expect_err("parse should fail on invalid escape");
    assert!(
        matches!(err.code, s_expr_core::ParseErrorCode::InvalidAtom),
        "unexpected error code: {:?}",
        err.code
    );
}
