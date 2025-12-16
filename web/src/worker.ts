// worker.ts

// 1. Defined 'self' for TypeScript (optional but good practice)
// declare const self: DedicatedWorkerGlobalScope;

// 2. Immediate log - this should now ALWAYS show up
console.log('Worker: 🚀 Worker script has started executing!');

// 3. Keepalive ping
setInterval(() => {
    // Use self, not window
    self.postMessage({ type: 'ping', msg: 'Worker: Still alive!' });
}, 1000);

// async function main() {
//   try {
//     console.log('Worker: Attempting to import WASM...');

//     // 4. DYNAMIC IMPORT - This prevents the script from crashing at startup
//     // Note: Adjust the path if necessary, but keep the import inside this function.
//     const pkg = await import('../../pkg/gregsheet');
    
//     console.log('Worker: WASM module imported, initializing...');
    
//     // Initialize WASM memory
//     await pkg.default();
//     console.log('Worker: WASM initialized!');

//     // 5. Setup Message Listener (Wait for Init from Main Thread)
//     self.onmessage = (event) => {
//         const { type, canvas, width, height } = event.data;
        
//         if (type === 'init') {
//             console.log("Worker: Received Init command. Starting Bevy...");
            
//             // Call your Rust entry point (exposed via pkg)
//             // Ensure your Rust function accepts the OffscreenCanvas!
//             pkg.init_game_worker(canvas, width, height); 
//         } 
//         else if (type === 'event') {
//             // pkg.handle_input(event.data.payload);
//         }
//     };
    
//     // Notify main thread we are ready for the canvas
//     self.postMessage({ type: 'ready' });

//   } catch (error) {
//     // This will catch 404s, syntax errors in WASM, etc.
//     console.error('Worker: FATAL ERROR during initialization:', error);
//   }
// }

// main();

export {}; // Makes this a module