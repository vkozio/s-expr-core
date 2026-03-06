use crate::ast::{AtomKind, Expr, Node};

/// Depth-first iterator over nodes in an expression tree.
pub struct ExprIter<'a> {
    stack: Vec<std::slice::Iter<'a, Node>>,
}

impl<'a> ExprIter<'a> {
    pub fn new(nodes: &'a [Node]) -> Self {
        ExprIter {
            stack: vec![nodes.iter()],
        }
    }
}

impl<'a> Iterator for ExprIter<'a> {
    type Item = &'a Node;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let iter = self.stack.last_mut()?;
            if let Some(node) = iter.next() {
                if let Expr::List(children) = &node.expr {
                    if !children.is_empty() {
                        self.stack.push(children.iter());
                    }
                }
                return Some(node);
            } else {
                self.stack.pop();
            }
        }
    }
}

/// Iterator over all atoms in an expression tree, depth-first.
pub struct AtomIter<'a> {
    inner: ExprIter<'a>,
}

impl<'a> AtomIter<'a> {
    pub fn new(nodes: &'a [Node]) -> Self {
        AtomIter {
            inner: ExprIter::new(nodes),
        }
    }
}

impl<'a> Iterator for AtomIter<'a> {
    type Item = &'a AtomKind;

    fn next(&mut self) -> Option<Self::Item> {
        for node in self.inner.by_ref() {
            if let Expr::Atom(ref atom) = node.expr {
                return Some(atom);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::{Position, Span};

    fn dummy_span() -> Span {
        Span::new(Position::start(), Position::start())
    }

    #[test]
    fn expr_iter_traverses_depth_first() {
        // (root a (b c))
        let a = Node::atom(AtomKind::Symbol("a".to_string()), dummy_span());
        let b = Node::atom(AtomKind::Symbol("b".to_string()), dummy_span());
        let c = Node::atom(AtomKind::Symbol("c".to_string()), dummy_span());
        let inner = Node::list(vec![b.clone(), c.clone()], dummy_span());
        let root = Node::list(vec![a.clone(), inner.clone()], dummy_span());
        let nodes = vec![root.clone()];

        let kinds: Vec<&Expr> = ExprIter::new(&nodes).map(|n| &n.expr).collect();
        let labels: Vec<String> = kinds
            .into_iter()
            .map(|e| match e {
                Expr::List(_) => "list".to_string(),
                Expr::Atom(AtomKind::Symbol(s)) => s.clone(),
                other => format!("{other:?}"),
            })
            .collect();

        assert_eq!(labels, vec!["list", "a", "list", "b", "c"]);
    }

    #[test]
    fn atom_iter_visits_only_atoms_in_depth_first_order() {
        // (root a (b c))
        let a = Node::atom(AtomKind::Symbol("a".to_string()), dummy_span());
        let b = Node::atom(AtomKind::Symbol("b".to_string()), dummy_span());
        let c = Node::atom(AtomKind::Symbol("c".to_string()), dummy_span());
        let inner = Node::list(vec![b.clone(), c.clone()], dummy_span());
        let root = Node::list(vec![a.clone(), inner.clone()], dummy_span());
        let nodes = vec![root];

        let atoms: Vec<&AtomKind> = AtomIter::new(&nodes).collect();
        let symbols: Vec<&str> = atoms
            .iter()
            .map(|a| match a {
                AtomKind::Symbol(s) => s.as_str(),
                _ => panic!("expected only symbol atoms"),
            })
            .collect();

        assert_eq!(symbols, vec!["a", "b", "c"]);
    }
}
