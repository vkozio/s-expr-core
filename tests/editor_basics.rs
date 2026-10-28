use s_expr_core::{
    node_at, parse, parse_with_dialect, pretty_print, AtomKind, DialectConfig, Expr, LineIndex,
    Node,
};

fn atom_kinds(nodes: &[Node]) -> Vec<AtomKind> {
    fn collect(expr: &Expr, out: &mut Vec<AtomKind>) {
        match expr {
            Expr::Atom(a) => out.push(a.clone()),
            Expr::List(children) => {
                for c in children {
                    collect(&c.expr, out);
                }
            }
            Expr::Error(_) => {}
        }
    }
    let mut out = Vec::new();
    for n in nodes {
        collect(&n.expr, &mut out);
    }
    out
}

#[test]
fn floats_lex_as_float_atoms() {
    let nodes = parse("1.5 -0.25 1e9 2.5E-3").expect("parse ok");
    let kinds = atom_kinds(&nodes);
    assert_eq!(
        kinds,
        vec![
            AtomKind::Float(1.5),
            AtomKind::Float(-0.25),
            AtomKind::Float(1e9),
            AtomKind::Float(2.5E-3),
        ]
    );
}

#[test]
fn integer_stays_integer() {
    let nodes = parse("42 -7").expect("parse ok");
    let kinds = atom_kinds(&nodes);
    assert_eq!(kinds, vec![AtomKind::Integer(42), AtomKind::Integer(-7)]);
}

#[test]
fn nan_and_inf_stay_symbols() {
    let nodes = parse("nan inf Infinity").expect("parse ok");
    let kinds = atom_kinds(&nodes);
    assert_eq!(
        kinds,
        vec![
            AtomKind::Symbol("nan".into()),
            AtomKind::Symbol("inf".into()),
            AtomKind::Symbol("Infinity".into()),
        ]
    );
}

#[test]
fn floats_flag_off_lexes_as_symbols() {
    let dialect = DialectConfig {
        enable_floats: false,
        ..DialectConfig::default()
    };
    let nodes = parse_with_dialect("1.5 1e9", &dialect).expect("parse ok");
    let kinds = atom_kinds(&nodes);
    assert_eq!(
        kinds,
        vec![
            AtomKind::Symbol("1.5".into()),
            AtomKind::Symbol("1e9".into()),
        ]
    );
}

#[test]
fn pretty_float_round_trips() {
    for text in ["1.5", "-0.25", "1e9", "2.5E-3", "0.0", "1e20"] {
        let first = parse(text).expect("parse 1");
        let pretty = pretty_print(&first);
        let second = parse(&pretty).expect("parse pretty");
        assert_eq!(
            atom_kinds(&first),
            atom_kinds(&second),
            "round trip failed for {text:?} -> {pretty:?}"
        );
    }
}

#[test]
fn pretty_float_always_prints_readable_float() {
    // 1e9 prints as "1000000000" (no '.'/exponent); must become "1000000000.0".
    let nodes = parse("1e9").expect("parse ok");
    let pretty = pretty_print(&nodes);
    assert!(pretty.contains('.') || pretty.contains('e') || pretty.contains('E'));
    let reparsed = parse(&pretty).expect("reparse");
    assert_eq!(atom_kinds(&nodes), atom_kinds(&reparsed));
}

#[test]
fn node_at_finds_innermost_path() {
    // (a (b c)) d
    let nodes = parse("(a (b c)) d").expect("parse ok");

    // start of atom 'a' (byte 1): [root, a]
    let path = node_at(&nodes, 1);
    assert_eq!(path.len(), 2);
    assert!(matches!(path[0].expr, Expr::List(_)));
    assert!(matches!(path[1].expr, Expr::Atom(AtomKind::Symbol(ref s)) if s == "a"));

    // middle of atom 'b' (byte 4): [root, inner, b]
    let path = node_at(&nodes, 4);
    assert_eq!(path.len(), 3);
    assert!(matches!(path[2].expr, Expr::Atom(AtomKind::Symbol(ref s)) if s == "b"));

    // end-inclusive: right after last char of 'c' (byte 7) finds 'c'
    let path = node_at(&nodes, 7);
    assert_eq!(path.len(), 3);
    assert!(matches!(path[2].expr, Expr::Atom(AtomKind::Symbol(ref s)) if s == "c"));

    // on '(' (byte 0): only the root list
    let path = node_at(&nodes, 0);
    assert_eq!(path.len(), 1);
    assert!(matches!(path[0].expr, Expr::List(_)));

    // on ')' of inner list (byte 8): [root, inner]
    let path = node_at(&nodes, 8);
    assert_eq!(path.len(), 2);

    // between top-level forms: "a  b" -> byte 2 is strictly between the two atoms
    let nodes = parse("a  b").expect("parse ok");
    let path = node_at(&nodes, 2);
    assert!(path.is_empty());
}

#[test]
fn line_index_ascii() {
    let idx = LineIndex::new("abc\ndef\r\nghi");
    assert_eq!(idx.position(0), (0, 0));
    assert_eq!(idx.position(2), (0, 2));
    assert_eq!(idx.position(4), (1, 0));
    assert_eq!(idx.position(8), (1, 4));
    assert_eq!(idx.position(9), (2, 0));
    assert_eq!(idx.offset(0, 0), 0);
    assert_eq!(idx.offset(0, 3), 3);
    assert_eq!(idx.offset(1, 0), 4);
    assert_eq!(idx.offset(2, 1), 10);
    // offset ∘ position = identity on char boundaries
    for byte in [0u32, 1, 2, 4, 6, 8, 10, 11] {
        let (line, col) = idx.position(byte);
        assert_eq!(
            idx.offset(line, col),
            byte,
            "identity failed at byte {byte}"
        );
    }
}

#[test]
fn line_index_cyrillic() {
    // "привет" is 6 Cyrillic chars, each 2 bytes (12 bytes), 1 utf16 unit each.
    let text = "привет\nмир";
    let idx = LineIndex::new(text);
    // 'п' at byte 0, ... 'т' at byte 10, '\n' at byte 12, 'м' at byte 13.
    assert_eq!(idx.position(0), (0, 0));
    assert_eq!(idx.position(2), (0, 1));
    assert_eq!(idx.position(10), (0, 5));
    assert_eq!(idx.position(13), (1, 0));
    assert_eq!(idx.offset(0, 5), 10);
    assert_eq!(idx.offset(1, 0), 13);
    for byte in [0u32, 2, 4, 10, 12, 13, 15] {
        let (line, col) = idx.position(byte);
        assert_eq!(
            idx.offset(line, col),
            byte,
            "identity failed at byte {byte}"
        );
    }
}

#[test]
fn line_index_emoji_surrogate_pair() {
    // "a😀b": 'a'=1B, emoji=4B (utf16 col +2), 'b'=1B.
    let text = "a😀b\nx";
    let idx = LineIndex::new(text);
    assert_eq!(idx.position(1), (0, 1));
    // byte 5 is after the emoji (0:'a',1-4:emoji,5:'b').
    assert_eq!(idx.position(5), (0, 3));
    assert_eq!(idx.offset(0, 3), 5);
    assert_eq!(idx.offset(0, 1), 1);
    assert_eq!(idx.position(6), (0, 4));
    assert_eq!(idx.position(7), (1, 0));
    for byte in [0u32, 1, 5, 6, 7] {
        let (line, col) = idx.position(byte);
        assert_eq!(
            idx.offset(line, col),
            byte,
            "identity failed at byte {byte}"
        );
    }
}

#[test]
fn line_index_crlf_is_single_break() {
    let text = "ab\r\ncd";
    let idx = LineIndex::new(text);
    assert_eq!(idx.position(0), (0, 0));
    assert_eq!(idx.position(1), (0, 1));
    assert_eq!(idx.position(4), (1, 0));
    assert_eq!(idx.position(5), (1, 1));
    assert_eq!(idx.offset(0, 2), 2);
    assert_eq!(idx.offset(1, 0), 4);
    assert_eq!(idx.offset(1, 2), 6);
}

#[test]
fn overflowing_float_stays_symbol() {
    let nodes = parse("1e999").expect("parse ok");
    assert!(matches!(&nodes[0].expr, Expr::Atom(AtomKind::Symbol(s)) if s == "1e999"));
}
