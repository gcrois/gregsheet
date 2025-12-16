// Simple classic worker (non-module)
console.log('🔥 SIMPLE WORKER LOADED - CLASSIC SCRIPT');

self.postMessage({ type: 'loaded', message: 'Simple worker is alive!' });

self.onmessage = function(e) {
  console.log('Simple worker received:', e.data);
  self.postMessage({ type: 'echo', data: e.data });
};

// Keep alive
setInterval(() => {
  console.log('Simple worker: ping');
}, 2000);
