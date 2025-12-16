# Bevy in a Web Worker Migration Guide

This guide describes how to migrate an existing Bevy game to run inside a Web Worker. This architecture moves the game loop off the main thread, ensuring the UI remains responsive and providing a smoother gaming experience on the web.

## Architecture Overview

1.  **Main Thread (UI)**: Handles DOM, user input, and creating the canvas. Transfers the canvas to the worker.
2.  **Web Worker**: Hosting the WASM module. Receives inputs and the canvas.
3.  **Rust/Bevy**: Runs the game loop, processes inputs manually, and renders to the offscreen canvas.

## Prerequisites

-   A Bevy game project.
-   A web bundler (Vite recommended) with WASM support.
-   `wasm-pack` for building the Rust code.

---

## Step 1: Frontend Setup (The Host)

In your web application (e.g., React/Vite), you need to create the worker and transfer control of the canvas.

### 1. Create the Worker and Canvas

```typescript
// App.tsx
import { useEffect, useRef } from 'react';

export function Game() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const workerRef = useRef<Worker | null>(null);

  useEffect(() => {
    if (!canvasRef.current) return;

    // 1. Create the worker
    const worker = new Worker(new URL('./worker.ts', import.meta.url), {
      type: 'module',
    });
    workerRef.current = worker;

    // 2. Transfer the canvas
    // This makes the canvas unusable on the main thread but available in the worker
    const offscreen = canvasRef.current.transferControlToOffscreen();
    
    worker.postMessage({ 
      type: 'init', 
      canvas: offscreen,
      width: canvasRef.current.width,
      height: canvasRef.current.height
    }, [offscreen]); // Important: Transfer the offscreen canvas in the second argument

    return () => worker.terminate();
  }, []);

  return <canvas ref={canvasRef} style={{ width: '100%', height: '100vh' }} />;
}
```

### 2. Proxy Inputs

Since the worker doesn't have access to the DOM, you must capture events on the main thread and send them to the worker.

```typescript
// Add this inside your component or effect
useEffect(() => {
  const handleMouseMove = (e: MouseEvent) => {
    workerRef.current?.postMessage({
      type: 'event',
      payload: {
        event_type: 'mousemove',
        x: e.clientX,
        y: e.clientY,
      }
    });
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    workerRef.current?.postMessage({
      type: 'event',
      payload: {
        event_type: 'keydown',
        key: e.code,
      }
    });
  };

  window.addEventListener('mousemove', handleMouseMove);
  window.addEventListener('keydown', handleKeyDown);

  return () => {
    window.removeEventListener('mousemove', handleMouseMove);
    window.removeEventListener('keydown', handleKeyDown);
  };
}, []);
```

---

## Step 2: Worker Entry Point

Create a TypeScript worker file (`worker.ts`) that initializes the WASM module.

```typescript
// worker.ts
import init, { init_game_worker } from './pkg/your_wasm_package';

async function main() {
  // Initialize the WASM module
  await init();
  
  // Call the Rust entry point
  init_game_worker();
}

main();
```

---

## Step 3: Rust Implementation

You need to modify your Bevy app to accept external control instead of running its own event loop via `winit`.

### 1. Dependencies

Ensure `Cargo.toml` has the necessary features.

```toml
[dependencies]
bevy = { version = "0.15", default-features = false, features = [
    "webgl2",
    "bevy_render",
    "bevy_asset",
    # Note: We might disable bevy_winit if we are manually driving the loop, 
    # but keeping it is often easier if we just use a custom runner.
] }
wasm-bindgen = "0.2"
serde = { version = "1.0", features = ["derive"] }
serde-wasm-bindgen = "0.6"
web-sys = { version = "0.3", features = ["OffscreenCanvas", "MessageEvent"] }
```

### 2. The Worker Entry Point (`lib.rs`)

Create a struct to hold your App instance and expose it to JS.

```rust
use bevy::prelude::*;
use wasm_bindgen::prelude::*;
use web_sys::{OffscreenCanvas, MessageEvent};

#[wasm_bindgen]
pub fn init_game_worker() {
    // Set up a global message handler for the worker
    let global = js_sys::global().unchecked_into::<web_sys::DedicatedWorkerGlobalScope>();
    
    // In a real implementation, you'd store the App in a RefCell/Mutex to persist it
    // and access it inside the onmessage callback.
    // For simplicity, this guide outlines the structure.
    
    // See packages/rust-gui/worker/src/lib.rs for the closure setup pattern.
}

pub struct GameWorker {
    app: App,
}

impl GameWorker {
    pub fn new(canvas: OffscreenCanvas) -> Self {
        let mut app = App::new();
        
        // Configure Bevy to use the provided OffscreenCanvas
        // Note: This requires specific window plugin configuration to target the 
        // OffscreenCanvas or using a custom rendering surface setup.
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                // In a worker, we don't have a DOM selector.
                // You often need to use `canvas` field with a raw web_sys element 
                // if the Bevy version supports it, or configure the renderer manually.
                canvas: Some("#canvas".into()), // Placeholder
                ..default()
            }),
            ..default()
        }));

        // Use a custom runner so we can step the app manually
        app.set_runner(|_| {}); 

        app.add_systems(Startup, setup_game);

        Self { app }
    }

    pub fn handle_message(&mut self, msg: JsValue) {
        // Parse message (e.g., Input events)
        // Inject events into Bevy's Events<T> resources
        // e.g. self.app.world.resource_mut::<Events<CursorMoved>>().send(...);
    }

    pub fn frame(&mut self) {
        // Run one frame of the game
        self.app.update();
    }
}
```

### 3. Handling Inputs

Since `winit` (Bevy's default windowing library) relies on main-thread window events, you must manually feed the events received from the main thread into Bevy.

```rust
// Example: Processing a mouse move
fn process_mouse_move(app: &mut App, x: f32, y: f32) {
    let mut events = app.world_mut().resource_mut::<Events<CursorMoved>>();
    events.send(CursorMoved {
        window: Entity::PLACEHOLDER, // You'll need the primary window entity
        position: Vec2::new(x, y),
        delta: None,
    });
}
```

## Summary of Changes

| Feature | Standard Bevy Web | Worker Bevy |
|box|---|---|
| **Entry Point** | `app.run()` | Custom `init` function that holds `App` state |
| **Canvas** | Auto-created or by ID | `OffscreenCanvas` passed from main thread |
| **Loop** | `winit` Event Loop | `requestAnimationFrame` in Worker calling `app.update()` |
| **Inputs** | Auto-handled by `winit` | Manually proxied via `postMessage` |

## Reference Implementation

See `packages/rust-gui/worker` in this repository for a complete example of the worker message handling and loop structure (implemented with a custom renderer), and `apps/website` for the frontend integration.
