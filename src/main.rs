use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{Material2d, Material2dPlugin},
};

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        Material2dPlugin::<SpreadsheetGridMaterial>::default(),
    ))
    .add_systems(Startup, (setup, setup_ui))
    .add_systems(Update, (
        update_grid_to_camera,
        grid_interaction,
        handle_camera_buttons,
    ));
    app.run();
}

// --- Material Definition ---
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct SpreadsheetGridMaterial {
    #[uniform(0)]
    viewport_top_left: Vec2,
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
}

impl Material2d for SpreadsheetGridMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/grid.wgsl".into()
    }
}

#[derive(Component)]
struct GridBackdrop;

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
) {
    commands.spawn((Camera2d, Transform::from_xyz(0.0, 0.0, 0.0)));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))),
        // CHANGED: MeshMaterial2d accepts our standard Material now
        MeshMaterial2d(materials.add(SpreadsheetGridMaterial {
            viewport_top_left: Vec2::ZERO,
            viewport_size: Vec2::ONE,
            cell_size: Vec2::new(80.0, 30.0),
            line_width: 1.0,
            color_bg: LinearRgba::WHITE,
            color_line: LinearRgba::gray(0.8),
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
            mat.viewport_top_left = Vec2::new(min.x.min(max.x), max.y.max(min.y));
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
) {
    let Ok((camera, cam_transform)) = camera_q.single() else { return };
    let Ok(window) = window_q.single() else { return };
    let Ok(grid_handle) = grid_q.single() else { return };
    let Some(mat) = materials.get(grid_handle) else { return };

    if let Some(cursor_pos) = window.cursor_position() {
        
        // Calculate world position
        if let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) {
            let col = (world_pos.x / mat.cell_size.x).floor() as i32;
            let row = (-world_pos.y / mat.cell_size.y).floor() as i32;

            // --- onMouseDown Handler ---
            if mouse_btn.just_pressed(MouseButton::Left) {
                println!("🔻 Mouse Down at: {:?} -> Cell: ({}, {})", world_pos, col, row);
            }

            // --- onMouseUp Handler ---
            if mouse_btn.just_released(MouseButton::Left) {
                println!("🔺 Mouse Up at: {:?} -> Cell: ({}, {})", world_pos, col, row);
            }
            
            // --- continuous (While Holding) ---
            if mouse_btn.pressed(MouseButton::Left) {
                // logic for dragging / hovering while held
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
                    create_button(parent, "Pan Up (↑)", CameraButton::PanUp);

                    parent
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(10.0),
                            ..default()
                        })
                        .with_children(|parent| {
                            create_button(parent, "← Left", CameraButton::PanLeft);
                            create_button(parent, "Right →", CameraButton::PanRight);
                        });

                    create_button(parent, "Pan Down (↓)", CameraButton::PanDown);
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
    mut camera_q: Query<&mut Transform, With<Camera2d>>,
) {
    let Ok(mut camera_transform) = camera_q.single_mut() else { return };

    for (interaction, button_type) in &interaction_query {
        if *interaction == Interaction::Pressed {
            match button_type {
                CameraButton::ZoomIn => {
                    camera_transform.scale *= 0.8;
                }
                CameraButton::ZoomOut => {
                    camera_transform.scale *= 1.25;
                }
                CameraButton::PanLeft => {
                    camera_transform.translation.x -= 100.0 * camera_transform.scale.x;
                }
                CameraButton::PanRight => {
                    camera_transform.translation.x += 100.0 * camera_transform.scale.x;
                }
                CameraButton::PanUp => {
                    camera_transform.translation.y += 100.0 * camera_transform.scale.y;
                }
                CameraButton::PanDown => {
                    camera_transform.translation.y -= 100.0 * camera_transform.scale.y;
                }
                CameraButton::Reset => {
                    camera_transform.translation = Vec3::ZERO;
                    camera_transform.scale = Vec3::ONE;
                }
            }
        }
    }
}