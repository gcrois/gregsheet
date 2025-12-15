#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct GridMaterial {
    viewport_bottom_left: vec2<f32>,
    viewport_size: vec2<f32>,
    cell_size: vec2<f32>,
    line_width: f32,
    color_bg: vec4<f32>,
    color_line: vec4<f32>,
    grid_dimensions: vec2<f32>,
    show_grid: f32,
}

@group(2) @binding(0)
var<uniform> material: GridMaterial;

@group(2) @binding(1)
var<storage, read> cell_data: array<u32>; // Viewport-relative buffer

@group(2) @binding(2)
var rich_cell_textures: texture_2d_array<f32>;

@group(2) @binding(3)
var rich_cell_sampler: sampler;

@group(2) @binding(4)
var<storage, read> rich_cell_indices: array<i32>; // Viewport-relative buffer

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // Flip V coordinate: UV (0,0) is top-left, but we want bottom-left for world pos
    let uv_flipped = vec2<f32>(mesh.uv.x, 1.0 - mesh.uv.y);
    let world_pos = material.viewport_bottom_left + uv_flipped * material.viewport_size;

    // Grid Logic
    let col = i32(floor(world_pos.x / material.cell_size.x));
    let row = i32(floor(-world_pos.y / material.cell_size.y));
    let grid_pos = vec2<f32>(world_pos.x, -world_pos.y) / material.cell_size;
    let cell_uv = fract(grid_pos);
    let dist_to_line = min(cell_uv, 1.0 - cell_uv);
    
    let closest_line_idx = vec2<i32>(round(grid_pos));
    
    // Determine line thickness multiplier
    var width_mult = vec2<f32>(1.0, 1.0);
    
    // Vertical lines (X-index)
    if (closest_line_idx.x == 0) {
        width_mult.x = 2.0; // Axis slightly thicker
    } else if (closest_line_idx.x % 5 == 0) {
        width_mult.x = 2.0; // Major line
    }
    
    // Horizontal lines (Y-index)
    if (closest_line_idx.y == 0) {
        width_mult.y = 2.0; // Axis slightly thicker
    } else if (closest_line_idx.y % 5 == 0) {
        width_mult.y = 2.0; // Major line
    }

    let line_width_norm = (material.line_width * width_mult) / material.cell_size;
    
    let on_vert = dist_to_line.x < line_width_norm.x;
    let on_horiz = dist_to_line.y < line_width_norm.y;

    if (material.show_grid > 0.5 && (on_vert || on_horiz)) {
        // Axis Colors
        // Horizontal Axis (y=0) -> Red
        if (on_horiz && closest_line_idx.y == 0) {
            return vec4<f32>(0.8, 0.2, 0.2, 1.0);
        }
        // Vertical Axis (x=0) -> Green
        if (on_vert && closest_line_idx.x == 0) {
            return vec4<f32>(0.2, 0.8, 0.2, 1.0);
        }
        
        return material.color_line;
    }

    // Calculate viewport-relative coordinates
    let min_col = i32(floor(material.viewport_bottom_left.x / material.cell_size.x));

    let viewport_top_right = material.viewport_bottom_left + material.viewport_size;
    let min_row = i32(floor(-viewport_top_right.y / material.cell_size.y));

    let rel_col = col - min_col;
    let rel_row = row - min_row;
    let width = i32(material.grid_dimensions.x);
    let height = i32(material.grid_dimensions.y);

    var final_color = material.color_bg;

    // Check bounds of relative coordinates
    if (rel_col >= 0 && rel_col < width && rel_row >= 0 && rel_row < height) {
        let index = u32(rel_row) * u32(width) + u32(rel_col);
        
        if (index < arrayLength(&cell_data)) {
            let cell_flags = cell_data[index];
            let is_selected = (cell_flags & 1u) != 0u;  // Bit 0
            let is_error = (cell_flags & 4u) != 0u;     // Bit 2

            if (is_error) {
                final_color = vec4<f32>(1.0, 0.3, 0.3, 1.0);
            } else if (is_selected) {
                final_color = mix(material.color_bg, vec4<f32>(0.2, 0.4, 0.8, 1.0), 0.5);
            }
        }

        // Rich Content (SVG) Layer
        if (index < arrayLength(&rich_cell_indices)) {
            let texture_layer = rich_cell_indices[index];

            if (texture_layer >= 0) {
                // Sample from texture array
                let texture_color = textureSample(
                    rich_cell_textures,
                    rich_cell_sampler,
                    cell_uv,
                    texture_layer
                );

                // Alpha blend over background
                final_color = mix(final_color, vec4<f32>(texture_color.rgb, 1.0), texture_color.a);
            }
        }
    }
    
    return final_color;
}
