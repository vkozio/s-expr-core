use s_expr_core::{parse, pretty_print, AtomKind, Expr, Node};

#[derive(Debug, Clone, PartialEq)]
enum LogicalExpr {
    Atom(AtomKind),
    List(Vec<LogicalExpr>),
}

fn to_logical_expr(expr: &Expr) -> LogicalExpr {
    match expr {
        Expr::Atom(a) => LogicalExpr::Atom(a.clone()),
        Expr::List(children) => {
            let logical_children = children
                .iter()
                .map(|n| to_logical_expr(&n.expr))
                .collect::<Vec<LogicalExpr>>();
            LogicalExpr::List(logical_children)
        }
        Expr::Error(text) => panic!("unexpected error expr: {text}"),
    }
}

fn logical_from_nodes(nodes: &[Node]) -> Vec<LogicalExpr> {
    nodes.iter().map(|n| to_logical_expr(&n.expr)).collect()
}

#[test]
fn roundtrip_preserves_structure_for_program_like_inputs() {
    let src = r#"
        (program
          (stage
            (scan :node-type "User")
            (where (= :country "US")))
          (stage
            (scan :node-type "Order")
            (where (> :total 100))
            (return :id :user-id :total)))
    "#;

    let first = parse(src).expect("parse ok");
    let printed = pretty_print(&first);
    let second = parse(&printed).expect("re-parse ok");

    assert_eq!(
        logical_from_nodes(&first),
        logical_from_nodes(&second),
        "roundtrip logical structure mismatch for program-like input"
    );
}

#[test]
fn roundtrip_preserves_strings_and_escapes() {
    let srcs = [
        r#"(list "simple" "with\nnewline" "quote \"inner\"")"#,
        r#"(list "tab\there" "backslash\\here")"#,
    ];

    for src in srcs {
        let first = parse(src).expect("parse ok");
        let printed = pretty_print(&first);
        let second = parse(&printed).expect("re-parse ok");
        assert_eq!(
            logical_from_nodes(&first),
            logical_from_nodes(&second),
            "roundtrip mismatch for input: {}",
            src
        );
    }
}

#[test]
fn invalid_input_does_not_become_successful_roundtrip() {
    let src = "(\"foo\\q\")"; // invalid escape
    let first_err = parse(src).expect_err("first parse must fail");
    assert!(matches!(
        first_err.code,
        s_expr_core::ParseErrorCode::InvalidAtom
    ));
}
