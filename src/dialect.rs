use crate::limits::Limits;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct DialectConfig {
    /// Whether line comments are enabled.
    pub enable_comments: bool,
    /// Single-character prefix that starts a comment (default: ';').
    pub comment_prefix: char,
    /// Whether booleans like `true` / `false` are recognized as a distinct atom kind.
    pub enable_booleans: bool,
    /// Whether floating-point literals like `1.5` are recognized as a distinct atom kind.
    pub enable_floats: bool,
    /// Whether keywords like `:foo` are recognized as a distinct atom kind.
    pub enable_keywords: bool,
    /// Resource limits for parsing; see Limits for details.
    pub limits: Limits,
}

impl DialectConfig {
    /// Default dialect matching the contract examples.
    pub fn default_dialect() -> Self {
        Self {
            enable_comments: true,
            comment_prefix: ';',
            enable_booleans: true,
            enable_floats: true,
            enable_keywords: true,
            limits: Limits::default(),
        }
    }
}

impl Default for DialectConfig {
    fn default() -> Self {
        Self::default_dialect()
    }
}
