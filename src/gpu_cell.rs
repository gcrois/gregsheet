use crate::cell::Cell;

/// Compact GPU representation of a cell (8 bytes total: 2 × u32)
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuCell {
    /// Bitmask flags: Bit 0 = Selected, Bit 1 = Is Formula, Bit 2 = Error
    pub flags: u32,
}

impl GpuCell {
    pub const FLAG_SELECTED: u32 = 1 << 0; // Bit 0
    pub const FLAG_FORMULA: u32 = 1 << 1;  // Bit 1
    pub const FLAG_ERROR: u32 = 1 << 2;    // Bit 2

    /// Convert a CPU Cell to GPU representation
    pub fn from_cell(cell: &Cell, selected: bool) -> Self {
        let mut flags = 0u32;

        if selected {
            flags |= Self::FLAG_SELECTED;
        }
        if cell.is_formula {
            flags |= Self::FLAG_FORMULA;
        }
        if cell.error {
            flags |= Self::FLAG_ERROR;
        }

        Self {
            flags,
        }
    }

    /// Convert GpuCell to one u32 value for the shader buffer
    pub fn to_u32(self) -> u32 {
        self.flags
    }
}
