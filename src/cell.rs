/// Represents a single spreadsheet cell on the CPU side
#[derive(Clone, Debug)]
pub struct Cell {
    /// The raw source text: "= A0 + B0" or "100"
    pub raw: String,
    /// The computed integer result
    pub value: i64,
    /// True if the content starts with '='
    pub is_formula: bool,
    /// True if evalexpr returned an error
    pub error: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            raw: String::new(),
            value: 0,
            is_formula: false,
            error: false,
        }
    }
}

impl Cell {
    /// Create a new cell from raw text
    pub fn new(raw: String) -> Self {
        let is_formula = raw.trim_start().starts_with('=');
        Self {
            raw,
            value: 0,
            is_formula,
            error: false,
        }
    }

    /// Update the raw text and reset state
    pub fn set_raw(&mut self, raw: String) {
        self.raw = raw;
        self.is_formula = self.raw.trim_start().starts_with('=');
        self.error = false;
    }
}
