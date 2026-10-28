use crate::ast::{Expr, Node};

/// Return the path from the root to the innermost node whose span contains `byte`.
///
/// The span is end-inclusive: a cursor positioned right after the last character of an
/// atom still resolves to that atom. For a byte that falls between top-level forms (i.e.
/// inside no node span), the returned vector is empty.
pub fn node_at<'a>(nodes: &'a [Node], byte: u32) -> Vec<&'a Node> {
    let mut path = Vec::new();
    let mut current: &'a [Node] = nodes;
    loop {
        let mut found = None;
        for node in current {
            if node.span.start.byte <= byte && byte <= node.span.end.byte {
                found = Some(node);
                break;
            }
        }
        match found {
            Some(node) => {
                path.push(node);
                match &node.expr {
                    Expr::List(children) => current = children,
                    Expr::Atom(_) => break,
                }
            }
            None => break,
        }
    }
    path
}
