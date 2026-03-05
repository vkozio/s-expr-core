use s_expr_core::{parse, AtomKind, Expr, Node};

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
