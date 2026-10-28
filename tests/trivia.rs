use s_expr_core::{
    parse_with_trivia, pretty_print, pretty_print_with_comments, Comment, DialectConfig, Expr,
    Node, Position, Span,
};

fn strip(node: &Node) -> Node {
    let expr = match &node.expr {
        Expr::Atom(a) => Expr::Atom(a.clone()),
        Expr::List(children) => Expr::List(children.iter().map(strip).collect()),
    };
    Node {
        expr,
        span: Span::new(Position::start(), Position::start()),
    }
}

fn roundtrip(input: &str) {
    let dialect = DialectConfig::default();
    let (nodes, comments) = parse_with_trivia(input, &dialect).unwrap();
    let out = pretty_print_with_comments(&nodes, &comments);
    let (nodes2, comments2) = parse_with_trivia(&out, &dialect).unwrap();
    let a: Vec<Node> = nodes.iter().map(strip).collect();
    let b: Vec<Node> = nodes2.iter().map(strip).collect();
    assert_eq!(a, b, "nodes differ for input {:?}\noutput:\n{}", input, out);
    let texts: Vec<&str> = comments.iter().map(|c| c.text.as_str()).collect();
    let texts2: Vec<&str> = comments2.iter().map(|c| c.text.as_str()).collect();
    assert_eq!(
        texts, texts2,
        "comment texts differ for input {:?}\noutput:\n{}",
        input, out
    );
}

fn texts(comments: &[Comment]) -> Vec<&str> {
    comments.iter().map(|c| c.text.as_str()).collect()
}

#[test]
fn comment_at_file_start() {
    roundtrip("; hello\n(a b c)");
    let dialect = DialectConfig::default();
    let (_, comments) = parse_with_trivia("; hello\n(a b c)", &dialect).unwrap();
    assert_eq!(texts(&comments), vec![" hello"]);
}

#[test]
fn comment_between_top_level_forms() {
    roundtrip("(a)\n; note\n(b c)");
    let dialect = DialectConfig::default();
    let (_, comments) = parse_with_trivia("(a)\n; note\n(b c)", &dialect).unwrap();
    assert_eq!(texts(&comments), vec![" note"]);
}

#[test]
fn comment_between_list_elements() {
    roundtrip("(a ; mid\n b c)");
    let dialect = DialectConfig::default();
    let (_, comments) = parse_with_trivia("(a ; mid\n b c)", &dialect).unwrap();
    assert_eq!(texts(&comments), vec![" mid"]);
}

#[test]
fn comment_before_closing_paren() {
    roundtrip("(a b ; tail\n)");
    let dialect = DialectConfig::default();
    let (_, comments) = parse_with_trivia("(a b ; tail\n)", &dialect).unwrap();
    assert_eq!(texts(&comments), vec![" tail"]);
}

#[test]
fn comment_in_nested_lists() {
    roundtrip("(outer ; before inner\n (inner ; inside\n x))");
    let dialect = DialectConfig::default();
    let (_, comments) =
        parse_with_trivia("(outer ; before inner\n (inner ; inside\n x))", &dialect).unwrap();
    assert_eq!(texts(&comments), vec![" before inner", " inside"]);
}

#[test]
fn comment_at_file_end() {
    roundtrip("(a b)\n; done");
    let dialect = DialectConfig::default();
    let (_, comments) = parse_with_trivia("(a b)\n; done", &dialect).unwrap();
    assert_eq!(texts(&comments), vec![" done"]);
}

#[test]
fn comment_with_crlf() {
    roundtrip("(a\r\n; note\r\n b)");
    let dialect = DialectConfig::default();
    let (_, comments) = parse_with_trivia("(a\r\n; note\r\n b)", &dialect).unwrap();
    assert_eq!(texts(&comments), vec![" note"]);
}

#[test]
fn multiple_comments_preserved_in_order() {
    roundtrip("; one\n(a ; two\n b ; three\n)\n; four");
    let dialect = DialectConfig::default();
    let (_, comments) =
        parse_with_trivia("; one\n(a ; two\n b ; three\n)\n; four", &dialect).unwrap();
    assert_eq!(texts(&comments), vec![" one", " two", " three", " four"]);
}

#[test]
fn no_comment_prefix_in_comment() {
    roundtrip("(a ; semi ; colon\n b)");
}

#[test]
fn empty_comment() {
    roundtrip("(a ;\n b)");
    let dialect = DialectConfig::default();
    let (_, comments) = parse_with_trivia("(a ;\n b)", &dialect).unwrap();
    assert_eq!(texts(&comments), vec![""]);
}

#[test]
fn empty_comment_list_equals_pretty_print() {
    let dialect = DialectConfig::default();
    let input = "(foo (bar baz) \"str\" :kw 1 -2 3.5 true)";
    let (nodes, comments) = parse_with_trivia(input, &dialect).unwrap();
    assert!(comments.is_empty());
    assert_eq!(
        pretty_print_with_comments(&nodes, &comments),
        pretty_print(&nodes)
    );
}

#[test]
fn layout_inline_without_inner_comments_breaks_with_them() {
    let dialect = DialectConfig::default();
    let src = "; top\n(rule r1 (a x) (b x))\n(rule r2\n  ; why\n  (a x) (c x))\n";
    let (nodes, comments) = parse_with_trivia(src, &dialect).expect("parse ok");
    let out = pretty_print_with_comments(&nodes, &comments);
    assert_eq!(
        out,
        "; top\n(rule r1 (a x) (b x))\n(rule r2\n  ; why\n  (a x)\n  (c x))"
    );
}
