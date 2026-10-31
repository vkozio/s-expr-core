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

    /// The children if this node is a list, else `None`.
    pub fn as_list(&self) -> Option<&[Node]> {
        match &self.expr {
            Expr::List(children) => Some(children),
            _ => None,
        }
    }

    /// Whether this node is a list.
    pub fn is_list(&self) -> bool {
        self.as_list().is_some()
    }

    /// The symbol text if this node is a symbol atom, else `None`.
    pub fn as_symbol(&self) -> Option<&str> {
        match &self.expr {
            Expr::Atom(AtomKind::Symbol(s)) => Some(s),
            _ => None,
        }
    }

    /// The keyword text (without the leading `:`) if this node is a keyword
    /// atom, else `None`.
    pub fn as_keyword(&self) -> Option<&str> {
        match &self.expr {
            Expr::Atom(AtomKind::Keyword(k)) => Some(k),
            _ => None,
        }
    }

    /// The string contents if this node is a string atom, else `None`.
    pub fn as_string(&self) -> Option<&str> {
        match &self.expr {
            Expr::Atom(AtomKind::String(s)) => Some(s),
            _ => None,
        }
    }

    /// The value if this node is an integer atom, else `None`.
    pub fn as_integer(&self) -> Option<i64> {
        match &self.expr {
            Expr::Atom(AtomKind::Integer(n)) => Some(*n),
            _ => None,
        }
    }

    /// The value if this node is a float atom, else `None`.
    pub fn as_float(&self) -> Option<f64> {
        match &self.expr {
            Expr::Atom(AtomKind::Float(f)) => Some(*f),
            _ => None,
        }
    }

    /// The value if this node is a boolean atom, else `None`.
    pub fn as_bool(&self) -> Option<bool> {
        match &self.expr {
            Expr::Atom(AtomKind::Boolean(b)) => Some(*b),
            _ => None,
        }
    }
}
