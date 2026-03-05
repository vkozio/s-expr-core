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
