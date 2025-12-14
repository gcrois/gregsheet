#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct GridMaterial {
    viewport_bottom_left: vec2<f32>,
    viewport_size: vec2<f32>,
    cell_size: vec2<f32>,
    line_width: f32,
    color_bg: vec4<f32>,
    color_line: vec4<f32>,
    grid_dimensions: vec2<f32>,
}

@group(2) @binding(0)
var<uniform> material: GridMaterial;

@group(2) @binding(1)
var<storage, read> cell_data: array<u32>;

// --- BITMASK FONT LOGIC ---
// 4x5 Pixel Font.
var<private> FONT: array<u32, 10> = array<u32, 10>(
    0x69f96u, // 0
    0x26227u, // 1
    0x6924fu, // 2
    0x69296u, // 3
    0x99f11u, // 4
    0xf8e1eu, // 5
    0x68e96u, // 6
    0xf1222u, // 7
    0x69696u, // 8
    0x69f16u  // 9
);

fn get_digit_pixel(digit: i32, pos: vec2<i32>) -> f32 {
    if (digit < 0 || digit > 9) { return 0.0; }
    if (pos.x < 0 || pos.x >= 4 || pos.y < 0 || pos.y >= 5) { return 0.0; }
    
    let bit_index = u32(pos.y * 4 + (3 - pos.x));
    
    if ((FONT[digit] & (1u << bit_index)) != 0u) { return 1.0; }
    return 0.0;
}

fn draw_number(n_in: i32, uv_in: vec2<f32>) -> f32 {
    var n = n_in;
    var cursor = uv_in;
    var pixel_on = 0.0;

    // 1. Handle Negative Sign
    if (n < 0) {
        n = -n;
        // Draw minus sign (centered vertically in the 5px height)
        // Check if cursor is in the "minus" box (approx 3x5 pixels wide area)
        if (cursor.x >= 0.0 && cursor.x < 1.0 && cursor.y >= 0.4 && cursor.y < 0.6) {
             pixel_on = 1.0;
        }
        cursor.x -= 1.2; // Shift cursor for next character
    }

    // 2. Extract Digits
    var d1 = n % 10;           // Ones
    var d2 = (n / 10) % 10;    // Tens
    var d3 = (n / 100) % 10;   // Hundreds

    // Determine how many digits we have to draw
    var count = 1;
    if (n >= 10) { count = 2; }
    if (n >= 100) { count = 3; }

    // 3. Draw Digits (Left to Right)
    // Hundreds
    if (count >= 3) {
        let local_pos = vec2<i32>(floor(cursor * vec2<f32>(4.0, 5.0)));
        pixel_on = max(pixel_on, get_digit_pixel(d3, local_pos));
        cursor.x -= 1.2;
    }
    // Tens
    if (count >= 2) {
        let local_pos = vec2<i32>(floor(cursor * vec2<f32>(4.0, 5.0)));
        pixel_on = max(pixel_on, get_digit_pixel(d2, local_pos));
        cursor.x -= 1.2;
    }
    // Ones
    if (count >= 1) {
        let local_pos = vec2<i32>(floor(cursor * vec2<f32>(4.0, 5.0)));
        pixel_on = max(pixel_on, get_digit_pixel(d1, local_pos));
    }

    return pixel_on;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // Flip V coordinate: UV (0,0) is top-left, but we want bottom-left
    let uv_flipped = vec2<f32>(mesh.uv.x, 1.0 - mesh.uv.y);
    let world_pos = material.viewport_bottom_left + uv_flipped * material.viewport_size;

    // Grid Logic
    // IMPORTANT: This calculation MUST match world_pos_to_cell() in src/main.rs
    let col = i32(floor(world_pos.x / material.cell_size.x));
    let row = i32(floor(-world_pos.y / material.cell_size.y));
    let cell_uv = fract(vec2<f32>(world_pos.x, -world_pos.y) / material.cell_size);
    let dist_to_line = min(cell_uv, 1.0 - cell_uv);
    let line_width_norm = material.line_width / material.cell_size;
    
    if (any(dist_to_line < line_width_norm)) {
        return material.color_line;
    }

    // Background Color - Read GpuCell (2 u32s per cell)
    var final_color = material.color_bg;
    if (col >= 0 && col < i32(material.grid_dimensions.x) &&
        row >= 0 && row < i32(material.grid_dimensions.y)) {
        let index = u32(row) * u32(material.grid_dimensions.x) + u32(col);
        let cell_value_u32 = cell_data[index * 2u];
        let cell_flags = cell_data[index * 2u + 1u];

        let is_selected = (cell_flags & 1u) != 0u;  // Bit 0
        let is_error = (cell_flags & 4u) != 0u;     // Bit 2

        // Error background (red)
        if (is_error) {
            final_color = vec4<f32>(1.0, 0.3, 0.3, 1.0);
        }
        // Selection highlight (blue tint)
        else if (is_selected) {
            final_color = mix(material.color_bg, vec4<f32>(0.2, 0.4, 0.8, 1.0), 0.5);
        }
    }
    
    let pixel_offset = (cell_uv - 0.5) * material.cell_size;
    let font_size_px = 8.0;

    // Flip Y for text rendering since we flipped the UV coordinate
    let char_uv_base = vec2<f32>(pixel_offset.x, -pixel_offset.y) / font_size_px;

    // Render cell value in the center (adjusted Y to show full 5px height)
    let value_uv = char_uv_base - vec2<f32>(-1.0, 0.3);

    var text_alpha = 0.0;
    var text_color = vec4<f32>(0.1, 0.1, 0.1, 1.0); // Default black

    // Get cell value and flags for rendering
    if (col >= 0 && col < i32(material.grid_dimensions.x) &&
        row >= 0 && row < i32(material.grid_dimensions.y)) {
        let index = u32(row) * u32(material.grid_dimensions.x) + u32(col);
        let cell_value_u32 = cell_data[index * 2u];
        let cell_flags = cell_data[index * 2u + 1u];
        let cell_value = i32(cell_value_u32);

        let is_formula = (cell_flags & 2u) != 0u;  // Bit 1

        // Blue for formulas, black for literals
        if (is_formula) {
            text_color = vec4<f32>(0.0, 0.2, 0.8, 1.0);
        }

        text_alpha = draw_number(cell_value, value_uv);
    }

    if (text_alpha > 0.5) {
        return text_color;
    }

    return final_color;
}