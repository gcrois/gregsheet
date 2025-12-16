import { useEffect, useRef } from "react";
// import rWorker from "./worker.ts?worker";
// import rWorker from "./test-worker.ts?worker";

function App() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const workerRef = useRef<Worker | null>(null);
  const initializedRef = useRef(false);

  useEffect(() => {
    console.log("=== APP EFFECT RUNNING ===");
    if (!canvasRef.current || initializedRef.current) return;
    initializedRef.current = true;

    console.log("Creating worker...");

    // 1. Create a classic worker from public directory
    const worker = new Worker('/simple-worker.js');
    console.log("Worker created:", worker);

    worker.onmessage = (event) => {
        const { data } = event;
        console.log("Message from worker:", data);
    };

    worker.onerror = (error) => {
        console.error("Worker error:", error);
    };

    worker.onmessageerror = (error) => {
        console.error("Worker message error:", error);
    };

    workerRef.current = worker;

    // 2. Transfer the canvas
    const canvas = canvasRef.current;
    const offscreen = canvas.transferControlToOffscreen();

    console.log("Transferring canvas to worker...", {
      width: canvas.width,
      height: canvas.height,
    });

    worker.postMessage(
      {
        type: "init",
        canvas: offscreen,
        width: canvas.width,
        height: canvas.height,
      },
      [offscreen]
    );

    // 3. Proxy input events
    const handleMouseMove = (e: MouseEvent) => {
      const rect = canvas.getBoundingClientRect();
      worker.postMessage({
        type: "event",
        payload: {
          event_type: "mousemove",
          x: e.clientX - rect.left,
          y: e.clientY - rect.top,
        },
      });
    };

    const handleMouseDown = (e: MouseEvent) => {
      const rect = canvas.getBoundingClientRect();
      worker.postMessage({
        type: "event",
        payload: {
          event_type: "mousedown",
          x: e.clientX - rect.left,
          y: e.clientY - rect.top,
          button: e.button,
        },
      });
    };

    const handleMouseUp = (e: MouseEvent) => {
      const rect = canvas.getBoundingClientRect();
      worker.postMessage({
        type: "event",
        payload: {
          event_type: "mouseup",
          x: e.clientX - rect.left,
          y: e.clientY - rect.top,
          button: e.button,
        },
      });
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      worker.postMessage({
        type: "event",
        payload: {
          event_type: "keydown",
          key: e.code,
        },
      });
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      worker.postMessage({
        type: "event",
        payload: {
          event_type: "keyup",
          key: e.code,
        },
      });
    };

    canvas.addEventListener("mousemove", handleMouseMove);
    canvas.addEventListener("mousedown", handleMouseDown);
    canvas.addEventListener("mouseup", handleMouseUp);
    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("keyup", handleKeyUp);

    return () => {
      canvas.removeEventListener("mousemove", handleMouseMove);
      canvas.removeEventListener("mousedown", handleMouseDown);
      canvas.removeEventListener("mouseup", handleMouseUp);
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("keyup", handleKeyUp);
      worker.terminate();
    };
  }, []);

  return (
    <div style={{ width: "100vw", height: "100vh", margin: 0, padding: 0 }}>
      <canvas
        ref={canvasRef}
        id="bevy-canvas"
        width={1920}
        height={1080}
        style={{
          width: "100%",
          height: "100%",
          display: "block",
        }}
      />
    </div>
  );
}

export default App;
