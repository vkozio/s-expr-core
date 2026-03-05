use crate::ast::{AtomKind, Expr, Node};

/// Canonical pretty-printer for S-expression trees.
///
/// The formatting strategy is intentionally simple and deterministic:
/// - Top-level forms are separated by a single newline.
/// - Lists are printed as `(elem1 elem2 ...)` without extra newlines.
/// - Strings use double quotes with standard escape sequences.
pub fn pretty_print(nodes: &[Node]) -> String {
    let mut out = String::new();
    for (i, node) in nodes.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        fmt_expr(&node.expr, &mut out);
    }
    out
}

fn fmt_expr(expr: &Expr, out: &mut String) {
    match expr {
        Expr::Atom(atom) => fmt_atom(atom, out),
        Expr::List(children) => {
            out.push('(');
            for (i, child) in children.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                fmt_expr(&child.expr, out);
            }
            out.push(')');
        }
    }
}

fn fmt_atom(atom: &AtomKind, out: &mut String) {
    match atom {
        AtomKind::Symbol(s) => out.push_str(s),
        AtomKind::String(s) => {
            out.push('"');
            for ch in s.chars() {
                match ch {
                    '"' => {
                        out.push('\\');
                        out.push('"');
                    }
                    '\\' => {
                        out.push('\\');
                        out.push('\\');
                    }
                    '\n' => {
                        out.push('\\');
                        out.push('n');
                    }
                    '\r' => {
                        out.push('\\');
                        out.push('r');
                    }
                    '\t' => {
                        out.push('\\');
                        out.push('t');
                    }
                    other => out.push(other),
                }
            }
            out.push('"');
        }
        AtomKind::Integer(i) => {
            out.push_str(&i.to_string());
        }
        AtomKind::Boolean(b) => {
            out.push_str(if *b { "true" } else { "false" });
        }
        AtomKind::Keyword(k) => {
            out.push(':');
            out.push_str(k);
        }
    }
}
