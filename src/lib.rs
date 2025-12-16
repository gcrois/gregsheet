use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    render::storage::ShaderStorageBuffer,
    shader::ShaderRef,
    sprite_render::{Material2d, Material2dPlugin},
};
use wasm_bindgen::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
use web_sys;

// Re-export all modules from main.rs
mod cell;
mod gpu_cell;
mod grid_state;
mod formula;
mod evaluator;
mod demo;
mod svg_renderer;

use grid_state::GridState;
use svg_renderer::{SvgRenderer, SvgRenderRequest};
use bevy::render::render_resource::{TextureDimension, TextureFormat, Extent3d};
use bevy::asset::RenderAssetUsages;
use evaluator::{TickControl, EvaluationTimer, tick_evaluation_system};

const GRID_COLS: i32 = 128;
const GRID_ROWS: i32 = 128;

#[wasm_bindgen]
pub fn init_game_worker() {
    console_error_panic_hook::set_once();

    // Get the worker global scope
    let global = js_sys::global()
        .dyn_into::<web_sys::DedicatedWorkerGlobalScope>()
        .expect("Not running in a worker!");

    // Create the game worker instance wrapped in Rc<RefCell<>>
    let worker: Rc<RefCell<Option<GameWorker>>> = Rc::new(RefCell::new(None));
    let worker_clone = worker.clone();

    // Set up message handler
    let onmessage = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
        let data = event.data();

        // Extract message type
        if let Ok(obj) = js_sys::Reflect::get(&data, &"type".into()) {
            if let Some(msg_type) = obj.as_string() {
                match msg_type.as_str() {
                    "init" => {
                        web_sys::console::log_1(&"Initializing game worker...".into());

                        // Extract canvas, width, height
                        let canvas = js_sys::Reflect::get(&data, &"canvas".into())
                            .ok()
                            .and_then(|v| v.dyn_into::<web_sys::OffscreenCanvas>().ok())
                            .expect("Failed to get canvas");

                        let width = js_sys::Reflect::get(&data, &"width".into())
                            .ok()
                            .and_then(|v| v.as_f64())
                            .map(|v| v as u32)
                            .unwrap_or(1920);

                        let height = js_sys::Reflect::get(&data, &"height".into())
                            .ok()
                            .and_then(|v| v.as_f64())
                            .map(|v| v as u32)
                            .unwrap_or(1080);

                        let game_worker = GameWorker::new(canvas, width, height);
                        *worker_clone.borrow_mut() = Some(game_worker);

                        // Start the game loop
                        request_animation_frame(worker_clone.clone());
                    }
                    "event" => {
                        if let Ok(payload_val) = js_sys::Reflect::get(&data, &"payload".into()) {
                            if let Ok(payload) = serde_wasm_bindgen::from_value::<InputEvent>(payload_val) {
                                if let Some(w) = worker_clone.borrow_mut().as_mut() {
                                    w.handle_event(payload);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }) as Box<dyn FnMut(_)>);

    global.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();
}

fn request_animation_frame(worker: Rc<RefCell<Option<GameWorker>>>) {
    let closure = Closure::wrap(Box::new(move || {
        // Run one frame
        if let Some(w) = worker.borrow_mut().as_mut() {
            w.frame();
        }

        // Schedule next frame
        request_animation_frame(worker.clone());
    }) as Box<dyn FnMut()>);

    // In a worker, we need to use setTimeout instead of requestAnimationFrame
    let global = js_sys::global().dyn_into::<web_sys::DedicatedWorkerGlobalScope>().unwrap();
    let _ = global.set_timeout_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        16, // ~60fps
    );

    closure.forget();
}

#[derive(serde::Deserialize)]
struct InputEvent {
    event_type: String,
    #[serde(default)]
    x: Option<f32>,
    #[serde(default)]
    y: Option<f32>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    button: Option<u32>,
}

pub struct GameWorker {
    app: App,
    _canvas: web_sys::OffscreenCanvas,
}

impl GameWorker {
    pub fn new(canvas: web_sys::OffscreenCanvas, width: u32, height: u32) -> Self {
        let mut app = App::new();

        // Add plugins with canvas selector
        app.add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    canvas: Some("#bevy-canvas".into()),
                    resolution: (width, height).into(),
                    ..default()
                }),
                ..default()
            }),
            Material2dPlugin::<SpreadsheetGridMaterial>::default(),
        ))
        .insert_resource({
            let mut grid = GridState::new();
            demo::setup_demo_data(&mut grid);
            grid
        });

        app.insert_resource(SvgRenderer::new());
        app.insert_resource(DragState::default())
            .insert_resource(TickControl::default())
            .insert_resource(EvaluationTimer::default())
            .insert_resource(EditingState::default())
            .insert_resource(LensState::default())
            .add_systems(Startup, (setup, setup_ui))
            .add_systems(Update, (
                tick_evaluation_system,
                update_grid_to_camera,
                grid_interaction,
                handle_camera_buttons,
                handle_tick_buttons,
                handle_lens_buttons,
                update_tick_button_text,
                update_lens_button_text,
                handle_keyboard_input,
                handle_editor_input,
                update_editor_display,
                apply_camera_actions,
                sync_grid_buffer,
                manage_svg_cells
            ));

        Self {
            app,
            _canvas: canvas,
        }
    }

    pub fn handle_event(&mut self, event: InputEvent) {
        // Handle events - for now, just log them
        // In a full implementation, you'd inject these into Bevy's event system
        web_sys::console::log_1(&format!("Event: {:?}", event.event_type).into());
    }

    pub fn frame(&mut self) {
        self.app.update();
    }
}

// Copy all the components, resources, and systems from main.rs

#[derive(Resource, Default)]
struct EditingState {
    pub active_cell: Option<(i32, i32)>,
    pub buffer: String,
}

#[derive(Resource)]
struct LensState {
    pub show_value: bool,
    pub show_position: bool,
    pub show_formula: bool,
    pub show_grid: bool,
}

impl Default for LensState {
    fn default() -> Self {
        Self {
            show_value: true,
            show_position: false,
            show_formula: false,
            show_grid: true,
        }
    }
}

#[derive(Component)]
struct EditorText;

#[derive(Component)]
enum LensButton {
    Value,
    Position,
    Formula,
    Grid,
}

#[derive(Resource, Default)]
struct DragState {
    is_dragging: bool,
    toggled_cells: std::collections::HashSet<(i32, i32)>,
}

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
    #[uniform(0)]
    show_grid: f32,
    #[storage(1, read_only)]
    cell_data: Handle<ShaderStorageBuffer>,
    #[texture(2, dimension = "2d_array")]
    #[sampler(3)]
    rich_cell_textures: Handle<Image>,
    #[storage(4, read_only)]
    rich_cell_indices: Handle<ShaderStorageBuffer>,
}

impl Material2d for SpreadsheetGridMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/grid.wgsl".into()
    }
}

#[derive(Component)]
struct GridBackdrop;

fn world_pos_to_cell(world_pos: Vec2, cell_size: Vec2) -> (i32, i32) {
    let col = (world_pos.x / cell_size.x).floor() as i32;
    let row = (-world_pos.y / cell_size.y).floor() as i32;
    (col, row)
}

#[derive(Component, Clone, Copy, Debug)]
enum CameraAction {
    Zoom(f32),
    Pan(Vec2),
    Reset,
}

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

#[derive(Component)]
enum TickButton {
    ManualTick,
    AutoTickToggle,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SpreadsheetGridMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    commands.spawn((Camera2d, Transform::from_xyz(0.0, 0.0, 0.0)));

    let buffer_handle = buffers.add(ShaderStorageBuffer::from(vec![0u32]));
    let indices_handle = buffers.add(ShaderStorageBuffer::from(vec![-1i32]));

    let dummy_texture = Image::new(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 2,
        },
        TextureDimension::D2,
        vec![0, 0, 0, 0, 0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    let texture_handle = images.add(dummy_texture);

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
            show_grid: 1.0,
            cell_data: buffer_handle,
            rich_cell_textures: texture_handle,
            rich_cell_indices: indices_handle,
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

        if let Some(mat) = materials.get_mut(&grid_handle.0) {
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
    mut editing_state: ResMut<EditingState>,
) {
    let Ok((camera, cam_transform)) = camera_q.single() else { return };
    let Ok(window) = window_q.single() else { return };
    let Ok(grid_handle) = grid_q.single() else { return };
    let Some(mat) = materials.get(&grid_handle.0) else { return };

    if mouse_btn.just_pressed(MouseButton::Left) {
        drag_state.is_dragging = true;
        drag_state.toggled_cells.clear();
    }

    if mouse_btn.just_released(MouseButton::Left) {
        drag_state.is_dragging = false;
        drag_state.toggled_cells.clear();
    }

    if let Some(cursor_pos) = window.cursor_position() {
        if let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) {
            let (col, row) = world_pos_to_cell(world_pos, mat.cell_size);

            if mouse_btn.just_pressed(MouseButton::Left) {
                grid_state.selected.clear();
                grid_state.selected.insert((col, row));

                editing_state.active_cell = Some((col, row));
                if let Some(cell) = grid_state.get_cell(col, row) {
                    editing_state.buffer = cell.raw.clone();
                } else {
                    editing_state.buffer = String::new();
                }
            }

            if drag_state.is_dragging {
                let cell_coord = (col, row);
                if !drag_state.toggled_cells.contains(&cell_coord) {
                    drag_state.toggled_cells.insert(cell_coord);
                    grid_state.selected.insert(cell_coord);
                }
            }
        }
    }
}

fn setup_ui(mut commands: Commands) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        })
        .with_children(|parent| {
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
                    parent.spawn(Node { height: Val::Px(20.0), ..default() });
                    create_tick_button(parent, "Tick", TickButton::ManualTick);
                    create_tick_button(parent, "Auto Tick: OFF", TickButton::AutoTickToggle);

                    parent.spawn(Node { height: Val::Px(20.0), ..default() });
                    create_lens_button(parent, "Value: ON", LensButton::Value);
                    create_lens_button(parent, "Pos: OFF", LensButton::Position);
                    create_lens_button(parent, "Formula: OFF", LensButton::Formula);
                    create_lens_button(parent, "Grid: ON", LensButton::Grid);
                });

            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(150.0),
                        top: Val::Px(10.0),
                        width: Val::Px(400.0),
                        height: Val::Px(40.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Start,
                        padding: UiRect::all(Val::Px(5.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                    BorderColor::from(Color::srgb(0.3, 0.3, 0.3)),
                ))
                .with_child((
                    Text::new("Formula: "),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::WHITE),
                ))
                .with_child((
                    Text::new(""),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::WHITE),
                    EditorText,
                ));

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
            TextFont { font_size: 14.0, ..default() },
            TextColor(Color::WHITE),
        ));
}

fn create_tick_button(parent: &mut ChildSpawnerCommands, label: &str, button_type: TickButton) {
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
            BackgroundColor(Color::srgb(0.2, 0.5, 0.2)),
            button_type,
        ))
        .with_child((
            Text::new(label),
            TextFont { font_size: 14.0, ..default() },
            TextColor(Color::WHITE),
        ));
}

fn create_lens_button(parent: &mut ChildSpawnerCommands, label: &str, button_type: LensButton) {
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
            BackgroundColor(Color::srgb(0.2, 0.2, 0.5)),
            button_type,
        ))
        .with_child((
            Text::new(label),
            TextFont { font_size: 14.0, ..default() },
            TextColor(Color::WHITE),
        ));
}

fn handle_lens_buttons(
    interaction_query: Query<(&Interaction, &LensButton), Changed<Interaction>>,
    mut lens_state: ResMut<LensState>,
    mut materials: ResMut<Assets<SpreadsheetGridMaterial>>,
    grid_q: Query<&MeshMaterial2d<SpreadsheetGridMaterial>>,
) {
    for (interaction, button) in &interaction_query {
        if *interaction == Interaction::Pressed {
            match button {
                LensButton::Value => lens_state.show_value = !lens_state.show_value,
                LensButton::Position => lens_state.show_position = !lens_state.show_position,
                LensButton::Formula => lens_state.show_formula = !lens_state.show_formula,
                LensButton::Grid => {
                    lens_state.show_grid = !lens_state.show_grid;
                    if let Ok(grid_handle) = grid_q.single() {
                        if let Some(mat) = materials.get_mut(&grid_handle.0) {
                            mat.show_grid = if lens_state.show_grid { 1.0 } else { 0.0 };
                        }
                    }
                }
            }
        }
    }
}

fn update_lens_button_text(
    lens_state: Res<LensState>,
    mut button_query: Query<(&LensButton, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    if !lens_state.is_changed() { return; }
    for (button, children) in &mut button_query {
        let text_val = match button {
            LensButton::Value => format!("Value: {}", if lens_state.show_value { "ON" } else { "OFF" }),
            LensButton::Position => format!("Pos: {}", if lens_state.show_position { "ON" } else { "OFF" }),
            LensButton::Formula => format!("Formula: {}", if lens_state.show_formula { "ON" } else { "OFF" }),
            LensButton::Grid => format!("Grid: {}", if lens_state.show_grid { "ON" } else { "OFF" }),
        };
        for child in children {
            if let Ok(mut text) = text_query.get_mut(*child) {
                **text = text_val.clone();
            }
        }
    }
}

fn handle_camera_buttons(
    interaction_query: Query<(&Interaction, &CameraButton), Changed<Interaction>>,
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

fn handle_tick_buttons(
    interaction_query: Query<(&Interaction, &TickButton), Changed<Interaction>>,
    mut tick_control: ResMut<TickControl>,
) {
    for (interaction, button_type) in &interaction_query {
        if *interaction == Interaction::Pressed {
            match button_type {
                TickButton::ManualTick => {
                    tick_control.manual_tick_requested = true;
                }
                TickButton::AutoTickToggle => {
                    tick_control.auto_tick_enabled = !tick_control.auto_tick_enabled;
                }
            }
        }
    }
}

fn update_tick_button_text(
    tick_control: Res<TickControl>,
    mut button_query: Query<(&TickButton, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    if !tick_control.is_changed() { return; }
    for (button_type, children) in &mut button_query {
        if let TickButton::AutoTickToggle = button_type {
            for child in children {
                if let Ok(mut text) = text_query.get_mut(*child) {
                    **text = if tick_control.auto_tick_enabled {
                        "Auto Tick: ON".to_string()
                    } else {
                        "Auto Tick: OFF".to_string()
                    };
                }
            }
        }
    }
}

fn handle_keyboard_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    if keyboard.just_pressed(KeyCode::Equal) || keyboard.just_pressed(KeyCode::NumpadAdd) {
        commands.spawn(CameraAction::Zoom(0.8));
    }
    if keyboard.just_pressed(KeyCode::Minus) || keyboard.just_pressed(KeyCode::NumpadSubtract) {
        commands.spawn(CameraAction::Zoom(1.25));
    }
    if keyboard.just_pressed(KeyCode::ArrowUp) { commands.spawn(CameraAction::Pan(Vec2::new(0.0, 100.0))); }
    if keyboard.just_pressed(KeyCode::ArrowDown) { commands.spawn(CameraAction::Pan(Vec2::new(0.0, -100.0))); }
    if keyboard.just_pressed(KeyCode::ArrowLeft) { commands.spawn(CameraAction::Pan(Vec2::new(-100.0, 0.0))); }
    if keyboard.just_pressed(KeyCode::ArrowRight) { commands.spawn(CameraAction::Pan(Vec2::new(100.0, 0.0))); }
}

fn handle_editor_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut editing_state: ResMut<EditingState>,
    mut grid_state: ResMut<GridState>,
) {
    if editing_state.active_cell.is_none() { return; }

    if keyboard.just_pressed(KeyCode::Enter) {
        if let Some((col, row)) = editing_state.active_cell {
            grid_state.get_cell_mut_or_create(col, row).set_raw(editing_state.buffer.clone());
        }
        return;
    }

    if keyboard.just_pressed(KeyCode::Backspace) {
        editing_state.buffer.pop();
    }

    for key in keyboard.get_just_pressed() {
        let char = match key {
            KeyCode::KeyA => Some('A'),
            KeyCode::KeyB => Some('B'),
            KeyCode::KeyC => Some('C'),
            KeyCode::KeyD => Some('D'),
            KeyCode::Digit0 => Some('0'),
            KeyCode::Digit1 => Some('1'),
            KeyCode::Digit2 => Some('2'),
            KeyCode::Digit3 => Some('3'),
            KeyCode::Digit4 => Some('4'),
            KeyCode::Digit5 => Some('5'),
            KeyCode::Digit6 => Some('6'),
            KeyCode::Digit7 => Some('7'),
            KeyCode::Digit8 => Some('8'),
            KeyCode::Digit9 => Some('9'),
            KeyCode::Space => Some(' '),
            KeyCode::Equal | KeyCode::NumpadEqual => Some('='),
            KeyCode::NumpadAdd => Some('+'),
            KeyCode::Minus | KeyCode::NumpadSubtract => Some('-'),
            _ => None,
        };

        if let Some(c) = char {
            editing_state.buffer.push(c);
        }
    }
}

fn update_editor_display(
    editing_state: Res<EditingState>,
    mut query: Query<&mut Text, With<EditorText>>,
) {
    for mut text in &mut query {
        if let Some((col, row)) = editing_state.active_cell {
            **text = format!("({}, {}): {}", col, row, editing_state.buffer);
        } else {
            **text = "Select a cell".to_string();
        }
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
            CameraAction::Zoom(factor) => { camera_transform.scale *= *factor; }
            CameraAction::Pan(delta) => {
                camera_transform.translation.x += delta.x * camera_transform.scale.x;
                camera_transform.translation.y += delta.y * camera_transform.scale.y;
            }
            CameraAction::Reset => {
                camera_transform.translation = Vec3::ZERO;
                camera_transform.scale = Vec3::ONE;
            }
        }
        commands.entity(entity).despawn();
    }
}

fn sync_grid_buffer(
    grid_state: Res<GridState>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    grid_q: Query<&MeshMaterial2d<SpreadsheetGridMaterial>>,
    mut materials: ResMut<Assets<SpreadsheetGridMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    let Ok((camera, cam_transform)) = camera_q.single() else { return };
    let Ok(grid_handle) = grid_q.single() else { return };
    let Some(mat) = materials.get_mut(&grid_handle.0) else { return };

    let Some(rect) = camera.logical_viewport_rect() else { return };
    let min_world = camera.viewport_to_world_2d(cam_transform, rect.min).ok();
    let max_world = camera.viewport_to_world_2d(cam_transform, rect.max).ok();

    if let (Some(min), Some(max)) = (min_world, max_world) {
        let bottom_left = Vec2::new(min.x.min(max.x), min.y.min(max.y));
        let top_right = Vec2::new(min.x.max(max.x), min.y.max(max.y));

        let min_col = (bottom_left.x / mat.cell_size.x).floor() as i32;
        let max_col = (top_right.x / mat.cell_size.x).ceil() as i32;
        let min_row = (-top_right.y / mat.cell_size.y).floor() as i32;
        let max_row = (-bottom_left.y / mat.cell_size.y).ceil() as i32;

        let width = max_col - min_col + 1;
        let height = max_row - min_row + 1;

        mat.grid_dimensions = Vec2::new(width as f32, height as f32);

        if let Some(buffer) = buffers.get_mut(&mat.cell_data) {
            let gpu_data = grid_state.to_gpu_cells_viewport(min_col, min_row, width, height);
            buffer.set_data(gpu_data.as_slice());
        }
    }
}

fn manage_svg_cells(
    mut svg_renderer: ResMut<SvgRenderer>,
    grid_state: Res<GridState>,
    lens_state: Res<LensState>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    grid_q: Query<&MeshMaterial2d<SpreadsheetGridMaterial>>,
    mut materials: ResMut<Assets<SpreadsheetGridMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut last_visible_rich_cells: Local<Vec<(i32, i32)>>,
) {
    let Ok((camera, cam_transform)) = camera_q.single() else { return };
    let Ok(grid_handle) = grid_q.single() else { return };
    let Some(mat) = materials.get_mut(&grid_handle.0) else { return };

    let Some(rect) = camera.logical_viewport_rect() else { return };
    let min_world = camera.viewport_to_world_2d(cam_transform, rect.min).ok();
    let max_world = camera.viewport_to_world_2d(cam_transform, rect.max).ok();

    let mut current_visible_cells = Vec::new();
    let mut min_col = 0;
    let mut min_row = 0;
    let mut width = 0;
    let mut height = 0;

    if let (Some(min), Some(max)) = (min_world, max_world) {
        let bottom_left = Vec2::new(min.x.min(max.x), min.y.min(max.y));
        let top_right = Vec2::new(min.x.max(max.x), min.y.max(max.y));

        min_col = (bottom_left.x / mat.cell_size.x).floor() as i32;
        let max_col = (top_right.x / mat.cell_size.x).ceil() as i32;
        min_row = (-top_right.y / mat.cell_size.y).floor() as i32;
        let max_row = (-bottom_left.y / mat.cell_size.y).ceil() as i32;

        width = max_col - min_col + 1;
        height = max_row - min_row + 1;

        for row in min_row..=max_row {
            for col in min_col..=max_col {
                current_visible_cells.push((col, row));

                if let Some(cell) = grid_state.get_cell(col, row) {
                    let svg = generate_svg(cell, col, row, &lens_state);
                    let hash = seahash::hash(svg.as_bytes());

                    if !svg_renderer.is_cached(hash) {
                        svg_renderer.request_render(SvgRenderRequest {
                            cell_coord: (col, row),
                            svg,
                            width: 80,
                            height: 30,
                            content_hash: hash,
                        });
                    }
                }
            }
        }
    }

    let results = svg_renderer.poll_results();
    let results_received = !results.is_empty();

    current_visible_cells.sort();
    let visibility_changed = *last_visible_rich_cells != current_visible_cells;

    if results_received || visibility_changed {
        *last_visible_rich_cells = current_visible_cells.clone();

        let mut texture_data = Vec::new();
        let mut index_map = vec![-1i32; (width * height) as usize];
        let mut layer_count = 0;
        let mut hash_to_layer = std::collections::HashMap::new();

        for (col, row) in &current_visible_cells {
            let rel_x = col - min_col;
            let rel_y = row - min_row;
            if rel_x < 0 || rel_x >= width || rel_y < 0 || rel_y >= height { continue; }

            let viewport_idx = (rel_y * width + rel_x) as usize;

            if let Some(cell) = grid_state.get_cell(*col, *row) {
                let svg = generate_svg(cell, *col, *row, &lens_state);
                let hash = seahash::hash(svg.as_bytes());

                if let Some(buffer) = svg_renderer.pixel_cache.get(&hash) {
                    if let Some(&existing_layer) = hash_to_layer.get(&hash) {
                        index_map[viewport_idx] = existing_layer as i32;
                    } else {
                        texture_data.extend_from_slice(buffer);
                        index_map[viewport_idx] = layer_count;
                        hash_to_layer.insert(hash, layer_count);
                        layer_count += 1;
                    }
                }
            }
        }

        if let Some(buffer) = buffers.get_mut(&mat.rich_cell_indices) {
             buffer.set_data(index_map.as_slice());
        }

        if layer_count > 0 {
             let final_layer_count = if layer_count == 1 { 2 } else { layer_count };
             if layer_count == 1 {
                 texture_data.resize(texture_data.len() * 2, 0);
             }

             let texture_array = Image::new(
                Extent3d {
                    width: 80,
                    height: 30,
                    depth_or_array_layers: final_layer_count as u32,
                },
                TextureDimension::D2,
                texture_data,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            );
            mat.rich_cell_textures = images.add(texture_array);
        }
    }
}

fn generate_svg(cell: &crate::cell::Cell, col: i32, row: i32, lens_state: &LensState) -> String {
    let mut elements = String::new();

    let is_rich = (col == 0 && row == 2) || (col == 1 && row == 2);

    if is_rich && lens_state.show_value {
        if col == 0 && row == 2 {
            elements.push_str(r##"<rect width="80" height="30" fill="#e0f7fa"/><text x="5" y="20" font-family="sans-serif" font-size="12" fill="#006064">Status: OK</text>"##);
        } else if col == 1 && row == 2 {
            elements.push_str(r##"<circle cx="15" cy="15" r="8" fill="#4caf50"/><text x="30" y="20" font-family="sans-serif" font-size="12" fill="#333">Active</text>"##);
        }
    } else if lens_state.show_value {
        let text = match &cell.value {
            evalexpr::Value::Int(i) => i.to_string(),
            evalexpr::Value::Float(f) => format!("{:.2}", f),
            evalexpr::Value::String(s) => s.clone(),
            evalexpr::Value::Boolean(b) => b.to_string(),
            evalexpr::Value::Empty => "".to_string(),
            evalexpr::Value::Tuple(_) => "Tuple".to_string(),
        };
        elements.push_str(&format!(r##"<text x="40" y="20" font-family="sans-serif" font-size="14" fill="black" text-anchor="middle">{}</text>"##, text));
    }

    if lens_state.show_position {
        let coord_text = crate::formula::coord_to_name(col, row);
        elements.push_str(&format!(r##"<text x="2" y="8" font-family="sans-serif" font-size="8" fill="#aaaaaa">{}</text>"##, coord_text));
    }

    if lens_state.show_formula && cell.is_formula {
        let formula = cell.raw.replace("<", "&lt;").replace(">", "&gt;").replace("&", "&amp;");
        elements.push_str(&format!(r##"<text x="2" y="28" font-family="sans-serif" font-size="8" fill="blue">{}</text>"##, formula));
    }

    format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="80" height="30">{}</svg>"##, elements)
}
