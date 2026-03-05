#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// Zero-based byte offset from the beginning of the input.
    pub byte: u32,
    /// One-based line number.
    pub line: u32,
    /// One-based column number within the line, counting Unicode scalar values.
    pub column: u32,
}

impl Position {
    pub const fn start() -> Self {
        Position {
            byte: 0,
            line: 1,
            column: 1,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn new(start: Position, end: Position) -> Self {
        Span { start, end }
    }
}
