use wasm_bindgen::prelude::*;

use serde::Serialize;
use serde_wasm_bindgen::to_value;

use s_expr_core::{
    parse, pretty_print, AtomKind, Expr, Node, ParseError, ParseErrorCode, Position, Span,
};

// JS-facing shapes

#[derive(Serialize)]
struct JsPosition {
    byte: u32,
    line: u32,
    column: u32,
}

#[derive(Serialize)]
struct JsSpan {
    start: JsPosition,
    end: JsPosition,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum JsAtom {
    Symbol { value: String },
    String { value: String },
    Integer { value: i64 },
    Float { value: f64 },
    Boolean { value: bool },
    Keyword { value: String },
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum JsExpr {
    Atom { atom: JsAtom },
    List { children: Vec<JsNode> },
}

#[derive(Serialize)]
struct JsNode {
    expr: JsExpr,
    span: JsSpan,
}

#[derive(Serialize)]
struct JsError {
    code: String,
    message: String,
    span: JsSpan,
}

// Converters

fn js_position(pos: Position) -> JsPosition {
    JsPosition {
        byte: pos.byte,
        line: pos.line,
        column: pos.column,
    }
}

fn js_span(span: Span) -> JsSpan {
    JsSpan {
        start: js_position(span.start),
        end: js_position(span.end),
    }
}

fn js_atom(atom: AtomKind) -> JsAtom {
    match atom {
        AtomKind::Symbol(s) => JsAtom::Symbol { value: s },
        AtomKind::String(s) => JsAtom::String { value: s },
        AtomKind::Integer(i) => JsAtom::Integer { value: i },
        AtomKind::Float(f) => JsAtom::Float { value: f },
        AtomKind::Boolean(b) => JsAtom::Boolean { value: b },
        AtomKind::Keyword(k) => JsAtom::Keyword { value: k },
    }
}

fn js_expr(expr: Expr, span: Span) -> JsExpr {
    match expr {
        Expr::Atom(atom) => JsExpr::Atom {
            atom: js_atom(atom),
        },
        Expr::List(children) => {
            let js_children = children.into_iter().map(js_node).collect::<Vec<JsNode>>();
            JsExpr::List {
                children: js_children,
            }
        }
    }
}

fn js_node(node: Node) -> JsNode {
    let span = node.span;
    let expr = js_expr(node.expr, span);
    JsNode {
        expr,
        span: js_span(span),
    }
}

fn js_error(err: ParseError) -> JsValue {
    let code = match err.code {
        ParseErrorCode::UnexpectedEof => "UnexpectedEof",
        ParseErrorCode::UnbalancedParen => "UnbalancedParen",
        ParseErrorCode::UnterminatedString => "UnterminatedString",
        ParseErrorCode::InvalidAtom => "InvalidAtom",
        ParseErrorCode::LimitExceeded(_) => "LimitExceeded",
    }
    .to_string();

    let js_err = JsError {
        code,
        message: err.to_string(),
        span: js_span(err.span),
    };

    to_value(&js_err).unwrap_or_else(|_| JsValue::from_str("failed to serialize error"))
}

// Public WASM API

/// Parse S-expressions into a JSON-serializable tree.
///
/// Returns `Ok(JsValue)` with an array of nodes on success,
/// or `Err(JsValue)` with a structured error object on failure.
#[wasm_bindgen]
pub fn parse_s_expr(input: &str) -> Result<JsValue, JsValue> {
    match parse(input) {
        Ok(nodes) => {
            let js_nodes = nodes.into_iter().map(js_node).collect::<Vec<JsNode>>();
            to_value(&js_nodes).map_err(|e| JsValue::from_str(&format!("serialization error: {e}")))
        }
        Err(err) => Err(js_error(err)),
    }
}

/// Canonical pretty-printer for S-expressions.
///
/// This is equivalent to `pretty_print(parse(input)?)` and preserves
/// the logical tree modulo canonical whitespace and string escaping.
#[wasm_bindgen]
pub fn pretty_print_s_expr(input: &str) -> Result<String, JsValue> {
    let nodes = parse(input).map_err(js_error)?;
    Ok(pretty_print(&nodes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_position_and_span_match_core_values() {
        let start = Position::start();
        let end = Position {
            byte: 5,
            line: 1,
            column: 6,
        };
        let span = Span::new(start, end);

        let js_pos = js_position(start);
        assert_eq!(js_pos.byte, start.byte);
        assert_eq!(js_pos.line, start.line);
        assert_eq!(js_pos.column, start.column);

        let js_span = js_span(span);
        assert_eq!(js_span.start.byte, start.byte);
        assert_eq!(js_span.end.byte, end.byte);
    }

    #[test]
    fn js_atom_and_node_structure() {
        let span = Span::new(Position::start(), Position::start());

        let int_atom = AtomKind::Integer(42);
        let kw_atom = AtomKind::Keyword("kw".to_string());

        let int_node = Node {
            expr: Expr::Atom(int_atom),
            span,
        };
        let kw_node = Node {
            expr: Expr::Atom(kw_atom),
            span,
        };
        let list_node = Node {
            expr: Expr::List(vec![int_node, kw_node]),
            span,
        };

        let js = js_node(list_node);
        match js.expr {
            JsExpr::List { children } => {
                assert_eq!(children.len(), 2);
                match &children[0].expr {
                    JsExpr::Atom {
                        atom: JsAtom::Integer { value },
                    } => assert_eq!(*value, 42),
                    _ => panic!("expected integer atom"),
                }
                match &children[1].expr {
                    JsExpr::Atom {
                        atom: JsAtom::Keyword { value },
                    } => assert_eq!(value, "kw"),
                    _ => panic!("expected keyword atom"),
                }
            }
            _ => panic!("expected list expr"),
        }
    }
}
