use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    render::storage::ShaderStorageBuffer,
    shader::ShaderRef,
    sprite_render::{Material2d, Material2dPlugin},
};

const GRID_COLS: i32 = 128;
const GRID_ROWS: i32 = 128;

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        Material2dPlugin::<SpreadsheetGridMaterial>::default(),
    ))
    .insert_resource(GridState {
        cells: vec![0u32; (GRID_COLS * GRID_ROWS) as usize],
    })
    .insert_resource(DragState::default())
    .add_systems(Startup, (setup, setup_ui))
    .add_systems(Update, (
        update_grid_to_camera,
        grid_interaction,
        handle_camera_buttons,
        handle_keyboard_input,
        apply_camera_actions,
        sync_grid_buffer
    ));
    app.run();
}

// CPU-side source of truth for cell data
#[derive(Resource)]
struct GridState {
    cells: Vec<u32>, // 0 = empty, 1 = selected, 2 = heat
}

// Track drag state to toggle cells only once per drag
#[derive(Resource, Default)]
struct DragState {
    is_dragging: bool,
    toggled_cells: std::collections::HashSet<(i32, i32)>,
}

// --- Material Definition ---
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct SpreadsheetGridMaterial {
    #[uniform(0)]
    viewport_bottom_left: Vec2,
    #[uniform(0)]
    viewport_size: Vec2,
    #[uniform(0)]
    cell_size: Vec2,
    #[uniform(0)]
    line_width: f32,
    #[uniform(0)]
    color_bg: LinearRgba,
    #[uniform(0)]
    color_line: LinearRgba,
    #[uniform(0)]
    grid_dimensions: Vec2,
    #[storage(1, read_only)]
    cell_data: Handle<ShaderStorageBuffer>,
}

impl Material2d for SpreadsheetGridMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/grid.wgsl".into()
    }
}

#[derive(Component)]
struct GridBackdrop;

// Coordinate transformation utilities
// Single source of truth for world_pos -> (col, row)
fn world_pos_to_cell(world_pos: Vec2, cell_size: Vec2) -> (i32, i32) {
    let col = (world_pos.x / cell_size.x).floor() as i32;
    let row = (-world_pos.y / cell_size.y).floor() as i32;
    (col, row)
}

// Camera update types (interactions)
#[derive(Component, Clone, Copy, Debug)]
enum CameraAction {
    Zoom(f32),      // multiply scale by this factor
    Pan(Vec2),      // translate by this amount (in scaled units)
    Reset,
}

// Camera control components
#[derive(Component)]
enum CameraButton {
    ZoomIn,
    ZoomOut,
    PanLeft,
    PanRight,
    PanUp,
    PanDown,
    Reset,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SpreadsheetGridMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    commands.spawn((Camera2d, Transform::from_xyz(0.0, 0.0, 0.0)));

    let initial_data = vec![0u32; (GRID_COLS * GRID_ROWS) as usize];
    let buffer_handle = buffers.add(ShaderStorageBuffer::from(initial_data));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))),
        MeshMaterial2d(materials.add(SpreadsheetGridMaterial {
            viewport_bottom_left: Vec2::ZERO,
            viewport_size: Vec2::ONE,
            cell_size: Vec2::new(80.0, 30.0),
            line_width: 1.0,
            color_bg: LinearRgba::WHITE,
            color_line: LinearRgba::gray(0.8),
            grid_dimensions: Vec2::new(GRID_COLS as f32, GRID_ROWS as f32),
            cell_data: buffer_handle,
        })),
        Transform::from_xyz(0.0, 0.0, -100.0),
        GridBackdrop,
    ));
}

fn update_grid_to_camera(
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut grid_q: Query<(&mut Transform, &MeshMaterial2d<SpreadsheetGridMaterial>), With<GridBackdrop>>,
    mut materials: ResMut<Assets<SpreadsheetGridMaterial>>,
) {
    let Ok((camera, cam_transform)) = camera_q.single() else { return };
    let Ok((mut grid_transform, grid_handle)) = grid_q.single_mut() else { return };

    let Some(rect) = camera.logical_viewport_rect() else { return };
    let min_world = camera.viewport_to_world_2d(cam_transform, rect.min).ok();
    let max_world = camera.viewport_to_world_2d(cam_transform, rect.max).ok();

    if let (Some(min), Some(max)) = (min_world, max_world) {
        let size = (max - min).abs();
        let center = (min + max) / 2.0;

        grid_transform.translation.x = center.x;
        grid_transform.translation.y = center.y;
        grid_transform.scale = size.extend(1.0);

        if let Some(mat) = materials.get_mut(grid_handle) {
            let bottom_left = Vec2::new(min.x.min(max.x), min.y.min(max.y));

            mat.viewport_bottom_left = bottom_left;
            mat.viewport_size = size;
        }
    }
}

fn grid_interaction(
    window_q: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    grid_q: Query<&MeshMaterial2d<SpreadsheetGridMaterial>>,
    materials: Res<Assets<SpreadsheetGridMaterial>>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    mut grid_state: ResMut<GridState>,
    mut drag_state: ResMut<DragState>,
) {
    let Ok((camera, cam_transform)) = camera_q.single() else { return };
    let Ok(window) = window_q.single() else { return };
    let Ok(grid_handle) = grid_q.single() else { return };
    let Some(mat) = materials.get(grid_handle) else { return };

    // --- onMouseDown Handler ---
    if mouse_btn.just_pressed(MouseButton::Left) {
        drag_state.is_dragging = true;
        drag_state.toggled_cells.clear();
    }

    // --- onMouseUp Handler ---
    if mouse_btn.just_released(MouseButton::Left) {
        drag_state.is_dragging = false;
        drag_state.toggled_cells.clear();
    }

    if let Some(cursor_pos) = window.cursor_position() {
        // Calculate world position
        if let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) {
            let (col, row) = world_pos_to_cell(world_pos, mat.cell_size);

            // Debug: print when clicking
            if mouse_btn.just_pressed(MouseButton::Left) {
                println!("Click: cursor_pos={:?}, world_pos={:?}, cell=({}, {})", cursor_pos, world_pos, col, row);
            }

            // --- Toggle cells while dragging ---
            if drag_state.is_dragging && col >= 0 && col < GRID_COLS && row >= 0 && row < GRID_ROWS {
                let cell_coord = (col, row);

                // Only toggle if we haven't toggled this cell yet during this drag
                if !drag_state.toggled_cells.contains(&cell_coord) {
                    drag_state.toggled_cells.insert(cell_coord);

                    let index = (row * GRID_COLS + col) as usize;

                    // Toggle cell state
                    grid_state.cells[index] = match grid_state.cells[index] {
                        0 => 1,
                        1 => 2,
                        _ => 0,
                    };
                }
            }
        }
    }
}

fn setup_ui(mut commands: Commands) {
    // Root UI container
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        })
        .with_children(|parent| {
            // Left panel - Zoom controls
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    ..default()
                })
                .with_children(|parent| {
                    create_button(parent, "Zoom In (+)", CameraButton::ZoomIn);
                    create_button(parent, "Zoom Out (-)", CameraButton::ZoomOut);
                    create_button(parent, "Reset", CameraButton::Reset);
                });

            // Right panel - Pan controls
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    align_items: AlignItems::End,
                    ..default()
                })
                .with_children(|parent| {
                    create_button(parent, "Pan Up (^)", CameraButton::PanUp);

                    parent
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(10.0),
                            ..default()
                        })
                        .with_children(|parent| {
                            create_button(parent, "< Left", CameraButton::PanLeft);
                            create_button(parent, "Right >", CameraButton::PanRight);
                        });

                    create_button(parent, "Pan Down (v)", CameraButton::PanDown);
                });
        });
}

fn create_button(parent: &mut ChildSpawnerCommands, label: &str, button_type: CameraButton) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(120.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
            button_type,
        ))
        .with_child((
            Text::new(label),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));
}

fn handle_camera_buttons(
    interaction_query: Query<
        (&Interaction, &CameraButton),
        Changed<Interaction>,
    >,
    mut commands: Commands,
) {
    for (interaction, button_type) in &interaction_query {
        if *interaction == Interaction::Pressed {
            let action = match button_type {
                CameraButton::ZoomIn => CameraAction::Zoom(0.8),
                CameraButton::ZoomOut => CameraAction::Zoom(1.25),
                CameraButton::PanLeft => CameraAction::Pan(Vec2::new(-100.0, 0.0)),
                CameraButton::PanRight => CameraAction::Pan(Vec2::new(100.0, 0.0)),
                CameraButton::PanUp => CameraAction::Pan(Vec2::new(0.0, 100.0)),
                CameraButton::PanDown => CameraAction::Pan(Vec2::new(0.0, -100.0)),
                CameraButton::Reset => CameraAction::Reset,
            };
            commands.spawn(action);
        }
    }
}

fn handle_keyboard_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    // Zoom controls: +/- or =/- keys
    if keyboard.just_pressed(KeyCode::Equal) || keyboard.just_pressed(KeyCode::NumpadAdd) {
        commands.spawn(CameraAction::Zoom(0.8));
    }
    if keyboard.just_pressed(KeyCode::Minus) || keyboard.just_pressed(KeyCode::NumpadSubtract) {
        commands.spawn(CameraAction::Zoom(1.25));
    }

    // Pan controls: WASD
    if keyboard.just_pressed(KeyCode::KeyW) {
        commands.spawn(CameraAction::Pan(Vec2::new(0.0, 100.0)));
    }
    if keyboard.just_pressed(KeyCode::KeyS) {
        commands.spawn(CameraAction::Pan(Vec2::new(0.0, -100.0)));
    }
    if keyboard.just_pressed(KeyCode::KeyA) {
        commands.spawn(CameraAction::Pan(Vec2::new(-100.0, 0.0)));
    }
    if keyboard.just_pressed(KeyCode::KeyD) {
        commands.spawn(CameraAction::Pan(Vec2::new(100.0, 0.0)));
    }

    // Pan controls: Arrow keys
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        commands.spawn(CameraAction::Pan(Vec2::new(0.0, 100.0)));
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        commands.spawn(CameraAction::Pan(Vec2::new(0.0, -100.0)));
    }
    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        commands.spawn(CameraAction::Pan(Vec2::new(-100.0, 0.0)));
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) {
        commands.spawn(CameraAction::Pan(Vec2::new(100.0, 0.0)));
    }
}

fn apply_camera_actions(
    mut camera_q: Query<&mut Transform, With<Camera2d>>,
    actions_q: Query<(Entity, &CameraAction)>,
    mut commands: Commands,
) {
    let Ok(mut camera_transform) = camera_q.single_mut() else { return };

    for (entity, action) in &actions_q {
        match action {
            CameraAction::Zoom(factor) => {
                camera_transform.scale *= *factor;
            }
            CameraAction::Pan(delta) => {
                // Scale the pan delta by current zoom level
                camera_transform.translation.x += delta.x * camera_transform.scale.x;
                camera_transform.translation.y += delta.y * camera_transform.scale.y;
            }
            CameraAction::Reset => {
                camera_transform.translation = Vec3::ZERO;
                camera_transform.scale = Vec3::ONE;
            }
        }

        // Remove the action entity after processing
        commands.entity(entity).despawn();
    }
}

fn sync_grid_buffer(
    grid_state: Res<GridState>,
    grid_q: Query<&MeshMaterial2d<SpreadsheetGridMaterial>>,
    materials: Res<Assets<SpreadsheetGridMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    if !grid_state.is_changed() { return; }

    let Ok(grid_handle) = grid_q.single() else { return };
    if let Some(mat) = materials.get(grid_handle) {
        if let Some(buffer) = buffers.get_mut(&mat.cell_data) {
            buffer.set_data(grid_state.cells.as_slice());
        }
    }
}