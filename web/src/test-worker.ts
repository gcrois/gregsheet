console.log('TEST WORKER LOADED!!!');

self.postMessage({ type: 'test', message: 'Hello from test worker' });

self.onmessage = (e) => {
  console.log('Test worker received message:', e.data);
  self.postMessage({ type: 'echo', data: e.data });
};
