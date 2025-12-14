use bevy::prelude::*;
use std::collections::HashSet;

use crate::cell::Cell;
use crate::gpu_cell::GpuCell;

/// CPU-side grid state - source of truth for all cell data
#[derive(Resource)]
pub struct GridState {
    /// All cells in row-major order
    pub cells: Vec<Cell>,
    /// Set of selected cell coordinates (col, row)
    pub selected: HashSet<(i32, i32)>,
    /// Grid dimensions
    pub cols: i32,
    pub rows: i32,
}

impl GridState {
    /// Create a new grid with the given dimensions
    pub fn new(cols: i32, rows: i32) -> Self {
        Self {
            cells: vec![Cell::default(); (cols * rows) as usize],
            selected: HashSet::new(),
            cols,
            rows,
        }
    }

    /// Get the index into the cells vector for a given (col, row)
    pub fn get_index(&self, col: i32, row: i32) -> Option<usize> {
        if col >= 0 && col < self.cols && row >= 0 && row < self.rows {
            Some((row * self.cols + col) as usize)
        } else {
            None
        }
    }

    /// Get an immutable reference to a cell
    pub fn get_cell(&self, col: i32, row: i32) -> Option<&Cell> {
        self.get_index(col, row).map(|idx| &self.cells[idx])
    }

    /// Get a mutable reference to a cell
    pub fn get_cell_mut(&mut self, col: i32, row: i32) -> Option<&mut Cell> {
        self.get_index(col, row).map(|idx| &mut self.cells[idx])
    }

    /// Convert the entire grid to GPU format (2 u32s per cell)
    pub fn to_gpu_cells(&self) -> Vec<u32> {
        let mut buffer = Vec::with_capacity(self.cells.len() * 2);

        for (idx, cell) in self.cells.iter().enumerate() {
            let row = (idx as i32) / self.cols;
            let col = (idx as i32) % self.cols;
            let is_selected = self.selected.contains(&(col, row));

            let gpu_cell = GpuCell::from_cell(cell, is_selected);
            let (value_u32, flags) = gpu_cell.to_u32_pair();

            buffer.push(value_u32);
            buffer.push(flags);
        }

        buffer
    }
}
