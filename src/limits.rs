#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct Limits {
    /// Maximum nesting depth of lists. None means unbounded.
    pub max_depth: Option<usize>,
    /// Maximum number of elements in any single list. None means unbounded.
    pub max_list_length: Option<usize>,
    /// Maximum total number of nodes (atoms + lists) in the parsed tree. None means unbounded.
    pub max_nodes: Option<usize>,
}

impl Limits {
    /// Conservative defaults suitable for interactive use and most tests.
    pub fn default_for_general_use() -> Self {
        Self {
            max_depth: Some(256),
            max_list_length: Some(4096),
            max_nodes: Some(1_000_000),
        }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self::default_for_general_use()
    }
}
