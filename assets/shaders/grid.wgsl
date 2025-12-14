#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct GridMaterial {
    viewport_top_left: vec2<f32>,
    viewport_size: vec2<f32>,
    cell_size: vec2<f32>,
    line_width: f32,
    color_bg: vec4<f32>,
    color_line: vec4<f32>,
}

@group(2) @binding(0)
var<uniform> material: GridMaterial;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // Convert from normalized mesh coords to world coordinates
    let world_pos = material.viewport_top_left + mesh.uv * material.viewport_size;

    // Calculate position within cell (0.0 to 1.0)
    let cell_pos = fract(world_pos / material.cell_size);

    // Distance from grid lines (0.0 at line, 0.5 at center of cell)
    let dist_to_line = min(cell_pos, 1.0 - cell_pos);

    // Convert line width from pixels to cell fraction
    let line_width_normalized = material.line_width / material.cell_size;

    // Check if we're on a grid line
    let on_line_x = dist_to_line.x < line_width_normalized.x;
    let on_line_y = dist_to_line.y < line_width_normalized.y;

    // Return line color if on a line, otherwise background color
    if (on_line_x || on_line_y) {
        return material.color_line;
    } else {
        return material.color_bg;
    }
}
