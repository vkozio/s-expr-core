use crate::ast::{AtomKind, Node};
use crate::dialect::DialectConfig;
use crate::error::{ErrorDetails, LimitKind, ParseError, ParseErrorCode};
use crate::lexer::{Lexer, Token, TokenKind};
use crate::limits::Limits;
use crate::span::{Position, Span};
use crate::trivia::Comment;

/// Parse using the default dialect and default limits.
pub fn parse(input: &str) -> Result<Vec<Node>, ParseError> {
    parse_with_dialect(input, &DialectConfig::default())
}

/// Parse using a specific dialect and its embedded limits.
pub fn parse_with_dialect(input: &str, dialect: &DialectConfig) -> Result<Vec<Node>, ParseError> {
    parse_with_limit(input, dialect, &dialect.limits)
}

/// Parse using a specific dialect and explicit limits.
pub fn parse_with_limit(
    input: &str,
    dialect: &DialectConfig,
    limits: &Limits,
) -> Result<Vec<Node>, ParseError> {
    parse_with_limit_inner(input, dialect, limits, false).map(|(nodes, _)| nodes)
}

/// Parse using a specific dialect and its embedded limits, returning the same
/// top-level nodes as [`parse_with_dialect`] plus all comments encountered in
/// source order as trivia.
pub fn parse_with_trivia(
    input: &str,
    dialect: &DialectConfig,
) -> Result<(Vec<Node>, Vec<Comment>), ParseError> {
    parse_with_limit_inner(input, dialect, &dialect.limits, true)
}

fn handle_token(
    token: Token,
    roots: &mut Vec<Node>,
    stack: &mut Vec<(Vec<Node>, Position)>,
    total_nodes: &mut usize,
    limits: &Limits,
) -> Result<(), ParseError> {
    match token.kind {
        TokenKind::LParen => {
            enforce_depth_limit(stack, limits, token.span)?;
            stack.push((Vec::new(), token.span.start));
        }
        TokenKind::RParen => {
            let (children, start_pos) = stack.pop().ok_or_else(|| {
                ParseError::with_details(
                    ParseErrorCode::UnbalancedParen,
                    "Unmatched closing parenthesis",
                    token.span,
                    ErrorDetails {
                        expected: vec![
                            "atom".to_string(),
                            "list".to_string(),
                            "end-of-input".to_string(),
                        ],
                        found: Some(")".to_string()),
                    },
                )
            })?;

            let span = Span::new(start_pos, token.span.end);
            let node = Node::list(children, span);
            *total_nodes = total_nodes
                .checked_add(1)
                .ok_or_else(|| limit_overflow_error(LimitKind::NodeCount, span))?;
            enforce_node_limit(*total_nodes, limits, span)?;

            if let Some((ref mut parent_children, _)) = stack.last_mut() {
                enforce_list_length_limit(parent_children.len() + 1, limits, span)?;
                parent_children.push(node);
            } else {
                roots.push(node);
            }
        }
        TokenKind::Symbol(s) => push_atom(
            AtomKind::Symbol(s),
            token.span,
            roots,
            stack,
            total_nodes,
            limits,
        )?,
        TokenKind::Integer(i) => push_atom(
            AtomKind::Integer(i),
            token.span,
            roots,
            stack,
            total_nodes,
            limits,
        )?,
        TokenKind::Float(f) => push_atom(
            AtomKind::Float(f),
            token.span,
            roots,
            stack,
            total_nodes,
            limits,
        )?,
        TokenKind::String(s) => push_atom(
            AtomKind::String(s),
            token.span,
            roots,
            stack,
            total_nodes,
            limits,
        )?,
        TokenKind::Boolean(b) => push_atom(
            AtomKind::Boolean(b),
            token.span,
            roots,
            stack,
            total_nodes,
            limits,
        )?,
        TokenKind::Keyword(k) => push_atom(
            AtomKind::Keyword(k),
            token.span,
            roots,
            stack,
            total_nodes,
            limits,
        )?,
    }
    Ok(())
}

fn parse_with_limit_inner(
    input: &str,
    dialect: &DialectConfig,
    limits: &Limits,
    collect_comments: bool,
) -> Result<(Vec<Node>, Vec<Comment>), ParseError> {
    let mut lexer = Lexer::new(input, dialect).with_collect_comments(collect_comments);
    let mut roots: Vec<Node> = Vec::new();
    let mut stack: Vec<(Vec<Node>, Position)> = Vec::new();
    let mut total_nodes: usize = 0;

    while let Some(token) = lexer.next_token()? {
        handle_token(token, &mut roots, &mut stack, &mut total_nodes, limits)?;
    }

    if let Some((_, start_pos)) = stack.pop() {
        let span = Span::new(start_pos, lexer.position());
        return Err(ParseError::new(
            ParseErrorCode::UnbalancedParen,
            "Unclosed list at end of input",
            span,
        ));
    }

    Ok((roots, lexer.take_comments()))
}

fn push_atom(
    atom: AtomKind,
    span: Span,
    roots: &mut Vec<Node>,
    stack: &mut [(Vec<Node>, Position)],
    total_nodes: &mut usize,
    limits: &Limits,
) -> Result<(), ParseError> {
    let node = Node::atom(atom, span);
    *total_nodes = total_nodes
        .checked_add(1)
        .ok_or_else(|| limit_overflow_error(LimitKind::NodeCount, span))?;
    enforce_node_limit(*total_nodes, limits, span)?;

    if let Some((ref mut children, _)) = stack.last_mut() {
        enforce_list_length_limit(children.len() + 1, limits, span)?;
        children.push(node);
    } else {
        roots.push(node);
    }
    Ok(())
}

fn enforce_depth_limit(
    stack: &[(Vec<Node>, Position)],
    limits: &Limits,
    span: Span,
) -> Result<(), ParseError> {
    if let Some(max_depth) = limits.max_depth {
        let depth = stack.len() + 1; // depth including the new list we are about to push
        if depth > max_depth {
            return Err(limit_error(
                LimitKind::Depth,
                format!("Maximum list nesting depth {} exceeded", max_depth),
                span,
            ));
        }
    }
    Ok(())
}

fn enforce_list_length_limit(
    list_len_after_push: usize,
    limits: &Limits,
    span: Span,
) -> Result<(), ParseError> {
    if let Some(max_len) = limits.max_list_length {
        if list_len_after_push > max_len {
            return Err(limit_error(
                LimitKind::ListLength,
                format!("Maximum list length {} exceeded", max_len),
                span,
            ));
        }
    }
    Ok(())
}

fn enforce_node_limit(total_nodes: usize, limits: &Limits, span: Span) -> Result<(), ParseError> {
    if let Some(max_nodes) = limits.max_nodes {
        if total_nodes > max_nodes {
            return Err(limit_error(
                LimitKind::NodeCount,
                format!("Maximum node count {} exceeded", max_nodes),
                span,
            ));
        }
    }
    Ok(())
}

fn limit_error(kind: LimitKind, message: String, span: Span) -> ParseError {
    ParseError::with_details(
        ParseErrorCode::LimitExceeded(kind),
        message,
        span,
        ErrorDetails {
            expected: vec!["smaller input".to_string()],
            found: None,
        },
    )
}

fn limit_overflow_error(kind: LimitKind, span: Span) -> ParseError {
    limit_error(
        kind,
        "Overflow while counting nodes or depth".to_string(),
        span,
    )
}
