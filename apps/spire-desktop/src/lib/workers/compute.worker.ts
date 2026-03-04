/**
 * SPIRE — Compute Worker
 *
 * Web Worker script that receives ComputeTask messages from the
 * GridManager, invokes the Tauri backend's `run_analysis` IPC command,
 * and posts back ComputeResult or error messages.
 *
 * ## Design Rationale
 *
 * In the Tauri context, the actual computation runs in the Rust backend
 * via IPC — the worker's role is to provide **concurrency isolation**
 * so that multiple IPC calls can proceed in parallel without blocking
 * the main thread's UI event loop.  For a future pure-WASM deployment,
 * the `executeTask` function can be swapped to call the WASM module
 * directly (spire-bindings `runAnalysis`).
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
 * Execute a single analysis chunk by invoking the Tauri backend.
 *
 * In the Tauri context we use `@tauri-apps/api/tauri` invoke().
 * The import is dynamic so that this file can also be loaded in
 * environments where Tauri is not available (e.g., unit tests
 * with a mock, or future pure-WASM deployment).
 */
async function executeTask(task: ComputeTask): Promise<AnalysisResult> {
  // Dynamic import — resolved at runtime within the Tauri webview.
  const { invoke } = await import("@tauri-apps/api/tauri");
  return invoke<AnalysisResult>("run_analysis", { config: task.config });
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
