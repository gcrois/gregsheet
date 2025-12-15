use bevy::prelude::*;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::collections::{HashMap, HashSet};
use std::thread;

#[derive(Resource)]
pub struct SvgRenderer {
    request_tx: Sender<SvgRenderRequest>,
    result_rx: Receiver<SvgRenderResult>,

    /// Tracks pending render requests to avoid duplicates
    pub pending_renders: HashSet<(i32, i32)>,

    /// Caches rendered RGBA buffers by content hash
    pub pixel_cache: HashMap<u64, Vec<u8>>,
}

pub struct SvgRenderRequest {
    pub cell_coord: (i32, i32),
    pub svg: String,
    pub width: u32,
    pub height: u32,
    pub content_hash: u64,
}

pub struct SvgRenderResult {
    pub cell_coord: (i32, i32),
    pub rgba_buffer: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub content_hash: u64,
}

impl SvgRenderer {
    pub fn new() -> Self {
        let (req_tx, req_rx) = bounded::<SvgRenderRequest>(100);
        let (res_tx, res_rx) = bounded::<SvgRenderResult>(100);

        thread::spawn(move || {
            render_loop(req_rx, res_tx);
        });

        Self {
            request_tx: req_tx,
            result_rx: res_rx,
            pending_renders: HashSet::new(),
            pixel_cache: HashMap::new(),
        }
    }

    pub fn request_render(&mut self, req: SvgRenderRequest) {
        if !self.pending_renders.contains(&req.cell_coord) {
            self.pending_renders.insert(req.cell_coord);
            let _ = self.request_tx.send(req);
        }
    }

    pub fn poll_results(&mut self) -> Vec<SvgRenderResult> {
        let mut results = Vec::new();
        while let Ok(res) = self.result_rx.try_recv() {
            self.pending_renders.remove(&res.cell_coord);
            
            // Cache the result
            self.pixel_cache.insert(res.content_hash, res.rgba_buffer.clone());
            results.push(res);
        }
        results
    }
    
    pub fn is_cached(&self, hash: u64) -> bool {
        self.pixel_cache.contains_key(&hash)
    }
}

fn render_loop(rx: Receiver<SvgRenderRequest>, tx: Sender<SvgRenderResult>) {
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();

    let mut options = usvg::Options::default();
    options.fontdb = std::sync::Arc::new(fontdb);

    while let Ok(req) = rx.recv() {
        let buffer = render_svg_to_buffer(&req.svg, req.width, req.height, &options);
        
        // If rendering failed (empty buffer), we might want to send a placeholder or error
        // For now, we assume it works or returns a blank buffer
        let _ = tx.send(SvgRenderResult {
            cell_coord: req.cell_coord,
            rgba_buffer: buffer,
            width: req.width,
            height: req.height,
            content_hash: req.content_hash,
        });
    }
}

fn render_svg_to_buffer(svg_data: &str, width: u32, height: u32, options: &usvg::Options) -> Vec<u8> {
    // Parse SVG
    let tree = match usvg::Tree::from_str(svg_data, options) {
        Ok(t) => t,
        Err(_) => return vec![0; (width * height * 4) as usize], // Return empty transparent buffer on error
    };
    
    let size = tree.size();
    let svg_width = size.width();
    let svg_height = size.height();
    
    let scale_x = width as f32 / svg_width;
    let scale_y = height as f32 / svg_height;

    let transform = tiny_skia::Transform::from_scale(scale_x, scale_y);

    let mut pixmap = tiny_skia::Pixmap::new(width, height).unwrap();
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Convert to simple Vec<u8> (RGBA)
    pixmap.take()
}
