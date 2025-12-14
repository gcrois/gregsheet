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
    let cells_to_evaluate: Vec<(usize, String, bool)> = grid_state
        .cells
        .iter()
        .enumerate()
        .map(|(idx, cell)| (idx, cell.raw.clone(), cell.is_formula))
        .collect();

    for (idx, raw, is_formula) in cells_to_evaluate {
        let cell = &mut grid_state.cells[idx];

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
                    cell.value = 0;
                }
            }
        } else {
            // Parse literal value
            let new_value = raw.trim().parse::<i64>().unwrap_or(0);
            cell.value = new_value;
            cell.error = false;
        }
    }

    // GridState is automatically marked as changed because we used ResMut
}
