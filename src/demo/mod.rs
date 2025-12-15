use crate::grid_state::GridState;

#[cfg(test)]
mod tests;

/// Initialize the grid with demo data showcasing the formula evaluation system
pub fn setup_demo_data(grid: &mut GridState) {
    // Counter: A0 increments itself each tick
    grid.get_cell_mut_or_create(0, 0).set_raw("= A0 + 1".to_string());

    // Blinker: B0 and B1 oscillate based on the counter
    grid.get_cell_mut_or_create(1, 0).set_raw("= A0 % 2".to_string());
    grid.get_cell_mut_or_create(1, 1).set_raw("= (A0 + 1) % 2".to_string());

    // Accumulator: Literal values and their sum
    grid.get_cell_mut_or_create(2, 0).set_raw("10".to_string());
    grid.get_cell_mut_or_create(2, 1).set_raw("20".to_string());
    grid.get_cell_mut_or_create(2, 2).set_raw("= C0 + C1".to_string());

    // Fibonacci-like sequence
    for row in 0..5 {
        if row < 2 {
            grid.get_cell_mut_or_create(3, row).set_raw("1".to_string());
        } else {
            let formula = format!("= D{} + D{}", row - 2, row - 1);
            grid.get_cell_mut_or_create(3, row).set_raw(formula);
        }
    }

    // show all numbers numbers from 0 to 9 row 5 / 6
    for i in 0..10 {
        grid.get_cell_mut_or_create(i % 5, 5 + (i / 5)).set_raw(i.to_string());
    }
}
