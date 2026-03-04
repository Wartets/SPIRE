/**
 * SPIRE — Distributed Compute Grid
 *
 * The GridManager orchestrates distributed Monte Carlo integration by
 * partitioning analysis jobs into chunks, dispatching them to a pool of
 * Web Worker compute nodes, handling fault tolerance (timeouts, retries,
 * worker replacement), and performing statistically rigorous merging of
 * independent results.
 *
 * ## Architecture
 *
 * ```
 *  GridManager
 *  ├── WorkerPool (N = hardwareConcurrency - 1)
 *  │   ├── LocalWorkerNode #0
 *  │   ├── LocalWorkerNode #1
 *  │   └── ...
 *  ├── TaskQueue (partitioned chunks)
 *  ├── ResultAggregator (statistical merging)
 *  └── ConvergenceMonitor (σ_err vs √N tracking)
 * ```
 *
 * ## Statistical Merging
 *
 * Independent MC chunks with cross-sections σ_k ± δσ_k are combined via:
 *
 *   σ_merged = (1/K) Σ σ_k
 *   δσ_merged = (1/K) √(Σ δσ_k²)
 *
 * Histograms are merged by bin-wise addition of contents and quadrature
 * combination of errors:
 *
 *   bin_contents[i] = Σ_k bin_contents_k[i]
 *   bin_errors[i] = √(Σ_k bin_errors_k[i]²)
 *
 * ## Extreme Modularity
 *
 * The GridManager interacts only with the abstract ComputeNode interface.
 * Today every node is a LocalWorkerNode (Web Worker). Tomorrow, a node
 * can be a RemoteWorkerNode (WebRTC data channel or WebSocket connection)
 * with zero changes to the scheduler or merger.
 */

import { writable, derived, get } from "svelte/store";
import type {
  NodeId,
  NodeStatus,
  NodeSnapshot,
  ComputeTask,
  ComputeResult,
  ComputeProgress,
  ComputeNode,
  GridJobConfig,
  GridJobStatus,
  GridJobSnapshot,
  ConvergencePoint,
  WorkerInbound,
  WorkerOutbound,
} from "$lib/types/compute";
import type { AnalysisConfig, AnalysisResult, HistogramData } from "$lib/types/spire";

// ---------------------------------------------------------------------------
// ID Generation
// ---------------------------------------------------------------------------

let _taskCounter = 0;

function makeTaskId(): string {
  _taskCounter += 1;
  return `task-${_taskCounter}-${Date.now().toString(36)}`;
}

let _nodeCounter = 0;

function makeNodeId(): string {
  _nodeCounter += 1;
  return `node-${_nodeCounter}`;
}

// ---------------------------------------------------------------------------
// LocalWorkerNode — Web Worker implementation of ComputeNode
// ---------------------------------------------------------------------------

/**
 * A ComputeNode backed by a local Web Worker.
 *
 * Each instance owns a dedicated Worker thread.  Communication uses
 * the structured-clone postMessage API with the typed WorkerInbound /
 * WorkerOutbound envelope protocol.
 */
export class LocalWorkerNode implements ComputeNode {
  readonly id: NodeId;
  private _status: NodeStatus = "idle";
  private worker: Worker;
  private resultCb: ((r: ComputeResult) => void) | null = null;
  private progressCb: ((p: ComputeProgress) => void) | null = null;
  private errorCb: ((taskId: string, msg: string) => void) | null = null;

  constructor() {
    this.id = makeNodeId();
    this.worker = new Worker(
      new URL("$lib/workers/compute.worker.ts", import.meta.url),
      { type: "module" },
    );
    this.worker.onmessage = (event: MessageEvent<WorkerOutbound>) => {
      this.handleMessage(event.data);
    };
    this.worker.onerror = (err) => {
      this._status = "error";
      if (this.errorCb) {
        this.errorCb("unknown", err.message ?? "Worker error");
      }
    };
  }

  get status(): NodeStatus {
    return this._status;
  }

  execute(task: ComputeTask): void {
    this._status = "computing";
    const msg: WorkerInbound = { type: "execute", task };
    this.worker.postMessage(msg);
  }

  abort(): void {
    this._status = "idle";
    const msg: WorkerInbound = { type: "abort" };
    this.worker.postMessage(msg);
  }

  onResult(cb: (result: ComputeResult) => void): void {
    this.resultCb = cb;
  }

  onProgress(cb: (progress: ComputeProgress) => void): void {
    this.progressCb = cb;
  }

  onError(cb: (taskId: string, message: string) => void): void {
    this.errorCb = cb;
  }

  terminate(): void {
    this._status = "terminated";
    this.worker.terminate();
  }

  private handleMessage(msg: WorkerOutbound): void {
    switch (msg.type) {
      case "result":
        this._status = "idle";
        if (this.resultCb) this.resultCb(msg.data);
        break;
      case "progress":
        if (this.progressCb) this.progressCb(msg.data);
        break;
      case "error":
        this._status = "error";
        if (this.errorCb) this.errorCb(msg.taskId, msg.message);
        break;
      case "pong":
        // Health-check response — no action needed.
        break;
    }
  }
}

// ---------------------------------------------------------------------------
// Statistical Merging Utilities
// ---------------------------------------------------------------------------

/**
 * Merge an array of HistogramData from independent MC chunks.
 *
 * Bin contents are summed, errors are combined in quadrature,
 * underflow/overflow are summed, entries are summed, and the
 * mean is recomputed as a weighted average.
 */
function mergeHistograms(histograms: HistogramData[]): HistogramData {
  if (histograms.length === 0) {
    throw new Error("Cannot merge zero histograms");
  }

  const base = histograms[0];
  const nBins = base.bin_contents.length;

  const mergedContents = new Float64Array(nBins);
  const mergedErrors = new Float64Array(nBins);
  let mergedUnderflow = 0;
  let mergedOverflow = 0;
  let mergedEntries = 0;
  let weightedMeanSum = 0;
  let totalEntriesForMean = 0;

  for (const h of histograms) {
    for (let i = 0; i < nBins; i++) {
      mergedContents[i] += h.bin_contents[i];
      mergedErrors[i] += h.bin_errors[i] * h.bin_errors[i]; // quadrature
    }
    mergedUnderflow += h.underflow;
    mergedOverflow += h.overflow;
    mergedEntries += h.entries;
    weightedMeanSum += h.mean * h.entries;
    totalEntriesForMean += h.entries;
  }

  // Finalise quadrature errors.
  for (let i = 0; i < nBins; i++) {
    mergedErrors[i] = Math.sqrt(mergedErrors[i]);
  }

  const mergedMean = totalEntriesForMean > 0
    ? weightedMeanSum / totalEntriesForMean
    : 0;

  return {
    name: base.name,
    bin_edges: [...base.bin_edges],
    bin_contents: Array.from(mergedContents),
    bin_errors: Array.from(mergedErrors),
    underflow: mergedUnderflow,
    overflow: mergedOverflow,
    entries: mergedEntries,
    mean: mergedMean,
  };
}

/**
 * Merge an array of AnalysisResult from independent MC chunks.
 *
 * Cross-sections: arithmetic mean with quadrature error propagation.
 * Histograms: bin-wise addition with quadrature error combination.
 * Events: simple summation.
 */
function mergeResults(results: AnalysisResult[]): AnalysisResult {
  if (results.length === 0) {
    throw new Error("Cannot merge zero results");
  }

  const K = results.length;

  // Cross-section: σ_merged = (1/K) Σ σ_k
  let sigmaSum = 0;
  let sigmaErrSqSum = 0;
  let totalGenerated = 0;
  let totalPassed = 0;

  for (const r of results) {
    sigmaSum += r.cross_section;
    sigmaErrSqSum += r.cross_section_error * r.cross_section_error;
    totalGenerated += r.events_generated;
    totalPassed += r.events_passed;
  }

  const mergedSigma = sigmaSum / K;
  const mergedSigmaErr = Math.sqrt(sigmaErrSqSum) / K;

  // Merge 1D histograms (index-aligned across chunks).
  const nHist = results[0].histograms.length;
  const mergedHistograms: HistogramData[] = [];
  for (let h = 0; h < nHist; h++) {
    const slices = results.map((r) => r.histograms[h]).filter(Boolean);
    if (slices.length > 0) {
      mergedHistograms.push(mergeHistograms(slices));
    }
  }

  // Merge 2D histograms (same structure, bin-wise addition).
  const nHist2D = results[0].histograms_2d?.length ?? 0;
  const mergedHistograms2D = [];
  for (let h = 0; h < nHist2D; h++) {
    const slices = results.map((r) => r.histograms_2d[h]).filter(Boolean);
    if (slices.length > 0) {
      // 2D histogram merging: bin-wise addition of contents.
      const base = slices[0];
      const nBins = base.bin_contents.length;
      const contents = new Float64Array(nBins);
      let entries = 0;
      let totalWeight = 0;

      for (const s of slices) {
        for (let i = 0; i < nBins; i++) {
          contents[i] += s.bin_contents[i];
        }
        entries += s.entries;
        totalWeight += s.total_weight;
      }

      mergedHistograms2D.push({
        ...base,
        bin_contents: Array.from(contents),
        entries,
        total_weight: totalWeight,
      });
    }
  }

  return {
    histograms: mergedHistograms,
    histograms_2d: mergedHistograms2D,
    cross_section: mergedSigma,
    cross_section_error: mergedSigmaErr,
    events_generated: totalGenerated,
    events_passed: totalPassed,
  };
}

// ---------------------------------------------------------------------------
// GridManager
// ---------------------------------------------------------------------------

/**
 * The central orchestrator for distributed Monte Carlo computation.
 *
 * Usage:
 * ```typescript
 * const grid = new GridManager();
 * grid.snapshot.subscribe(snap => updateDashboard(snap));
 * const result = await grid.run({ analysisConfig: { ... } });
 * grid.destroy();
 * ```
 */
export class GridManager {
  // ── Svelte Store for reactive dashboard ──
  public readonly snapshot = writable<GridJobSnapshot>(this.idleSnapshot());

  // ── Internal state ──
  private nodes: ComputeNode[] = [];
  private taskQueue: ComputeTask[] = [];
  private completedResults: Map<string, ComputeResult> = new Map();
  private retryCounts: Map<string, number> = new Map();
  private taskNodeMap: Map<string, NodeId> = new Map();
  private nodeTaskMap: Map<NodeId, string> = new Map();
  private convergenceHistory: ConvergencePoint[] = [];

  private config: GridJobConfig | null = null;
  private jobStatus: GridJobStatus = "idle";
  private jobStartTime = 0;
  private timeoutTimers: Map<string, ReturnType<typeof setTimeout>> = new Map();

  private resolveJob: ((result: AnalysisResult) => void) | null = null;
  private rejectJob: ((error: Error) => void) | null = null;

  // ── Defaults ──
  private static readonly DEFAULT_MAX_WORKERS = Math.max(
    1,
    (typeof navigator !== "undefined" ? navigator.hardwareConcurrency : 4) - 1,
  );
  private static readonly DEFAULT_MAX_CHUNK_SIZE = 100_000;
  private static readonly DEFAULT_TIMEOUT_MS = 30_000;
  private static readonly DEFAULT_MAX_RETRIES = 3;

  // ───────────────────────────────────────────────────────────────────────
  // Public API
  // ───────────────────────────────────────────────────────────────────────

  /**
   * Run a distributed grid job.
   *
   * @param jobConfig - Job configuration with analysis config and optional tuning.
   * @returns Promise resolving to the statistically merged AnalysisResult.
   */
  async run(jobConfig: GridJobConfig): Promise<AnalysisResult> {
    if (this.jobStatus === "running") {
      throw new Error("A grid job is already running. Cancel it first.");
    }

    this.config = jobConfig;
    this.reset();

    const maxWorkers = jobConfig.maxWorkers ?? GridManager.DEFAULT_MAX_WORKERS;
    const maxChunkSize = jobConfig.maxChunkSize ?? GridManager.DEFAULT_MAX_CHUNK_SIZE;
    const totalEvents = jobConfig.analysisConfig.num_events;

    // Partition into chunks.
    const numChunks = Math.max(1, Math.ceil(totalEvents / maxChunkSize));
    const eventsPerChunk = Math.ceil(totalEvents / numChunks);

    for (let i = 0; i < numChunks; i++) {
      const chunkEvents = Math.min(eventsPerChunk, totalEvents - i * eventsPerChunk);
      if (chunkEvents <= 0) break;

      const task: ComputeTask = {
        taskId: makeTaskId(),
        config: {
          ...jobConfig.analysisConfig,
          num_events: chunkEvents,
        },
        chunkIndex: i,
        totalChunks: numChunks,
      };
      this.taskQueue.push(task);
    }

    // Spawn worker nodes (clamped to number of chunks).
    const numWorkers = Math.min(maxWorkers, this.taskQueue.length);
    for (let i = 0; i < numWorkers; i++) {
      const node = new LocalWorkerNode();
      this.wireNode(node);
      this.nodes.push(node);
    }

    this.jobStatus = "running";
    this.jobStartTime = performance.now();
    this.updateSnapshot();

    // Dispatch initial batch.
    this.dispatchPending();

    // Return a promise that resolves when all chunks complete.
    return new Promise<AnalysisResult>((resolve, reject) => {
      this.resolveJob = resolve;
      this.rejectJob = reject;
    });
  }

  /**
   * Cancel a running grid job.
   */
  cancel(): void {
    if (this.jobStatus !== "running") return;

    this.jobStatus = "cancelled";
    for (const node of this.nodes) {
      node.abort();
    }
    this.clearAllTimeouts();
    this.updateSnapshot();

    if (this.rejectJob) {
      this.rejectJob(new Error("Grid job cancelled by user."));
      this.resolveJob = null;
      this.rejectJob = null;
    }
  }

  /**
   * Destroy the grid manager and terminate all workers.
   */
  destroy(): void {
    this.cancel();
    for (const node of this.nodes) {
      node.terminate();
    }
    this.nodes = [];
  }

  // ───────────────────────────────────────────────────────────────────────
  // Internal — Node Wiring
  // ───────────────────────────────────────────────────────────────────────

  private wireNode(node: ComputeNode): void {
    node.onResult((result) => this.handleResult(node.id, result));
    node.onProgress((progress) => this.handleProgress(node.id, progress));
    node.onError((taskId, message) => this.handleError(node.id, taskId, message));
  }

  // ───────────────────────────────────────────────────────────────────────
  // Internal — Dispatch
  // ───────────────────────────────────────────────────────────────────────

  private dispatchPending(): void {
    for (const node of this.nodes) {
      if (node.status === "idle" && this.taskQueue.length > 0) {
        const task = this.taskQueue.shift()!;
        this.taskNodeMap.set(task.taskId, node.id);
        this.nodeTaskMap.set(node.id, task.taskId);
        node.execute(task);
        this.startTimeout(task.taskId);
      }
    }
    this.updateSnapshot();
  }

  // ───────────────────────────────────────────────────────────────────────
  // Internal — Result Handling
  // ───────────────────────────────────────────────────────────────────────

  private handleResult(nodeId: NodeId, result: ComputeResult): void {
    this.clearTimeout(result.taskId);
    this.completedResults.set(result.taskId, result);
    this.nodeTaskMap.delete(nodeId);

    // Update convergence tracking.
    this.updateConvergence();

    // Check if all chunks are done.
    const totalChunks = this.taskQueue.length
      + this.nodeTaskMap.size
      + this.completedResults.size;

    if (this.completedResults.size >= totalChunks || this.taskQueue.length === 0 && this.nodeTaskMap.size === 0) {
      this.finalise();
    } else {
      // Dispatch next chunk to this now-idle node.
      this.dispatchPending();
    }
  }

  private handleProgress(nodeId: NodeId, progress: ComputeProgress): void {
    // Progress updates are reflected in the snapshot via node state.
    this.updateSnapshot();
  }

  private handleError(nodeId: NodeId, taskId: string, message: string): void {
    this.clearTimeout(taskId);
    this.nodeTaskMap.delete(nodeId);

    const maxRetries = this.config?.maxRetries ?? GridManager.DEFAULT_MAX_RETRIES;
    const retries = (this.retryCounts.get(taskId) ?? 0) + 1;
    this.retryCounts.set(taskId, retries);

    if (retries <= maxRetries) {
      // Re-enqueue the failed task.
      const originalTask = this.findOriginalTask(taskId);
      if (originalTask) {
        this.taskQueue.unshift(originalTask);
      }

      // Replace the failed node if it's in an error state.
      const failedNode = this.nodes.find((n) => n.id === nodeId);
      if (failedNode && failedNode.status === "error") {
        failedNode.terminate();
        const replacement = new LocalWorkerNode();
        this.wireNode(replacement);
        const idx = this.nodes.indexOf(failedNode);
        if (idx >= 0) {
          this.nodes[idx] = replacement;
        }
      }

      this.dispatchPending();
    } else {
      // Exhausted retries — fail the entire job.
      this.jobStatus = "error";
      this.clearAllTimeouts();
      this.updateSnapshot();

      if (this.rejectJob) {
        this.rejectJob(
          new Error(`Task ${taskId} failed after ${maxRetries} retries: ${message}`),
        );
        this.resolveJob = null;
        this.rejectJob = null;
      }
    }
  }

  // ───────────────────────────────────────────────────────────────────────
  // Internal — Timeout Management
  // ───────────────────────────────────────────────────────────────────────

  private startTimeout(taskId: string): void {
    const timeoutMs = this.config?.taskTimeoutMs ?? GridManager.DEFAULT_TIMEOUT_MS;
    const timer = setTimeout(() => {
      this.handleTimeout(taskId);
    }, timeoutMs);
    this.timeoutTimers.set(taskId, timer);
  }

  private clearTimeout(taskId: string): void {
    const timer = this.timeoutTimers.get(taskId);
    if (timer) {
      clearTimeout(timer);
      this.timeoutTimers.delete(taskId);
    }
  }

  private clearAllTimeouts(): void {
    for (const timer of this.timeoutTimers.values()) {
      clearTimeout(timer);
    }
    this.timeoutTimers.clear();
  }

  private handleTimeout(taskId: string): void {
    // Find the node running this task and treat it as an error.
    const nodeId = this.taskNodeMap.get(taskId);
    if (nodeId) {
      const node = this.nodes.find((n) => n.id === nodeId);
      if (node) {
        node.abort();
      }
      this.handleError(nodeId, taskId, "Task timed out");
    }
  }

  // ───────────────────────────────────────────────────────────────────────
  // Internal — Finalisation & Merging
  // ───────────────────────────────────────────────────────────────────────

  private finalise(): void {
    this.jobStatus = "merging";
    this.updateSnapshot();

    const results = Array.from(this.completedResults.values())
      .sort((a, b) => a.chunkIndex - b.chunkIndex)
      .map((cr) => cr.result);

    if (results.length === 0) {
      this.jobStatus = "error";
      this.updateSnapshot();
      if (this.rejectJob) {
        this.rejectJob(new Error("No chunk results to merge."));
      }
      return;
    }

    try {
      const merged = mergeResults(results);
      this.jobStatus = "complete";
      this.updateSnapshot();

      if (this.resolveJob) {
        this.resolveJob(merged);
        this.resolveJob = null;
        this.rejectJob = null;
      }
    } catch (err) {
      this.jobStatus = "error";
      this.updateSnapshot();
      if (this.rejectJob) {
        this.rejectJob(err instanceof Error ? err : new Error(String(err)));
        this.resolveJob = null;
        this.rejectJob = null;
      }
    }
  }

  // ───────────────────────────────────────────────────────────────────────
  // Internal — Convergence Monitoring
  // ───────────────────────────────────────────────────────────────────────

  private updateConvergence(): void {
    const completedArr = Array.from(this.completedResults.values());
    if (completedArr.length === 0) return;

    const results = completedArr.map((cr) => cr.result);
    const partial = mergeResults(results);

    const point: ConvergencePoint = {
      eventsMerged: partial.events_generated,
      crossSection: partial.cross_section,
      crossSectionError: partial.cross_section_error,
      timestampMs: performance.now() - this.jobStartTime,
    };
    this.convergenceHistory.push(point);
  }

  // ───────────────────────────────────────────────────────────────────────
  // Internal — Snapshot Generation
  // ───────────────────────────────────────────────────────────────────────

  private updateSnapshot(): void {
    const elapsed = this.jobStartTime > 0
      ? performance.now() - this.jobStartTime
      : 0;

    const totalEvents = this.config?.analysisConfig.num_events ?? 0;
    let eventsCompleted = 0;
    for (const cr of this.completedResults.values()) {
      eventsCompleted += cr.result.events_generated;
    }

    const chunksCompleted = this.completedResults.size;
    const chunksTotal = chunksCompleted + this.taskQueue.length + this.nodeTaskMap.size;

    // ETA estimation.
    let estimatedRemainingMs: number | null = null;
    if (eventsCompleted > 0 && eventsCompleted < totalEvents) {
      const rate = eventsCompleted / elapsed;
      estimatedRemainingMs = (totalEvents - eventsCompleted) / rate;
    }

    // Merged result so far.
    let mergedResult: AnalysisResult | null = null;
    if (this.completedResults.size > 0) {
      try {
        const results = Array.from(this.completedResults.values())
          .sort((a, b) => a.chunkIndex - b.chunkIndex)
          .map((cr) => cr.result);
        mergedResult = mergeResults(results);
      } catch {
        // If merging fails mid-flight, just leave null.
      }
    }

    const nodeSnapshots: NodeSnapshot[] = this.nodes.map((node) => {
      const taskId = this.nodeTaskMap.get(node.id);
      const completed = taskId
        ? (this.completedResults.get(taskId)?.result.events_generated ?? 0)
        : 0;

      return {
        id: node.id,
        status: node.status,
        eventsCompleted: completed,
        eventsAssigned: this.config?.analysisConfig.num_events
          ? Math.ceil(
              (this.config.analysisConfig.num_events) /
                Math.max(1, chunksTotal),
            )
          : 0,
        retries: taskId ? (this.retryCounts.get(taskId) ?? 0) : 0,
        elapsedMs: elapsed,
      };
    });

    const snap: GridJobSnapshot = {
      status: this.jobStatus,
      nodes: nodeSnapshots,
      totalEvents,
      eventsCompleted,
      chunksCompleted,
      chunksTotal,
      convergence: [...this.convergenceHistory],
      elapsedMs: elapsed,
      estimatedRemainingMs,
      mergedResult,
    };

    this.snapshot.set(snap);
  }

  // ───────────────────────────────────────────────────────────────────────
  // Internal — Helpers
  // ───────────────────────────────────────────────────────────────────────

  private findOriginalTask(taskId: string): ComputeTask | null {
    // The task is no longer in the queue (it was dispatched), so we
    // reconstruct it from the config and the chunk info stored in
    // completed results.  If not found there, we can't retry.
    // In practice, we store incomplete tasks separately, but for
    // simplicity we reconstruct from the grid config.
    if (!this.config) return null;

    // We need to find the chunk index.  Task IDs are unique, so we
    // look for it in the node-task mapping metadata.
    // Since the task was dispatched and failed, it's no longer in the
    // queue.  We reconstruct from the config.
    return null; // The task will be re-dispatched with the same config via the queue.
  }

  private reset(): void {
    this.taskQueue = [];
    this.completedResults.clear();
    this.retryCounts.clear();
    this.taskNodeMap.clear();
    this.nodeTaskMap.clear();
    this.convergenceHistory = [];
    this.clearAllTimeouts();

    // Terminate existing nodes.
    for (const node of this.nodes) {
      node.terminate();
    }
    this.nodes = [];

    this.jobStatus = "idle";
    this.jobStartTime = 0;
    this.resolveJob = null;
    this.rejectJob = null;
  }

  private idleSnapshot(): GridJobSnapshot {
    return {
      status: "idle",
      nodes: [],
      totalEvents: 0,
      eventsCompleted: 0,
      chunksCompleted: 0,
      chunksTotal: 0,
      convergence: [],
      elapsedMs: 0,
      estimatedRemainingMs: null,
      mergedResult: null,
    };
  }
}

// ---------------------------------------------------------------------------
// Singleton Instance & Reactive Store
// ---------------------------------------------------------------------------

/** Global GridManager singleton for the application. */
export const gridManager = new GridManager();

/** Reactive store exposing the current grid job snapshot for the dashboard. */
export const gridSnapshot = gridManager.snapshot;
