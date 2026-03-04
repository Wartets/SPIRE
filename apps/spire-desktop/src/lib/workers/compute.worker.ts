/**
 * SPIRE — Compute Worker
 *
 * Web Worker script that receives ComputeTask messages from the
 * GridManager, invokes the backend's `run_analysis` command,
 * and posts back ComputeResult or error messages.
 *
 * ## Design Rationale
 *
 * The worker uses an environment-aware backend resolution strategy:
 *
 * 1. Attempt to load Tauri IPC (`@tauri-apps/api/tauri`). If available,
 *    analysis runs in the Rust backend process via IPC.
 * 2. If Tauri is not available (standard browser), attempt to load the
 *    WASM kernel module for in-browser computation.
 * 3. If neither is available, return an error indicating no compute
 *    backend is reachable from this worker context.
 *
 * ## Message Protocol
 *
 * Inbound (main → worker):
 *   { type: "execute", task: ComputeTask }
 *   { type: "abort" }
 *   { type: "ping" }
 *
 * Outbound (worker → main):
 *   { type: "result", data: ComputeResult }
 *   { type: "progress", data: ComputeProgress }
 *   { type: "error", taskId: string, message: string }
 *   { type: "pong" }
 */

import type {
  WorkerInbound,
  WorkerOutbound,
  ComputeTask,
  ComputeResult,
  ComputeProgress,
} from "$lib/types/compute";
import type { AnalysisResult } from "$lib/types/spire";

// ---------------------------------------------------------------------------
// Abort Controller
// ---------------------------------------------------------------------------

let currentTaskId: string | null = null;
let aborted = false;

// ---------------------------------------------------------------------------
// Task Execution
// ---------------------------------------------------------------------------

/**
 * Execute a single analysis chunk using the best available backend.
 *
 * Resolution order:
 *   1. Tauri IPC (if running inside the Tauri webview)
 *   2. WASM kernel (if WebAssembly is available)
 *   3. Error — no backend reachable from worker context
 */
async function executeTask(task: ComputeTask): Promise<AnalysisResult> {
  // --- Attempt 1: Tauri IPC ---
  try {
    const { invoke } = await import("@tauri-apps/api/tauri");
    return await invoke<AnalysisResult>("run_analysis", { config: task.config });
  } catch {
    // Tauri not available — fall through to WASM.
  }

  // --- Attempt 2: WASM kernel ---
  try {
    // @ts-expect-error — spire-kernel-wasm is an optional dependency built separately
    const wasm = await import(/* @vite-ignore */ "spire-kernel-wasm");
    if (typeof wasm.default === "function") {
      await wasm.default(); // wasm-bindgen init
    }
    const fn = (wasm as Record<string, unknown>)["runAnalysis"];
    if (typeof fn === "function") {
      const resultJson = fn(JSON.stringify({ config: task.config }));
      const resolved = resultJson instanceof Promise ? await resultJson : resultJson;
      if (typeof resolved === "string") return JSON.parse(resolved) as AnalysisResult;
      return resolved as AnalysisResult;
    }
  } catch {
    // WASM not available either — fall through.
  }

  throw new Error(
    "No computation backend available in this worker context. " +
    "Neither Tauri IPC nor WASM kernel could be loaded.",
  );
}

// ---------------------------------------------------------------------------
// Message Handler
// ---------------------------------------------------------------------------

self.onmessage = async (event: MessageEvent<WorkerInbound>) => {
  const msg = event.data;

  switch (msg.type) {
    case "ping": {
      const reply: WorkerOutbound = { type: "pong" };
      self.postMessage(reply);
      break;
    }

    case "abort": {
      aborted = true;
      currentTaskId = null;
      break;
    }

    case "execute": {
      const task = msg.task;
      currentTaskId = task.taskId;
      aborted = false;

      const startTime = performance.now();

      // Post initial progress (0%).
      const initialProgress: WorkerOutbound = {
        type: "progress",
        data: {
          taskId: task.taskId,
          eventsCompleted: 0,
          eventsTotal: task.config.num_events,
          elapsedMs: 0,
        } satisfies ComputeProgress,
      };
      self.postMessage(initialProgress);

      try {
        const result = await executeTask(task);

        // Check if aborted during execution.
        if (aborted || currentTaskId !== task.taskId) {
          return;
        }

        const wallTimeMs = performance.now() - startTime;

        const resultMsg: WorkerOutbound = {
          type: "result",
          data: {
            taskId: task.taskId,
            result,
            wallTimeMs,
            chunkIndex: task.chunkIndex,
          } satisfies ComputeResult,
        };
        self.postMessage(resultMsg);
      } catch (err) {
        if (aborted || currentTaskId !== task.taskId) {
          return;
        }

        const errorMsg: WorkerOutbound = {
          type: "error",
          taskId: task.taskId,
          message: err instanceof Error ? err.message : String(err),
        };
        self.postMessage(errorMsg);
      } finally {
        if (currentTaskId === task.taskId) {
          currentTaskId = null;
        }
      }
      break;
    }
  }
};
