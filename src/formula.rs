use evalexpr::{HashMapContext, Value, ContextWithMutableVariables};

use crate::grid_state::GridState;

/// Convert (col, row) to Excel-style name: A0, B0, ... Z0, AA0, AB0, etc.
pub fn coord_to_name(col: i32, row: i32) -> String {
    let mut name = String::new();
    let mut c = col;

    // Convert column number to letters (A-Z, AA-AZ, BA-BZ, etc.)
    loop {
        name.push((b'A' + (c % 26) as u8) as char);
        c /= 26;
        if c == 0 {
            break;
        }
        c -= 1; // Adjust for 0-indexing
    }

    // Reverse to get correct order, then append row number
    name.chars().rev().collect::<String>() + &row.to_string()
}

/// Build evaluation context from current grid state
/// Maps all cell coordinates to their current values (e.g., A0 = 5, B0 = 10)
pub fn build_context(grid: &GridState) -> HashMapContext {
    let mut context = HashMapContext::new();

    for row in 0..grid.rows {
        for col in 0..grid.cols {
            let var_name = coord_to_name(col, row);
            let value = grid
                .get_cell(col, row)
                .map(|cell| cell.value)
                .unwrap_or(0);

            // Set the variable in the context
            let _ = context.set_value(var_name, Value::Int(value));
        }
    }

    context
}

/// Evaluate a formula expression (without the leading '=')
/// Returns the i64 result or an error if evaluation fails
pub fn evaluate_formula(
    expr: &str,
    context: &HashMapContext,
) -> Result<i64, evalexpr::EvalexprError> {
    let result = evalexpr::eval_with_context(expr, context)?;

    // Convert result to i64
    match result {
        Value::Int(i) => Ok(i),
        Value::Float(f) => Ok(f as i64),
        Value::Boolean(b) => Ok(if b { 1 } else { 0 }),
        _ => Ok(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coord_to_name() {
        assert_eq!(coord_to_name(0, 0), "A0");
        assert_eq!(coord_to_name(1, 0), "B0");
        assert_eq!(coord_to_name(25, 0), "Z0");
        assert_eq!(coord_to_name(26, 0), "AA0");
        assert_eq!(coord_to_name(27, 0), "AB0");
        assert_eq!(coord_to_name(0, 15), "A15");
        assert_eq!(coord_to_name(26, 10), "AA10");
    }
}
