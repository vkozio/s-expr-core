use crate::span::Span;

/// A source comment captured as trivia.
///
/// `text` is the comment body without the leading prefix character, with a
/// trailing `\r` trimmed (so a CRLF comment reads back identically).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub text: String,
    pub span: Span,
}
