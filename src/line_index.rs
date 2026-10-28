/// Byte offsets <-> LSP positions (line, UTF-16 column). Owns a copy of the text.
#[derive(Debug, Clone)]
pub struct LineIndex {
    text: String,
    line_starts: Vec<u32>,
}

impl LineIndex {
    /// Build a line index over `text`. Line breaks are `\n` and `\r\n`.
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0u32];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push((i + 1) as u32);
            }
        }
        LineIndex {
            text: text.to_owned(),
            line_starts,
        }
    }

    /// Convert a byte offset to a `(line0, utf16_col)` pair.
    pub fn position(&self, byte: u32) -> (u32, u32) {
        let byte = byte.min(self.text.len() as u32);
        let line = match self.line_starts.binary_search(&byte) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let line_start = self.line_starts[line] as usize;
        let mut col = 0u32;
        let mut pos = line_start;
        for ch in self.text[line_start..].chars() {
            if pos >= byte as usize {
                break;
            }
            pos += ch.len_utf8();
            if pos <= byte as usize {
                col += ch.len_utf16() as u32;
            }
        }
        (line as u32, col)
    }

    /// Convert a `(line0, utf16_col)` pair back to a byte offset.
    pub fn offset(&self, line0: u32, utf16_col: u32) -> u32 {
        let line = line0 as usize;
        let line_start = match self.line_starts.get(line) {
            Some(&s) => s as usize,
            None => return self.text.len() as u32,
        };
        let line_end = self
            .line_starts
            .get(line + 1)
            .map(|&e| e as usize)
            .unwrap_or(self.text.len());
        let mut col = 0u32;
        let mut pos = line_start;
        for ch in self.text[line_start..line_end].chars() {
            if col >= utf16_col {
                break;
            }
            pos += ch.len_utf8();
            col += ch.len_utf16() as u32;
        }
        pos.min(line_end) as u32
    }
}
