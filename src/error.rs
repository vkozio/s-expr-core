use crate::span::Span;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitKind {
    Depth,
    ListLength,
    NodeCount,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorCode {
    UnexpectedEof,
    UnbalancedParen,
    UnterminatedString,
    UnterminatedBlockComment,
    InvalidAtom,
    LimitExceeded(LimitKind),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ErrorDetails {
    /// Human-readable token kinds that were expected at the error location.
    pub expected: Vec<String>,
    /// Optional textual representation of what was actually found.
    pub found: Option<String>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(thiserror::Error, Debug, Clone)]
#[error("{code:?}: {message}")]
pub struct ParseError {
    pub code: ParseErrorCode,
    pub message: String,
    pub span: Span,
    pub details: Option<ErrorDetails>,
}

impl ParseError {
    pub fn new(code: ParseErrorCode, message: impl Into<String>, span: Span) -> Self {
        ParseError {
            code,
            message: message.into(),
            span,
            details: None,
        }
    }

    pub fn with_details(
        code: ParseErrorCode,
        message: impl Into<String>,
        span: Span,
        details: ErrorDetails,
    ) -> Self {
        ParseError {
            code,
            message: message.into(),
            span,
            details: Some(details),
        }
    }
}
