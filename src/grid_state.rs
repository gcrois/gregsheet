use bevy::prelude::*;
use std::collections::{HashSet, HashMap};

use crate::cell::Cell;
use crate::gpu_cell::GpuCell;

/// CPU-side grid state - source of truth for all cell data
#[derive(Resource)]
pub struct GridState {
    /// Sparse cells storage
    pub cells: HashMap<(i32, i32), Cell>,
    /// Set of selected cell coordinates (col, row)
    pub selected: HashSet<(i32, i32)>,
}

impl GridState {
    /// Create a new empty grid
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            selected: HashSet::new(),
        }
    }

    /// Get an immutable reference to a cell
    pub fn get_cell(&self, col: i32, row: i32) -> Option<&Cell> {
        self.cells.get(&(col, row))
    }

    /// Get a mutable reference to a cell
    pub fn get_cell_mut(&mut self, col: i32, row: i32) -> Option<&mut Cell> {
        self.cells.get_mut(&(col, row))
    }

    /// Get a mutable reference to a cell, creating it if it doesn't exist
    pub fn get_cell_mut_or_create(&mut self, col: i32, row: i32) -> &mut Cell {
        self.cells.entry((col, row)).or_default()
    }
    
    /// Insert or update a cell
    pub fn set_cell(&mut self, col: i32, row: i32, cell: Cell) {
        self.cells.insert((col, row), cell);
    }

    /// Generate GPU buffer for a specific viewport region
    pub fn to_gpu_cells_viewport(&self, min_col: i32, min_row: i32, width: i32, height: i32) -> Vec<u32> {
        let count = (width * height) as usize;
        let mut buffer = Vec::with_capacity(count); // 1 u32 per cell

        for y in 0..height {
            for x in 0..width {
                let col = min_col + x;
                let row = min_row + y;
                
                let is_selected = self.selected.contains(&(col, row));
                
                if let Some(cell) = self.cells.get(&(col, row)) {
                    let gpu_cell = GpuCell::from_cell(cell, is_selected);
                    let flags = gpu_cell.to_u32();
                    buffer.push(flags);
                } else {
                    // Empty cell
                    let mut flags = 0u32;
                    if is_selected {
                        flags |= GpuCell::FLAG_SELECTED;
                    }
                    buffer.push(flags);
                }
            }
        }

        buffer
    }
}
