//! Lossless conversion between the s-expr AST and JSON.
//!
//! This is a generic, syntax-independent machine form of the parse tree: a
//! list is a JSON array, and every atom kind is a single-key tagged object so
//! no two kinds collide on round-trip. Floats are carried as their canonical
//! text so non-finite values (`nan`, `inf`, `-inf`) survive. Spans are not
//! part of the logical AST and are dropped; use [`crate::SourceMap`]-style
//! bookkeeping on the consumer side if spans are needed.
//!
//! Mapping:
//! - `Expr::List(ch)` → `[child, …]`
//! - `Expr::Atom(AtomKind::Boolean(b))` → `true` / `false`
//! - `Expr::Atom(AtomKind::Symbol(s))` → `{"sym": s}`
//! - `Expr::Atom(AtomKind::String(s))` → `{"str": s}`
//! - `Expr::Atom(AtomKind::Integer(n))` → `{"int": n}`
//! - `Expr::Atom(AtomKind::Float(f))` → `{"float": "<canonical text>"}`
//! - `Expr::Atom(AtomKind::Keyword(k))` → `{"kw": k}`
//! - `Expr::Error(t)` → `{"error": t}`

use serde_json::Value;

use crate::ast::{AtomKind, Expr, Node};
use crate::span::Span;

/// An error decoding a JSON value into an s-expr expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError(pub String);

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for JsonError {}

/// Canonical text of a float that reads back as the same value. Non-finite
/// values are printed as `nan` / `inf` / `-inf`.
fn float_text(f: f64) -> String {
    if f.is_nan() {
        "nan".to_string()
    } else if f.is_infinite() {
        if f.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        }
    } else {
        let s = f.to_string();
        // Ensure it round-trips as a float (not an integer-looking token).
        if !s.contains('.') && !s.contains('e') && !s.contains('E') {
            format!("{s}.0")
        } else {
            s
        }
    }
}

/// A single-key JSON object.
fn obj(key: &str, value: Value) -> Value {
    Value::Object([(key.to_string(), value)].into_iter().collect())
}

/// Encode an expression as JSON.
pub fn expr_to_value(e: &Expr) -> Value {
    match e {
        Expr::List(children) => Value::Array(children.iter().map(node_to_value).collect()),
        Expr::Atom(AtomKind::Boolean(b)) => Value::Bool(*b),
        Expr::Atom(AtomKind::Symbol(s)) => obj("sym", Value::String(s.clone())),
        Expr::Atom(AtomKind::String(s)) => obj("str", Value::String(s.clone())),
        Expr::Atom(AtomKind::Integer(n)) => obj("int", Value::Number((*n).into())),
        Expr::Atom(AtomKind::Float(f)) => obj("float", Value::String(float_text(*f))),
        Expr::Atom(AtomKind::Keyword(k)) => obj("kw", Value::String(k.clone())),
        Expr::Error(t) => obj("error", Value::String(t.clone())),
    }
}

/// Encode a node (its expression) as JSON.
pub fn node_to_value(n: &Node) -> Value {
    expr_to_value(&n.expr)
}

/// Decode a JSON value into an expression.
pub fn value_to_expr(v: &Value) -> Result<Expr, JsonError> {
    match v {
        Value::Array(items) => {
            let mut children = Vec::with_capacity(items.len());
            for item in items {
                children.push(value_to_node(item, Span::default())?);
            }
            Ok(Expr::List(children))
        }
        Value::Bool(b) => Ok(Expr::Atom(AtomKind::Boolean(*b))),
        Value::Object(map) => {
            if map.len() != 1 {
                return Err(JsonError("tagged atom must have exactly one key".into()));
            }
            let (tag, val) = map.iter().next().unwrap();
            match tag.as_str() {
                "sym" => Ok(Expr::Atom(AtomKind::Symbol(tag_string(val, "sym")?))),
                "str" => Ok(Expr::Atom(AtomKind::String(tag_string(val, "str")?))),
                "kw" => Ok(Expr::Atom(AtomKind::Keyword(tag_string(val, "kw")?))),
                "int" => {
                    let n = val
                        .as_i64()
                        .ok_or_else(|| JsonError("tagged `int` value must be an integer".into()))?;
                    Ok(Expr::Atom(AtomKind::Integer(n)))
                }
                "float" => {
                    let s = tag_string(val, "float")?;
                    let f = match s.as_str() {
                        "nan" => f64::NAN,
                        "inf" => f64::INFINITY,
                        "-inf" => f64::NEG_INFINITY,
                        other => other
                            .parse::<f64>()
                            .map_err(|_| JsonError(format!("invalid float text `{other}`")))?,
                    };
                    Ok(Expr::Atom(AtomKind::Float(f)))
                }
                "error" => Ok(Expr::Error(tag_string(val, "error")?)),
                other => Err(JsonError(format!("unknown atom tag `{other}`"))),
            }
        }
        other => Err(JsonError(format!("cannot decode {other:?} as an s-expr"))),
    }
}

fn tag_string(val: &Value, tag: &str) -> Result<String, JsonError> {
    val.as_str()
        .map(str::to_string)
        .ok_or_else(|| JsonError(format!("tagged `{tag}` value must be a string")))
}

/// Decode a JSON value into a node with the given `span`.
pub fn value_to_node(v: &Value, span: Span) -> Result<Node, JsonError> {
    Ok(Node {
        expr: value_to_expr(v)?,
        span,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    /// Encode → decode → encode is stable, and every atom kind decodes to the
    /// same kind. Spans are intentionally dropped, so structure is compared
    /// through JSON (where floats are strings, so `NaN` compares equal).
    fn assert_json_roundtrip(e: &Expr) {
        let v = expr_to_value(e);
        let back = value_to_expr(&v).expect("decode ok");
        assert_eq!(expr_to_value(&back), v, "round-trip not stable: {v}");
    }

    #[test]
    fn roundtrip_preserves_all_atom_kinds() {
        let src = r#"(foo 1 -2 3.5 "s" :kw true false (a (b c)))"#;
        let nodes = parse(src).expect("parse ok");
        assert_json_roundtrip(&nodes[0].expr);
        // The decoded kinds are exactly the source kinds.
        let v = node_to_value(&nodes[0]);
        let back = value_to_node(&v, Span::default()).expect("decode ok");
        let Expr::List(children) = &back.expr else {
            panic!("expected a list");
        };
        assert_eq!(children[0].as_symbol(), Some("foo"));
        assert_eq!(children[1].as_integer(), Some(1));
        assert_eq!(children[2].as_integer(), Some(-2));
        assert_eq!(children[3].as_float(), Some(3.5));
        assert_eq!(children[4].as_string(), Some("s"));
        assert_eq!(children[5].as_keyword(), Some("kw"));
        assert_eq!(children[6].as_bool(), Some(true));
        assert_eq!(children[7].as_bool(), Some(false));
    }

    #[test]
    fn roundtrip_preserves_nonfinite_floats() {
        let list = Expr::List(vec![
            Node::atom(AtomKind::Float(f64::NAN), Span::default()),
            Node::atom(AtomKind::Float(f64::INFINITY), Span::default()),
            Node::atom(AtomKind::Float(f64::NEG_INFINITY), Span::default()),
        ]);
        assert_json_roundtrip(&list);
        let back = value_to_expr(&expr_to_value(&list)).expect("decode ok");
        let Expr::List(children) = back else {
            panic!("expected a list");
        };
        assert!(children[0].as_float().unwrap().is_nan());
        assert_eq!(children[1].as_float(), Some(f64::INFINITY));
        assert_eq!(children[2].as_float(), Some(f64::NEG_INFINITY));
    }

    #[test]
    fn tags_are_ambiguous_free() {
        // A symbol and a keyword of the same text must not collide.
        let sym = expr_to_value(&Expr::Atom(AtomKind::Symbol("kw".into())));
        let kw = expr_to_value(&Expr::Atom(AtomKind::Keyword("kw".into())));
        assert_ne!(sym, kw);
        assert_eq!(
            value_to_expr(&sym).unwrap(),
            Expr::Atom(AtomKind::Symbol("kw".into()))
        );
        assert_eq!(
            value_to_expr(&kw).unwrap(),
            Expr::Atom(AtomKind::Keyword("kw".into()))
        );
    }
}
