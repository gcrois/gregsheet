# Gregsheet - Project Summary

Gregsheet is a high-performance, GPU-accelerated spreadsheet application built with Rust and the Bevy game engine. Originally created for the 2025 Langjam GameJam, it explores the intersection of game engine technology and productivity software to create a highly responsive and visual grid environment.

## Core Features

- **High-Performance Grid**: Supports large grid dimensions (default 128x128) with zero-overhead rendering for thousands of cells.
- **Formula Evaluation**: Integrated formula engine (via `evalexpr`) supporting cell references (e.g., `= A0 + B0`), arithmetic, and logic.
- **Rich Content Rendering**: Native support for SVG rendering within cells, allowing for complex visualizations, icons, and custom styling beyond simple text.
- **Reactive Updates**: Tick-based evaluation system enabling time-dependent simulations (e.g., cellular automata, logic gates) and dynamic updates.
- **Interactive Camera**: Smooth pan and zoom controls for navigating the infinite canvas.

## Architecture & Technical Design

Gregsheet leverages Bevy's ECS (Entity Component System) and `wgpu` capabilities to achieve performance that traditional DOM-based or immediate-mode GUI spreadsheets struggle to match.

### 1. Hybrid Rendering Pipeline
The rendering system bypasses standard sprite rendering for the grid cells, utilizing a custom shader approach:

- **Grid Shader (`assets/shaders/grid.wgsl`)**:
  - The entire grid is rendered in a **single draw call**.
  - **Procedural Rendering**: Grid lines and background colors are computed procedurally in the fragment shader.
  - **Bitmask Font**: Numeric values are rendered directly in the shader using a lightweight 4x5 bitmask font, eliminating the need for thousands of individual Text entities.
  - **Storage Buffers**: Cell data is uploaded to the GPU via a `StorageBuffer<GpuCell>`, allowing the shader to access the state of any cell (value, selection status, error state) in constant time.

### 2. Rich SVG Integration
To support complex graphics without compromising the single-pass architecture:

- **Async Rasterization**: SVG content is parsed and rasterized to RGBA buffers on a dedicated background thread (using `resvg` and `tiny-skia`) to ensure the UI never stutters.
- **Texture Array Composition**: Rasterized images are aggregated into a dynamic `Texture2DArray`.
- **Shader Composition**: The grid shader samples from this array using an index buffer, compositing the high-fidelity SVG content over the cell background. This maintains the "one draw call" philosophy even with rich media.

### 3. Data Model
The application maintains a strict separation between CPU logic and GPU presentation:

- **`GridState` (CPU)**: The authoritative data store.
  - Stores `Cell` structs with raw text, parsed formulas, dependency information, and full SVG source.
  - Handles formula evaluation and dependency resolution.
- **`GpuCell` (GPU)**: A compact, 8-byte representation synced to the GPU for rendering.
  - `value: i32`: The computed numeric result.
  - `flags: u32`: Bitpacked state (Selected, Is Formula, Error, Has Rich Content).

## Codebase Structure

- **`src/main.rs`**: Application entry point, system registration, and main loop.
- **`src/grid_state.rs`**: Core data structures and grid logic.
- **`src/svg_renderer.rs`**: Manages the background SVG rendering thread, texture caching, and GPU upload synchronization.
- **`src/evaluator.rs`**: Handles the tick-based simulation loop and formula updates.
- **`assets/shaders/grid.wgsl`**: The custom WGSL shader defining the visual appearance of the grid.

## Recent Developments
The latest update introduced the **Rich Content** feature, enabling cells to display arbitrary SVG graphics. This required:
- Extending the `Cell` and `GpuCell` structures.
- Implementing a thread-safe `SvgRenderer` resource.
- Upgrading the shader to support `texture_2d_array` bindings.
- Ensuring robust fallback and error handling for invalid or missing content.

## Roadmap
- **WebAssembly (Wasm) Support**: Leveraging `wgpu`'s cross-platform capabilities to run in the browser.
- **Game Mechanics**: Extending the formula engine to interact with game systems (inventory, automation).
- **Advanced Editing**: Improving the input experience with a dedicated formula bar and syntax highlighting.
