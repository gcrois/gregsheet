use evalexpr::Value;

/// Represents a single spreadsheet cell on the CPU side
#[derive(Clone, Debug)]
pub struct Cell {
    /// The raw source text: "= A0 + B0" or "100"
    pub raw: String,
    /// The computed result
    pub value: Value,
    /// True if the content starts with '='
    pub is_formula: bool,
    /// True if evalexpr returned an error
    pub error: bool,
    /// Hash of the SVG content for caching
    pub content_hash: Option<u64>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            raw: String::new(),
            value: Value::Int(0),
            is_formula: false,
            error: false,
            content_hash: None,
        }
    }
}

impl Cell {
    /// Create a new cell from raw text
    pub fn new(raw: String) -> Self {
        let is_formula = raw.trim_start().starts_with('=');
        Self {
            raw,
            value: Value::Int(0),
            is_formula,
            error: false,
            content_hash: None,
        }
    }

    /// Update the raw text and reset state
    pub fn set_raw(&mut self, raw: String) {
        self.raw = raw;
        self.is_formula = self.raw.trim_start().starts_with('=');
        self.error = false;
    }
}
