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
