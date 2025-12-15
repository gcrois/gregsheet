use crate::cell::Cell;

/// Compact GPU representation of a cell (8 bytes total: 2 × u32)
/// This struct is packed into the shader storage buffer as two consecutive u32 values
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuCell {
    /// Cell value casted from i64 to i32
    pub value: i32,
    /// Bitmask flags: Bit 0 = Selected, Bit 1 = Is Formula, Bit 2 = Error
    pub flags: u32,
}

impl GpuCell {
    pub const FLAG_SELECTED: u32 = 1 << 0; // Bit 0
    pub const FLAG_FORMULA: u32 = 1 << 1;  // Bit 1
    pub const FLAG_ERROR: u32 = 1 << 2;    // Bit 2
    pub const FLAG_RICH: u32 = 1 << 3;     // Bit 3

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
        if cell.svg_content.is_some() {
            flags |= Self::FLAG_RICH;
        }

        Self {
            value: cell.value.clamp(i32::MIN as i64, i32::MAX as i64) as i32,
            flags,
        }
    }

    /// Convert GpuCell to two u32 values for the shader buffer
    /// Returns (value_as_u32, flags)
    pub fn to_u32_pair(self) -> (u32, u32) {
        (self.value as u32, self.flags)
    }
}
