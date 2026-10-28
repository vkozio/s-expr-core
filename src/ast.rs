use crate::span::Span;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum AtomKind {
    Symbol(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Keyword(String),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    List(Vec<Node>),
    Atom(AtomKind),
    /// Raw text of an unparseable piece (produced only by recovering parsing).
    Error(String),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub expr: Expr,
    pub span: Span,
}

impl Node {
    pub fn list(children: Vec<Node>, span: Span) -> Self {
        Node {
            expr: Expr::List(children),
            span,
        }
    }

    pub fn atom(kind: AtomKind, span: Span) -> Self {
        Node {
            expr: Expr::Atom(kind),
            span,
        }
    }
}
