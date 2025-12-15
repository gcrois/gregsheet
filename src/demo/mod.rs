use crate::grid_state::GridState;

#[cfg(test)]
mod tests;

/// Initialize the grid with demo data showcasing the formula evaluation system
pub fn setup_demo_data(grid: &mut GridState) {
    // Counter: A0 increments itself each tick
    if let Some(cell) = grid.get_cell_mut(0, 0) {
        cell.set_raw("= A0 + 1".to_string());
    }

    // Blinker: B0 and B1 oscillate based on the counter
    if let Some(cell) = grid.get_cell_mut(1, 0) {
        cell.set_raw("= A0 % 2".to_string());
    }
    if let Some(cell) = grid.get_cell_mut(1, 1) {
        cell.set_raw("= (A0 + 1) % 2".to_string());
    }

    // Accumulator: Literal values and their sum
    if let Some(cell) = grid.get_cell_mut(2, 0) {
        cell.set_raw("10".to_string());
    }
    if let Some(cell) = grid.get_cell_mut(2, 1) {
        cell.set_raw("20".to_string());
    }
    if let Some(cell) = grid.get_cell_mut(2, 2) {
        cell.set_raw("= C0 + C1".to_string());
    }

    // Fibonacci-like sequence
    for row in 0..5 {
        if let Some(cell) = grid.get_cell_mut(3, row) {
            if row < 2 {
                cell.set_raw("1".to_string());
            } else {
                cell.set_raw(format!("= D{} + D{}", row - 2, row - 1));
            }
        }
    }

    // show all numbers numbers from 0 to 9 row 5 / 6
    for i in 0..10 {
        if let Some(cell) = grid.get_cell_mut(i % 5, 5 + (i / 5)) {
            cell.set_raw(i.to_string());
        }
    }

    // Rich Content (SVG) Demo
    if let Some(cell) = grid.get_cell_mut(0, 2) {
        // Use r##" (two hashes) so that fill="#..." doesn't close the string early
        cell.svg_content = Some(r##"
            <svg xmlns="http://www.w3.org/2000/svg" width="80" height="30">
                <rect width="80" height="30" fill="#e0f7fa"/>
                <text x="5" y="20" font-family="sans-serif" font-size="12" fill="#006064">Status: OK</text>
            </svg>
        "##.to_string());
    }

    if let Some(cell) = grid.get_cell_mut(1, 2) {
        cell.svg_content = Some(r##"
            <svg xmlns="http://www.w3.org/2000/svg" width="80" height="30">
                <circle cx="15" cy="15" r="8" fill="#4caf50"/>
                <text x="30" y="20" font-family="sans-serif" font-size="12" fill="#333">Active</text>
            </svg>
        "##.to_string());
    }
}
