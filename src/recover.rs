use crate::ast::{AtomKind, Expr, Node};
use crate::dialect::DialectConfig;
use crate::error::{ErrorDetails, LimitKind, ParseError, ParseErrorCode};
use crate::lexer::{Token, TokenKind};
use crate::span::{Position, Span};
use crate::trivia::Comment;

/// Result of a recovering parse: a tree plus all accumulated errors.
pub struct Recovered {
    pub nodes: Vec<Node>,
    pub comments: Vec<Comment>,
    pub errors: Vec<ParseError>,
}

/// Parse `input` with best-effort recovery. Never fails and never panics.
///
/// On valid input this returns the same nodes and comments as [`crate::parse_with_trivia`]
/// with no errors. On malformed input it still produces a tree, replacing unparseable
/// pieces with [`Expr::Error`] nodes and recording every error.
pub fn parse_recovering(input: &str, dialect: &DialectConfig) -> Recovered {
    let mut scanner = Scanner::new(input, dialect);
    let (items, comments) = scanner.scan_all();
    let eof = scanner.pos;

    let mut ctx = ParserCtx {
        input,
        dialect,
        stack: Vec::new(),
        roots: Vec::new(),
        total_nodes: 0,
        errors: Vec::new(),
    };

    let mut i = 0;
    while i < items.len() {
        match &items[i] {
            ScanItem::RecordError(err) => {
                ctx.errors.push(err.clone());
                i += 1;
            }
            ScanItem::LexError { err } => {
                let span = err.span;
                let raw = input[span.start.byte as usize..span.end.byte as usize].to_string();
                ctx.errors.push(err.clone());
                let node = Node {
                    expr: Expr::Error(raw),
                    span,
                };
                ctx.place_node(node);
                i += 1;
            }
            ScanItem::Token(tok) => match tok.kind {
                TokenKind::LParen => {
                    let depth = ctx.stack.len() + 1;
                    if let Some(max) = dialect.limits.max_depth {
                        if depth > max {
                            ctx.errors.push(limit_error(
                                LimitKind::Depth,
                                format!("Maximum list nesting depth {} exceeded", max),
                                tok.span,
                            ));
                            let (raw, end_pos, next_i) =
                                skip_group(input, &items, tok.span, i + 1, eof);
                            let span = Span::new(tok.span.start, end_pos);
                            let node = Node {
                                expr: Expr::Error(raw),
                                span,
                            };
                            ctx.place_node(node);
                            i = next_i;
                            continue;
                        }
                    }
                    ctx.stack.push(Frame {
                        children: Vec::new(),
                        open: tok.span,
                    });
                    i += 1;
                }
                TokenKind::RParen => {
                    if let Some(frame) = ctx.stack.pop() {
                        let span = Span::new(frame.open.start, tok.span.end);
                        let node = Node::list(frame.children, span);
                        ctx.place_node(node);
                    } else {
                        ctx.errors.push(ParseError::with_details(
                            ParseErrorCode::UnbalancedParen,
                            "Unmatched closing parenthesis",
                            tok.span,
                            ErrorDetails {
                                expected: vec![
                                    "atom".to_string(),
                                    "list".to_string(),
                                    "end-of-input".to_string(),
                                ],
                                found: Some(")".to_string()),
                            },
                        ));
                    }
                    i += 1;
                }
                ref kind => {
                    ctx.place_atom(atom_kind(kind), tok.span);
                    i += 1;
                }
            },
        }
    }

    // EOF inside lists: report an UnbalancedParen error at every open paren in source
    // order, then close the lists innermost-first to build a tree.
    for frame in &ctx.stack {
        ctx.errors.push(ParseError::new(
            ParseErrorCode::UnbalancedParen,
            "Unclosed list at end of input",
            frame.open,
        ));
    }
    while let Some(frame) = ctx.stack.pop() {
        let span = Span::new(frame.open.start, eof);
        let node = Node::list(frame.children, span);
        ctx.place_node(node);
    }

    // Errors in source order (EOF closes lists last, but their errors point at the open parens).
    let mut errors = ctx.errors;
    errors.sort_by_key(|e| (e.span.start.byte, e.span.end.byte));
    Recovered {
        nodes: ctx.roots,
        comments,
        errors,
    }
}

fn atom_kind(kind: &TokenKind) -> AtomKind {
    match kind {
        TokenKind::Symbol(s) => AtomKind::Symbol(s.clone()),
        TokenKind::Integer(v) => AtomKind::Integer(*v),
        TokenKind::Float(f) => AtomKind::Float(*f),
        TokenKind::String(s) => AtomKind::String(s.clone()),
        TokenKind::Boolean(b) => AtomKind::Boolean(*b),
        TokenKind::Keyword(k) => AtomKind::Keyword(k.clone()),
        TokenKind::LParen | TokenKind::RParen => unreachable!("paren token passed as atom"),
    }
}

struct Frame {
    children: Vec<Node>,
    open: Span,
}

struct ParserCtx<'a> {
    input: &'a str,
    dialect: &'a DialectConfig,
    stack: Vec<Frame>,
    roots: Vec<Node>,
    total_nodes: usize,
    errors: Vec<ParseError>,
}

impl<'a> ParserCtx<'a> {
    fn place_atom(&mut self, atom: AtomKind, span: Span) {
        let node = Node::atom(atom, span);
        self.place_node(node);
    }

    fn place_node(&mut self, mut node: Node) {
        if let Some(max) = self.dialect.limits.max_nodes {
            if self.total_nodes + 1 > max {
                let raw = self.raw(node.span);
                self.errors.push(limit_error(
                    LimitKind::NodeCount,
                    format!("Maximum node count {} exceeded", max),
                    node.span,
                ));
                node = Node {
                    expr: Expr::Error(raw),
                    span: node.span,
                };
            }
        }

        let list_limit_violation = match self.dialect.limits.max_list_length {
            Some(max) => match self.stack.last() {
                Some(frame) if frame.children.len() + 1 > max => Some(max),
                _ => None,
            },
            None => None,
        };
        if let Some(max) = list_limit_violation {
            let raw = self.raw(node.span);
            self.errors.push(limit_error(
                LimitKind::ListLength,
                format!("Maximum list length {} exceeded", max),
                node.span,
            ));
            node = Node {
                expr: Expr::Error(raw),
                span: node.span,
            };
        }

        if let Some(frame) = self.stack.last_mut() {
            frame.children.push(node);
        } else {
            self.roots.push(node);
        }
        self.total_nodes += 1;
    }

    fn raw(&self, span: Span) -> String {
        self.input[span.start.byte as usize..span.end.byte as usize].to_string()
    }
}

enum ScanItem {
    Token(Token),
    LexError { err: ParseError },
    RecordError(ParseError),
}

struct Scanner<'a> {
    iter: std::str::CharIndices<'a>,
    lookahead: Option<(usize, char)>,
    pos: Position,
    dialect: &'a DialectConfig,
}

impl<'a> Scanner<'a> {
    fn new(input: &'a str, dialect: &'a DialectConfig) -> Self {
        let mut iter = input.char_indices();
        let lookahead = iter.next();
        Scanner {
            iter,
            lookahead,
            pos: Position::start(),
            dialect,
        }
    }

    fn bump(&mut self) -> Option<(usize, char)> {
        let current = self.lookahead;
        if let Some((idx, ch)) = current {
            if ch == '\n' {
                self.pos.line += 1;
                self.pos.column = 1;
            } else {
                self.pos.column += 1;
            }
            self.pos.byte = (idx + ch.len_utf8()) as u32;
        }
        self.lookahead = self.iter.next();
        current
    }

    fn peek(&self) -> Option<(usize, char)> {
        self.lookahead
    }

    fn span_from(&self, start: Position) -> Span {
        Span::new(start, self.pos)
    }

    fn skip_whitespace_and_comments(&mut self, comments: &mut Vec<Comment>) {
        loop {
            let mut advanced = false;
            while let Some((_, ch)) = self.peek() {
                if ch.is_whitespace() {
                    self.bump();
                    advanced = true;
                } else {
                    break;
                }
            }

            if self.dialect.enable_comments {
                if let Some((_, ch)) = self.peek() {
                    if ch == self.dialect.comment_prefix {
                        let start = self.pos;
                        self.bump();
                        let mut text = String::new();
                        while let Some((_, ch)) = self.peek() {
                            if ch == '\n' {
                                break;
                            }
                            self.bump();
                            text.push(ch);
                        }
                        if text.ends_with('\r') {
                            text.pop();
                        }
                        let end = self.pos;
                        comments.push(Comment {
                            text,
                            span: Span::new(start, end),
                        });
                        advanced = true;
                    }
                }
            }

            if !advanced {
                break;
            }
        }
    }

    fn scan_all(&mut self) -> (Vec<ScanItem>, Vec<Comment>) {
        let mut items = Vec::new();
        let mut comments = Vec::new();
        loop {
            self.skip_whitespace_and_comments(&mut comments);
            let (_, ch) = match self.peek() {
                Some(pair) => pair,
                None => break,
            };
            let start = self.pos;
            match ch {
                '(' => {
                    self.bump();
                    items.push(ScanItem::Token(Token {
                        kind: TokenKind::LParen,
                        span: self.span_from(start),
                    }));
                }
                ')' => {
                    self.bump();
                    items.push(ScanItem::Token(Token {
                        kind: TokenKind::RParen,
                        span: self.span_from(start),
                    }));
                }
                '"' => self.scan_string(start, &mut items),
                _ => self.scan_atom(start, &mut items),
            }
        }
        (items, comments)
    }

    fn scan_string(&mut self, start: Position, items: &mut Vec<ScanItem>) {
        self.bump();
        let mut buf = String::new();
        loop {
            match self.peek() {
                Some((_, '"')) => {
                    self.bump();
                    let span = self.span_from(start);
                    items.push(ScanItem::Token(Token {
                        kind: TokenKind::String(buf),
                        span,
                    }));
                    return;
                }
                Some((_, '\\')) => {
                    self.bump();
                    match self.peek() {
                        Some((_, esc)) => {
                            self.bump();
                            match esc {
                                'n' => buf.push('\n'),
                                'r' => buf.push('\r'),
                                't' => buf.push('\t'),
                                '\\' => buf.push('\\'),
                                '"' => buf.push('"'),
                                other => {
                                    let span = self.span_from(start);
                                    let err = ParseError::with_details(
                                        ParseErrorCode::InvalidAtom,
                                        format!("Invalid string escape '\\{}'", other),
                                        span,
                                        ErrorDetails {
                                            expected: vec!["valid escape sequence".to_string()],
                                            found: Some(other.to_string()),
                                        },
                                    );
                                    items.push(ScanItem::LexError { err });
                                    return;
                                }
                            }
                        }
                        None => {
                            let span = self.span_from(start);
                            let err = ParseError::new(
                                ParseErrorCode::UnterminatedString,
                                "Unterminated string literal",
                                span,
                            );
                            items.push(ScanItem::Token(Token {
                                kind: TokenKind::String(buf),
                                span,
                            }));
                            items.push(ScanItem::RecordError(err));
                            return;
                        }
                    }
                }
                Some((_, '\n')) | None => {
                    let span = self.span_from(start);
                    let err = ParseError::new(
                        ParseErrorCode::UnterminatedString,
                        "Unterminated string literal",
                        span,
                    );
                    items.push(ScanItem::Token(Token {
                        kind: TokenKind::String(buf),
                        span,
                    }));
                    items.push(ScanItem::RecordError(err));
                    return;
                }
                Some((_, ch)) => {
                    self.bump();
                    buf.push(ch);
                }
            }
        }
    }

    fn scan_atom(&mut self, start: Position, items: &mut Vec<ScanItem>) {
        let mut buf = String::new();
        loop {
            let ch_opt = self.peek().map(|(_, c)| c);
            match ch_opt {
                Some(ch)
                    if !ch.is_whitespace()
                        && ch != '('
                        && ch != ')'
                        && (!self.dialect.enable_comments || ch != self.dialect.comment_prefix) =>
                {
                    self.bump();
                    buf.push(ch);
                }
                _ => break,
            }
        }
        let span = self.span_from(start);

        if buf.is_empty() {
            let err = ParseError::with_details(
                ParseErrorCode::InvalidAtom,
                "Failed to lex atom",
                span,
                ErrorDetails {
                    expected: vec!["atom".to_string()],
                    found: None,
                },
            );
            items.push(ScanItem::LexError { err });
            return;
        }

        let kind = if self.dialect.enable_booleans && (buf == "true" || buf == "false") {
            TokenKind::Boolean(buf == "true")
        } else if self.dialect.enable_keywords && buf.starts_with(':') && buf.len() > 1 {
            TokenKind::Keyword(buf[1..].to_string())
        } else if let Some(kind) = try_lex_integer(&buf) {
            kind
        } else if self.dialect.enable_floats {
            if let Some(kind) = try_lex_float(&buf) {
                kind
            } else {
                TokenKind::Symbol(buf)
            }
        } else {
            TokenKind::Symbol(buf)
        };
        items.push(ScanItem::Token(Token { kind, span }));
    }
}

fn try_lex_integer(text: &str) -> Option<TokenKind> {
    let mut chars = text.chars();
    if let Some(first) = chars.next() {
        if first == '-' {
            if chars.clone().all(|c| c.is_ascii_digit()) {
                if let Ok(value) = text.parse::<i64>() {
                    return Some(TokenKind::Integer(value));
                }
            }
        } else if first.is_ascii_digit() && chars.all(|c| c.is_ascii_digit()) {
            if let Ok(value) = text.parse::<i64>() {
                return Some(TokenKind::Integer(value));
            }
        }
    }
    None
}

fn try_lex_float(text: &str) -> Option<TokenKind> {
    let mut chars = text.chars();
    let first = chars.next()?;
    let starts_with_digit = match first {
        '-' => chars.next()?.is_ascii_digit(),
        c => c.is_ascii_digit(),
    };
    if !starts_with_digit {
        return None;
    }
    if !text.contains('.') && !text.contains('e') && !text.contains('E') {
        return None;
    }
    if let Ok(v) =
        text.parse::<f64>().map_err(|_| ()).and_then(
            |v| {
                if v.is_finite() {
                    Ok(v)
                } else {
                    Err(())
                }
            },
        )
    {
        return Some(TokenKind::Float(v));
    }
    None
}

/// Skip the balanced group opened at `open`, returning its raw text, the position just
/// past the matching `)`, and the index of the first item after that `)`.
fn skip_group(
    input: &str,
    items: &[ScanItem],
    open: Span,
    mut idx: usize,
    eof: Position,
) -> (String, Position, usize) {
    let mut depth = 1usize;
    loop {
        if idx >= items.len() {
            let raw = &input[open.start.byte as usize..eof.byte as usize];
            return (raw.to_string(), eof, idx);
        }
        if let ScanItem::Token(tok) = &items[idx] {
            match tok.kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        let raw = &input[open.start.byte as usize..tok.span.end.byte as usize];
                        return (raw.to_string(), tok.span.end, idx + 1);
                    }
                }
                _ => {}
            }
        }
        idx += 1;
    }
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
