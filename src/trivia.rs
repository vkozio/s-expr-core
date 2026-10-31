use crate::span::Span;

/// The kind of a comment.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentKind {
    /// A line comment `; …` to end of line.
    Line,
    /// A nesting block comment `#| … |#`.
    Block,
}

/// A source comment captured as trivia.
///
/// `text` is the comment body without the delimiters: for [`CommentKind::Line`]
/// the leading prefix character, with a trailing `\r` trimmed (so a CRLF comment
/// reads back identically); for [`CommentKind::Block`] the raw content between
/// `#|` and `|#` (including any nested `#| … |#`). The printer re-emits the
/// delimiters from `kind`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub kind: CommentKind,
    pub text: String,
    pub span: Span,
}
