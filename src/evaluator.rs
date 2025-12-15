use bevy::prelude::*;

use crate::formula::{build_context, evaluate_formula};
use crate::grid_state::GridState;

/// Controls tick-based evaluation
#[derive(Resource)]
pub struct TickControl {
    /// When true, formulas auto-evaluate every 0.1s
    pub auto_tick_enabled: bool,
    /// When true, trigger one immediate evaluation and reset to false
    pub manual_tick_requested: bool,
}

impl Default for TickControl {
    fn default() -> Self {
        Self {
            auto_tick_enabled: false, // Off by default
            manual_tick_requested: false,
        }
    }
}

/// Timer for automatic tick evaluation
#[derive(Resource)]
pub struct EvaluationTimer {
    pub timer: Timer,
}

impl Default for EvaluationTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.1, TimerMode::Repeating),
        }
    }
}

/// Tick-based formula evaluation system
/// Runs every frame, but only evaluates when:
/// - Manual tick is requested, OR
/// - Auto-tick is enabled AND timer fires
pub fn tick_evaluation_system(
    time: Res<Time>,
    mut timer: ResMut<EvaluationTimer>,
    mut tick_control: ResMut<TickControl>,
    mut grid_state: ResMut<GridState>,
) {
    // Check if we should evaluate this frame
    let should_evaluate = if tick_control.manual_tick_requested {
        tick_control.manual_tick_requested = false; // Reset flag
        true
    } else if tick_control.auto_tick_enabled {
        timer.timer.tick(time.delta());
        timer.timer.just_finished()
    } else {
        false
    };

    if !should_evaluate {
        return;
    }

    // Phase 1: Build context from current grid values
    let context = build_context(&grid_state);

    // Phase 2: Evaluate all cells
    // Collect cells to avoid borrow checker issues
    // We store (col, row) as key
    let cells_to_evaluate: Vec<((i32, i32), String, bool)> = grid_state
        .cells
        .iter()
        .map(|(key, cell)| (*key, cell.raw.clone(), cell.is_formula))
        .collect();

    for (key, raw, is_formula) in cells_to_evaluate {
        // We can use get_mut because we hold the key and grid_state is ResMut
        // But we need to use 'if let Some' just in case, though keys came from it.
        if let Some(cell) = grid_state.cells.get_mut(&key) {
            if is_formula {
                // Strip leading '=' and whitespace
                let expr = raw.trim_start().trim_start_matches('=').trim();

                match evaluate_formula(expr, &context) {
                    Ok(new_value) => {
                        cell.value = new_value;
                        cell.error = false;
                    }
                    Err(_) => {
                        cell.error = true;
                        cell.value = evalexpr::Value::Int(0);
                    }
                }
            } else {
                // Parse literal value
                // Try to parse as number first (Int or Float), else String
                if let Ok(i) = raw.trim().parse::<i64>() {
                    cell.value = evalexpr::Value::Int(i);
                } else if let Ok(f) = raw.trim().parse::<f64>() {
                    cell.value = evalexpr::Value::Float(f);
                } else {
                    cell.value = evalexpr::Value::String(raw.clone());
                }
                cell.error = false;
            }
        }
    }

    // GridState is automatically marked as changed because we used ResMut
}
