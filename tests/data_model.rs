use s_expr_core::{parse, Expr, Node, Span};

fn collect_all_nodes<'a>(nodes: &'a [Node], out: &mut Vec<&'a Node>) {
    for node in nodes {
        out.push(node);
        if let Expr::List(children) = &node.expr {
            collect_all_nodes(children, out);
        }
    }
}

#[test]
fn every_node_has_non_empty_span_within_input() {
    let src = "(program (stage (scan :node-type \"User\") (where (= :country \"US\"))))";
    let nodes = parse(src).expect("parse ok");
    let mut all: Vec<&Node> = Vec::new();
    collect_all_nodes(&nodes, &mut all);

    let len = src.len() as u32;
    for node in all {
        let Span { start, end } = node.span;
        assert!(
            start.byte < end.byte,
            "expected non-empty span, got start={:?}, end={:?}",
            start,
            end
        );
        assert!(
            end.byte <= len,
            "span end {:?} must be within input len {}",
            end,
            len
        );
        assert!(
            start.byte < len,
            "span start {:?} must be within input len {}",
            start,
            len
        );
    }
}

#[test]
fn list_spans_enclose_child_spans_and_preserve_order() {
    let src = "(root a (b c) d)";
    let nodes = parse(src).expect("parse ok");
    assert_eq!(nodes.len(), 1);

    let root = &nodes[0];
    let root_span = root.span;
    let children = match &root.expr {
        Expr::List(children) => children,
        other => panic!("expected list at root, got {:?}", other),
    };

    // Order invariants: (root a (b c) d)
    assert_eq!(children.len(), 4);
    match &children[0].expr {
        Expr::Atom(_) => {}
        other => panic!("expected atom in position 0, got {:?}", other),
    }
    match &children[1].expr {
        Expr::Atom(_) => {}
        other => panic!("expected atom in position 1, got {:?}", other),
    }
    match &children[2].expr {
        Expr::List(nested) => {
            assert_eq!(nested.len(), 2);
        }
        other => panic!("expected nested list in position 2, got {:?}", other),
    }
    match &children[3].expr {
        Expr::Atom(_) => {}
        other => panic!("expected atom in position 3, got {:?}", other),
    }

    // Child spans must be inside parent span.
    for child in children {
        assert!(
            child.span.start.byte >= root_span.start.byte,
            "child start must be >= root start"
        );
        assert!(
            child.span.end.byte <= root_span.end.byte,
            "child end must be <= root end"
        );
    }
}
