use crate::error::ParseErrorCode;
use crate::{parse, pretty_print, AtomKind, DialectConfig, Expr, Limits, Node};

#[test]
fn parses_simple_list_with_spans() {
    let src = "(a b)";
    let nodes = parse(src).expect("parse ok");
    assert_eq!(nodes.len(), 1);
    let list = &nodes[0];
    assert!(list.span.start.byte < list.span.end.byte);
    assert_eq!(list.span.start.byte, 0);
    assert_eq!(list.span.end.byte, src.len() as u32);

    match &list.expr {
        Expr::List(children) => {
            assert_eq!(children.len(), 2);
            for child in children {
                assert!(child.span.start.byte < child.span.end.byte);
            }
        }
        _ => panic!("expected list"),
    }
}

#[test]
fn parses_comments_and_whitespace_in_default_dialect() {
    let src = "  ; leading comment\n (a  ; inner comment\n  b)\n  ; trailing comment\n";
    let nodes = parse(src).expect("parse ok");
    assert_eq!(nodes.len(), 1);
    match &nodes[0].expr {
        Expr::List(children) => {
            assert_eq!(children.len(), 2);
        }
        _ => panic!("expected single list"),
    }
}

#[test]
fn supports_booleans_and_keywords_in_default_dialect() {
    let nodes = parse("(true false :kw)").expect("parse ok");
    assert_eq!(nodes.len(), 1);
    match &nodes[0].expr {
        Expr::List(children) => {
            assert_eq!(children.len(), 3);
            let atoms: Vec<&AtomKind> = children
                .iter()
                .map(|n| match &n.expr {
                    Expr::Atom(a) => a,
                    _ => panic!("expected atom"),
                })
                .collect();
            assert!(matches!(atoms[0], AtomKind::Boolean(true)));
            assert!(matches!(atoms[1], AtomKind::Boolean(false)));
            assert!(matches!(atoms[2], AtomKind::Keyword(k) if k == "kw"));
        }
        _ => panic!("expected list"),
    }
}

#[test]
fn dialect_can_disable_comments() {
    let dialect = DialectConfig {
        enable_comments: false,
        ..DialectConfig::default()
    };
    let nodes = crate::parse_with_dialect("(foo;bar)", &dialect).expect("parse ok");
    assert_eq!(nodes.len(), 1);
    match &nodes[0].expr {
        Expr::List(children) => {
            assert_eq!(children.len(), 1);
            match &children[0].expr {
                Expr::Atom(AtomKind::Symbol(s)) => assert_eq!(s, "foo;bar"),
                _ => panic!("expected single symbol atom"),
            }
        }
        _ => panic!("expected list"),
    }
}

#[test]
fn detects_unbalanced_paren() {
    let err = parse("(a b").expect_err("should fail");
    assert!(matches!(err.code, ParseErrorCode::UnbalancedParen));
    assert!(err.span.start.byte < err.span.end.byte);
}

#[test]
fn detects_unterminated_string() {
    let err = parse("(\"foo").expect_err("should fail");
    assert!(matches!(err.code, ParseErrorCode::UnterminatedString));
    assert!(err.span.start.byte < err.span.end.byte);
}

#[test]
fn detects_invalid_string_escape_with_details() {
    let err = parse("(\"foo\\q\")").expect_err("should fail");
    assert!(matches!(err.code, ParseErrorCode::InvalidAtom));
    let details = err.details.expect("expected error details");
    assert!(details
        .expected
        .iter()
        .any(|s| s == "valid escape sequence"));
    assert_eq!(details.found.as_deref(), Some("q"));
}

#[test]
fn enforces_depth_limit() {
    let dialect = DialectConfig {
        limits: Limits {
            max_depth: Some(1),
            max_list_length: None,
            max_nodes: None,
        },
        ..DialectConfig::default()
    };
    let err = crate::parse_with_dialect("(a (b))", &dialect).expect_err("should fail");
    assert!(matches!(err.code, ParseErrorCode::LimitExceeded(_)));
}

#[test]
fn enforces_list_length_limit() {
    let dialect = DialectConfig {
        limits: Limits {
            max_depth: None,
            max_list_length: Some(1),
            max_nodes: None,
        },
        ..DialectConfig::default()
    };
    let err = crate::parse_with_dialect("(a b)", &dialect).expect_err("should fail");
    assert!(matches!(err.code, ParseErrorCode::LimitExceeded(_)));
}

#[test]
fn enforces_node_count_limit() {
    let dialect = DialectConfig {
        limits: Limits {
            max_depth: None,
            max_list_length: None,
            max_nodes: Some(1),
        },
        ..DialectConfig::default()
    };
    let err = crate::parse_with_dialect("(a b)", &dialect).expect_err("should fail");
    assert!(matches!(err.code, ParseErrorCode::LimitExceeded(_)));
}

#[test]
fn roundtrip_preserves_structure() {
    let src = "(foo 1 \"bar\" (true :kw))";
    let first = parse(src).expect("parse ok");
    let printed = pretty_print(&first);
    let second = parse(&printed).expect("parse ok");
    assert_eq!(first.len(), second.len());
    // logical structure equality ignoring spans
    fn strip_spans(nodes: &[Node]) -> Vec<Expr> {
        nodes.iter().map(|n| n.expr.clone()).collect()
    }
    assert_eq!(strip_spans(&first), strip_spans(&second));
}

#[test]
fn pretty_print_produces_canonical_whitespace() {
    let src = "(  foo\t1\n\"bar\"  ( true  :kw ) )";
    let nodes = parse(src).expect("parse ok");
    let printed = pretty_print(&nodes);
    assert_eq!(printed, "(foo 1 \"bar\" (true :kw))");
}

#[test]
fn node_accessors_decode_atom_kinds() {
    let list = parse("(a \"s\" 5 :kw true)").expect("parse ok");
    let children = list[0].as_list().expect("list");
    assert!(!children[0].is_list());
    assert_eq!(children[0].as_symbol(), Some("a"));
    assert_eq!(children[1].as_string(), Some("s"));
    assert_eq!(children[2].as_integer(), Some(5));
    assert_eq!(children[3].as_keyword(), Some("kw"));
    assert_eq!(children[4].as_bool(), Some(true));
    assert_eq!(children[3].as_symbol(), None);
    assert_eq!(children[0].as_keyword(), None);
}

#[test]
fn escape_string_roundtrips_through_parser() {
    let s = "a\\b\nc\rd\te";
    let escaped = crate::pretty::escape_string(s);
    let src = format!("(\"{escaped}\")");
    let nodes = parse(&src).expect("parse ok");
    assert_eq!(nodes[0].as_list().unwrap()[0].as_string(), Some(s));
}
