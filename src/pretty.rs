use crate::ast::{AtomKind, Expr, Node};
use crate::trivia::Comment;

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
        AtomKind::Float(f) => {
            let s = f.to_string();
            out.push_str(&s);
            // Ensure the printed form reads back as a float (always a '.' or exponent).
            // A non-finite value (only built by hand; the lexer never yields one) prints as
            // `NaN` / `inf` / `-inf` and reads back as a symbol.
            if f.is_finite() && !s.contains('.') && !s.contains('e') && !s.contains('E') {
                out.push_str(".0");
            }
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

/// Pretty-print the tree with comment trivia interleaved.
///
/// When `comments` is empty this returns exactly what [`pretty_print`] returns.
/// Otherwise each comment is emitted on its own line, using `;`, before the node
/// that follows it in the source at that node's indentation. Comments after the
/// last element of a list stay inside the list before `)`; comments after the
/// last top-level node are emitted at the end. A comment that originally shared
/// a line with a node may move to its own line.
pub fn pretty_print_with_comments(nodes: &[Node], comments: &[Comment]) -> String {
    if comments.is_empty() {
        return pretty_print(nodes);
    }
    let mut comments: Vec<Comment> = comments.to_vec();
    comments.sort_by_key(|c| (c.span.start.byte, c.span.start.line, c.span.start.column));
    let mut cursor = 0usize;
    let mut lines: Vec<String> = Vec::new();
    for node in nodes {
        fmt_node(node, &comments, &mut cursor, 0, &mut lines);
    }
    while cursor < comments.len() {
        lines.push(format!(";{}", comments[cursor].text));
        cursor += 1;
    }
    lines.join("\n")
}

fn fmt_node(
    node: &Node,
    comments: &[Comment],
    cursor: &mut usize,
    depth: usize,
    lines: &mut Vec<String>,
) {
    let indent = indent_str(depth);
    while *cursor < comments.len() && comments[*cursor].span.end.byte <= node.span.start.byte {
        lines.push(format!("{indent};{}", comments[*cursor].text));
        *cursor += 1;
    }
    let inner_comment =
        *cursor < comments.len() && comments[*cursor].span.start.byte < node.span.end.byte;
    let children = match &node.expr {
        Expr::List(children) if inner_comment => children,
        // No comment inside: the same text as `pretty_print`.
        expr => {
            let mut s = String::new();
            fmt_expr(expr, &mut s);
            lines.push(format!("{indent}{s}"));
            return;
        }
    };
    // A list with comments inside breaks: its leading atoms stay on the `(` line until the first
    // comment, then one element per line, `)` at the end of the last element's line.
    let mut rest = &children[..];
    let mut open = format!("{indent}(");
    while let Some(first) = rest.first() {
        let comment_before = comments[*cursor].span.end.byte <= first.span.start.byte;
        if !matches!(first.expr, Expr::Atom(_)) || comment_before {
            break;
        }
        if rest.len() < children.len() {
            open.push(' ');
        }
        fmt_expr(&first.expr, &mut open);
        rest = &rest[1..];
    }
    lines.push(open);
    for child in rest {
        fmt_node(child, comments, cursor, depth + 1, lines);
    }
    let mut last_is_comment = false;
    while *cursor < comments.len() && comments[*cursor].span.start.byte < node.span.end.byte {
        lines.push(format!(
            "{};{}",
            indent_str(depth + 1),
            comments[*cursor].text
        ));
        *cursor += 1;
        last_is_comment = true;
    }
    if last_is_comment {
        lines.push(format!("{indent})"));
    } else if let Some(last) = lines.last_mut() {
        last.push(')');
    }
}

fn indent_str(depth: usize) -> String {
    "  ".repeat(depth)
}
