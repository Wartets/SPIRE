/**
 * SPIRE - Distributed Compute Grid Types
 *
 * Type definitions for the message protocol between the GridManager
 * (main thread) and ComputeNode workers.  The protocol is designed
 * around an abstract ComputeNode interface so that today's local
 * Web Workers can be replaced tomorrow by remote nodes (WebRTC,
 * WebSocket, or cloud endpoints) without touching the GridManager.
 *
 * ## Protocol Flow
 *
 *   Main ──[ComputeTask]──▸ Worker
 *   Main ◂──[ComputeProgress]── Worker   (0..N progress updates)
 *   Main ◂──[ComputeResult]──── Worker   (final merged chunk result)
 */

import type { AnalysisConfig, AnalysisResult, HistogramData } from "./spire";

// ---------------------------------------------------------------------------
// Node Identity & Status
// ---------------------------------------------------------------------------

/** Unique identifier for a compute node within the grid. */
export type NodeId = string;

/** Lifecycle state of a compute node. */
export type NodeStatus = "idle" | "computing" | "merging" | "error" | "terminated";

/** Snapshot of a single compute node's state for the dashboard. */
export interface NodeSnapshot {
  /** Unique node identifier. */
  id: NodeId;
  /** Current lifecycle state. */
  status: NodeStatus;
  /** Events completed by this node so far. */
  eventsCompleted: number;
  /** Events assigned to this node. */
  eventsAssigned: number;
  /** Number of task retries consumed. */
  retries: number;
  /** Wall-clock time in ms since the node started its current task. */
  elapsedMs: number;
}

// ---------------------------------------------------------------------------
// Task & Result Messages
// ---------------------------------------------------------------------------

/** A unit of work dispatched to a compute node. */
export interface ComputeTask {
  /** Unique task identifier. */
  taskId: string;
  /** The analysis configuration for this chunk. */
  config: AnalysisConfig;
  /** Zero-based chunk index within the grid job. */
  chunkIndex: number;
  /** Total number of chunks in the grid job. */
  totalChunks: number;
}

/** Final result from a single compute node for one task chunk. */
export interface ComputeResult {
  /** Task identifier (matches the dispatched ComputeTask). */
  taskId: string;
  /** The analysis result for this chunk. */
  result: AnalysisResult;
  /** Wall-clock time in ms for this chunk. */
  wallTimeMs: number;
  /** Zero-based chunk index. */
  chunkIndex: number;
}

/** Intermediate progress update from a compute node. */
export interface ComputeProgress {
  /** Task identifier. */
  taskId: string;
  /** Events completed so far in this chunk. */
  eventsCompleted: number;
  /** Total events assigned to this chunk. */
  eventsTotal: number;
  /** Elapsed wall time in ms. */
  elapsedMs: number;
}

// ---------------------------------------------------------------------------
// Worker Message Protocol (postMessage envelope)
// ---------------------------------------------------------------------------

/**
 * Messages sent FROM the main thread TO a worker.
 *
 * Tagged union: discriminate on `type`.
 */
export type WorkerInbound =
  | { type: "execute"; task: ComputeTask }
  | { type: "abort" }
  | { type: "ping" };

/**
 * Messages sent FROM a worker TO the main thread.
 *
 * Tagged union: discriminate on `type`.
 */
export type WorkerOutbound =
  | { type: "result"; data: ComputeResult }
  | { type: "progress"; data: ComputeProgress }
  | { type: "error"; taskId: string; message: string }
  | { type: "pong" };

// ---------------------------------------------------------------------------
// Grid Job Configuration
// ---------------------------------------------------------------------------

/** Configuration for a distributed grid job. */
export interface GridJobConfig {
  /** The analysis configuration (total events, energy, etc.). */
  analysisConfig: AnalysisConfig;
  /** Maximum number of worker nodes to spawn. Defaults to navigator.hardwareConcurrency - 1. */
  maxWorkers?: number;
  /** Maximum events per chunk. Defaults to 100,000. */
  maxChunkSize?: number;
  /** Per-task timeout in milliseconds. Defaults to 30,000 ms. */
  taskTimeoutMs?: number;
  /** Maximum retries per failed task. Defaults to 3. */
  maxRetries?: number;
}

// ---------------------------------------------------------------------------
// Grid Job State (for the dashboard)
// ---------------------------------------------------------------------------

/** Overall status of a grid job. */
export type GridJobStatus = "idle" | "running" | "merging" | "complete" | "error" | "cancelled";

/** A snapshot of the convergence diagnostics at a point in time. */
export interface ConvergencePoint {
  /** Total events merged so far across all completed chunks. */
  eventsMerged: number;
  /** Running cross-section estimate (GeV⁻²). */
  crossSection: number;
  /** Running cross-section statistical error (GeV⁻²). */
  crossSectionError: number;
  /** Wall-clock timestamp (ms since grid job start). */
  timestampMs: number;
}

/** Complete snapshot of a running (or completed) grid job. */
export interface GridJobSnapshot {
  /** Overall job status. */
  status: GridJobStatus;
  /** Per-node state snapshots. */
  nodes: NodeSnapshot[];
  /** Total events requested. */
  totalEvents: number;
  /** Total events completed across all chunks. */
  eventsCompleted: number;
  /** Number of chunks completed. */
  chunksCompleted: number;
  /** Total number of chunks. */
  chunksTotal: number;
  /** Convergence history for the dashboard plot. */
  convergence: ConvergencePoint[];
  /** Wall-clock time since job start (ms). */
  elapsedMs: number;
  /** Estimated time remaining (ms), or null if unavailable. */
  estimatedRemainingMs: number | null;
  /** Merged result so far (null until at least one chunk completes). */
  mergedResult: AnalysisResult | null;
  /** Error message if status is "error". */
  errorMessage?: string;
}

// ---------------------------------------------------------------------------
// Abstract Compute Node Interface
// ---------------------------------------------------------------------------

/**
 * Abstract interface for a compute node.
 *
 * Today's implementation is `LocalWorkerNode` (Web Worker).
 * Tomorrow this could be `RemoteWorkerNode` (WebRTC/WebSocket)
 * or `CloudWorkerNode` (HTTP endpoint).
 *
 * The GridManager only interacts with nodes through this interface,
 * ensuring that the scheduling and merging logic is transport-agnostic.
 */
export interface ComputeNode {
  /** Unique node identifier. */
  readonly id: NodeId;
  /** Current status. */
  readonly status: NodeStatus;
  /** Dispatch a task to this node. */
  execute(task: ComputeTask): void;
  /** Abort the current task. */
  abort(): void;
  /** Register a callback for results. */
  onResult(cb: (result: ComputeResult) => void): void;
  /** Register a callback for progress updates. */
  onProgress(cb: (progress: ComputeProgress) => void): void;
  /** Register a callback for errors. */
  onError(cb: (taskId: string, message: string) => void): void;
  /** Terminate this node and release resources. */
  terminate(): void;
}
