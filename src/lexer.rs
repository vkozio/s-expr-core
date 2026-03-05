use crate::dialect::DialectConfig;
use crate::error::{ErrorDetails, ParseError, ParseErrorCode};
use crate::span::{Position, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    LParen,
    RParen,
    Symbol(String),
    Integer(i64),
    String(String),
    Boolean(bool),
    Keyword(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub struct Lexer<'a> {
    iter: std::str::CharIndices<'a>,
    lookahead: Option<(usize, char)>,
    pos: Position,
    dialect: &'a DialectConfig,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str, dialect: &'a DialectConfig) -> Self {
        let mut iter = input.char_indices();
        let lookahead = iter.next();
        Lexer {
            iter,
            lookahead,
            pos: Position::start(),
            dialect,
        }
    }

    pub fn position(&self) -> Position {
        self.pos
    }

    fn bump(&mut self) -> Option<(usize, char)> {
        let current = self.lookahead;
        if let Some((idx, ch)) = current {
            // advance position according to ch
            if ch == '\n' {
                self.pos.line += 1;
                self.pos.column = 1;
            } else {
                self.pos.column += 1;
            }
            // byte offset always reflects the index of the *next* character
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

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            let mut advanced = false;
            while let Some((_idx, ch)) = self.peek() {
                if ch.is_whitespace() {
                    self.bump();
                    advanced = true;
                } else {
                    break;
                }
            }

            if self.dialect.enable_comments {
                if let Some((_idx, ch)) = self.peek() {
                    if ch == self.dialect.comment_prefix {
                        // consume until end of line or input
                        while let Some((_i, ch)) = self.bump() {
                            if ch == '\n' {
                                break;
                            }
                        }
                        advanced = true;
                    }
                }
            }

            if !advanced {
                break;
            }
        }
    }

    pub fn next_token(&mut self) -> Result<Option<Token>, ParseError> {
        self.skip_whitespace_and_comments();

        let start_pos = self.pos;
        let (_idx, ch) = match self.peek() {
            Some(pair) => pair,
            None => return Ok(None),
        };

        if ch == '(' {
            self.bump();
            return Ok(Some(Token {
                kind: TokenKind::LParen,
                span: self.span_from(start_pos),
            }));
        }
        if ch == ')' {
            self.bump();
            return Ok(Some(Token {
                kind: TokenKind::RParen,
                span: self.span_from(start_pos),
            }));
        }
        if ch == '"' {
            return self.lex_string(start_pos);
        }

        // Fall back to generic atom.
        self.lex_atom(start_pos)
    }

    fn lex_string(&mut self, start_pos: Position) -> Result<Option<Token>, ParseError> {
        // consume opening quote
        self.bump();
        let mut buf = String::new();

        while let Some((_idx, ch)) = self.peek() {
            if ch == '"' {
                self.bump();
                let span = self.span_from(start_pos);
                return Ok(Some(Token {
                    kind: TokenKind::String(buf),
                    span,
                }));
            }
            if ch == '\\' {
                self.bump();
                match self.peek() {
                    Some((_eidx, esc)) => {
                        self.bump();
                        let translated = match esc {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            '\\' => '\\',
                            '"' => '"',
                            other => {
                                let span = self.span_from(start_pos);
                                return Err(ParseError::with_details(
                                    ParseErrorCode::InvalidAtom,
                                    format!("Invalid string escape '\\{}'", other),
                                    span,
                                    ErrorDetails {
                                        expected: vec!["valid escape sequence".to_string()],
                                        found: Some(other.to_string()),
                                    },
                                ));
                            }
                        };
                        buf.push(translated);
                    }
                    None => {
                        let span = self.span_from(start_pos);
                        return Err(ParseError::new(
                            ParseErrorCode::UnterminatedString,
                            "Unterminated string literal",
                            span,
                        ));
                    }
                }
            } else {
                self.bump();
                buf.push(ch);
            }
        }

        let span = self.span_from(start_pos);
        Err(ParseError::new(
            ParseErrorCode::UnterminatedString,
            "Unterminated string literal",
            span,
        ))
    }

    fn lex_atom(&mut self, start_pos: Position) -> Result<Option<Token>, ParseError> {
        let mut buf = String::new();
        // consume characters until we hit whitespace, paren, or comment prefix
        loop {
            let ch_opt = self.peek().map(|(_idx, ch)| ch);
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

        if buf.is_empty() {
            // This should not happen, but guard against infinite loops.
            let span = self.span_from(start_pos);
            return Err(ParseError::with_details(
                ParseErrorCode::InvalidAtom,
                "Failed to lex atom",
                span,
                ErrorDetails {
                    expected: vec!["atom".to_string()],
                    found: None,
                },
            ));
        }

        let span = self.span_from(start_pos);

        // classify atom
        if self.dialect.enable_booleans && (buf == "true" || buf == "false") {
            return Ok(Some(Token {
                kind: TokenKind::Boolean(buf == "true"),
                span,
            }));
        }

        if self.dialect.enable_keywords && buf.starts_with(':') && buf.len() > 1 {
            return Ok(Some(Token {
                kind: TokenKind::Keyword(buf[1..].to_string()),
                span,
            }));
        }

        if let Some(int_token) = self.try_lex_integer(&buf, span) {
            return Ok(Some(int_token));
        }

        Ok(Some(Token {
            kind: TokenKind::Symbol(buf),
            span,
        }))
    }

    fn try_lex_integer(&self, text: &str, span: Span) -> Option<Token> {
        let mut chars = text.chars();
        if let Some(first) = chars.next() {
            if first == '-' {
                if chars.clone().all(|c| c.is_ascii_digit()) {
                    if let Ok(value) = text.parse::<i64>() {
                        return Some(Token {
                            kind: TokenKind::Integer(value),
                            span,
                        });
                    }
                }
            } else if first.is_ascii_digit() && chars.all(|c| c.is_ascii_digit()) {
                if let Ok(value) = text.parse::<i64>() {
                    return Some(Token {
                        kind: TokenKind::Integer(value),
                        span,
                    });
                }
            }
        }
        None
    }
}
