use super::*;
use crate::formula::{build_context, evaluate_formula};

#[test]
fn test_demo_data_setup() {
    let mut grid = GridState::new(10, 10);
    setup_demo_data(&mut grid);

    // Test counter setup
    let counter_cell = grid.get_cell(0, 0).unwrap();
    assert_eq!(counter_cell.raw, "= A0 + 1");
    assert!(counter_cell.is_formula);
    assert!(!counter_cell.error);

    // Test blinker setup
    let blinker1 = grid.get_cell(1, 0).unwrap();
    assert_eq!(blinker1.raw, "= A0 % 2");
    assert!(blinker1.is_formula);

    let blinker2 = grid.get_cell(1, 1).unwrap();
    assert_eq!(blinker2.raw, "= (A0 + 1) % 2");
    assert!(blinker2.is_formula);

    // Test accumulator setup
    let acc1 = grid.get_cell(2, 0).unwrap();
    assert_eq!(acc1.raw, "10");
    assert!(!acc1.is_formula);

    let acc2 = grid.get_cell(2, 1).unwrap();
    assert_eq!(acc2.raw, "20");
    assert!(!acc2.is_formula);

    let acc_sum = grid.get_cell(2, 2).unwrap();
    assert_eq!(acc_sum.raw, "= C0 + C1");
    assert!(acc_sum.is_formula);

    // Test Fibonacci setup
    let fib1 = grid.get_cell(3, 0).unwrap();
    assert_eq!(fib1.raw, "1");

    let fib2 = grid.get_cell(3, 1).unwrap();
    assert_eq!(fib2.raw, "1");

    let fib3 = grid.get_cell(3, 2).unwrap();
    assert_eq!(fib3.raw, "= D0 + D1");
    assert!(fib3.is_formula);
}

#[test]
fn test_counter_evaluation() {
    let mut grid = GridState::new(10, 10);
    setup_demo_data(&mut grid);

    // Initial state - counter should be 0
    let context = build_context(&grid);
    let counter_cell = grid.get_cell(0, 0).unwrap();
    let expr = counter_cell.raw.trim_start().trim_start_matches('=').trim();
    let result = evaluate_formula(expr, &context).unwrap();

    // A0 is initially 0, so A0 + 1 = 1
    assert_eq!(result, 1);
}

#[test]
fn test_accumulator_evaluation() {
    let mut grid = GridState::new(10, 10);
    setup_demo_data(&mut grid);

    // Set values for accumulator literals
    if let Some(cell) = grid.get_cell_mut(2, 0) {
        cell.value = 10;
    }
    if let Some(cell) = grid.get_cell_mut(2, 1) {
        cell.value = 20;
    }

    let context = build_context(&grid);
    let acc_sum = grid.get_cell(2, 2).unwrap();
    let expr = acc_sum.raw.trim_start().trim_start_matches('=').trim();
    let result = evaluate_formula(expr, &context).unwrap();

    assert_eq!(result, 30);
}

#[test]
fn test_fibonacci_evaluation() {
    let mut grid = GridState::new(10, 10);
    setup_demo_data(&mut grid);

    // Set initial Fibonacci values
    if let Some(cell) = grid.get_cell_mut(3, 0) {
        cell.value = 1;
    }
    if let Some(cell) = grid.get_cell_mut(3, 1) {
        cell.value = 1;
    }

    let context = build_context(&grid);

    // Test D2 = D0 + D1 = 1 + 1 = 2
    let fib3 = grid.get_cell(3, 2).unwrap();
    let expr = fib3.raw.trim_start().trim_start_matches('=').trim();
    let result = evaluate_formula(expr, &context).unwrap();
    assert_eq!(result, 2);
}

#[test]
fn test_blinker_evaluation() {
    let mut grid = GridState::new(10, 10);
    setup_demo_data(&mut grid);

    // Test with even counter value
    {
        // Set counter to even value
        if let Some(cell) = grid.get_cell_mut(0, 0) {
            cell.value = 4;
        }

        let context = build_context(&grid);

        // B0 = A0 % 2 = 4 % 2 = 0
        let expr1 = grid.get_cell(1, 0).unwrap().raw.clone();
        let expr1 = expr1.trim_start().trim_start_matches('=').trim();
        let result1 = evaluate_formula(expr1, &context).unwrap();
        assert_eq!(result1, 0);

        // B1 = (A0 + 1) % 2 = 5 % 2 = 1
        let expr2 = grid.get_cell(1, 1).unwrap().raw.clone();
        let expr2 = expr2.trim_start().trim_start_matches('=').trim();
        let result2 = evaluate_formula(expr2, &context).unwrap();
        assert_eq!(result2, 1);
    }

    // Test with odd counter value
    {
        // Set counter to odd value
        if let Some(cell) = grid.get_cell_mut(0, 0) {
            cell.value = 5;
        }
        let context = build_context(&grid);

        // B0 = A0 % 2 = 5 % 2 = 1
        let expr1 = grid.get_cell(1, 0).unwrap().raw.clone();
        let expr1 = expr1.trim_start().trim_start_matches('=').trim();
        let result1_odd = evaluate_formula(expr1, &context).unwrap();
        assert_eq!(result1_odd, 1);

        // B1 = (A0 + 1) % 2 = 6 % 2 = 0
        let expr2 = grid.get_cell(1, 1).unwrap().raw.clone();
        let expr2 = expr2.trim_start().trim_start_matches('=').trim();
        let result2_odd = evaluate_formula(expr2, &context).unwrap();
        assert_eq!(result2_odd, 0);
    }
}

#[test]
fn test_all_digits_display() {
    let mut grid = GridState::new(10, 10);
    setup_demo_data(&mut grid);

    // Test that digits 0-9 are set up correctly
    for i in 0..10 {
        let col = i % 5;
        let row = 5 + (i / 5);
        let cell = grid.get_cell(col, row).unwrap();
        assert_eq!(cell.raw, i.to_string());
        assert!(!cell.is_formula);
    }
}
