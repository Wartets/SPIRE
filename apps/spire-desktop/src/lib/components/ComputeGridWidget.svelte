<!--
  SPIRE - Compute Grid Dashboard Widget

  Real-time monitoring and control interface for the distributed Monte
  Carlo compute grid.  Displays:

  1. **Node Status Panel** - Per-worker status indicators (Idle / Computing /
     Merging / Error) with retry counts and progress bars.
  2. **Global Progress Bar** - Events completed, chunks merged, ETA.
  3. **Convergence Plot** - Cross-section uncertainty (δσ) vs √N,
     rendered with Chart.js, showing the expected 1/√N convergence.
  4. **Results Summary** - Merged cross-section, histograms, and
     per-chunk wall-clock times.

  The widget subscribes reactively to the GridManager's Svelte store
  and updates in real time as chunks complete.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { appendLog } from "$lib/stores/physicsStore";
  import Icon from "$lib/components/ui/Icon.svelte";
  import SpireNumberInput from "$lib/components/ui/SpireNumberInput.svelte";
  import { gridManager, gridSnapshot } from "$lib/services/ComputeGrid";
  import type { GridJobSnapshot, ConvergencePoint } from "$lib/types/compute";
  import type { AnalysisResult } from "$lib/types/spire";
  import { publishWidgetInterop, widgetInteropState } from "$lib/stores/widgetInteropStore";
  import {
    Chart,
    LineController,
    LineElement,
    PointElement,
    CategoryScale,
    LinearScale,
    LogarithmicScale,
    Tooltip,
    Title,
    Legend,
    Filler,
  } from "chart.js";

  const logComputeGrid = (message: string): void => {
    appendLog(message, { category: "ComputeGrid" });
  };

  // Register Chart.js components.
  Chart.register(
    LineController,
    LineElement,
    PointElement,
    CategoryScale,
    LinearScale,
    LogarithmicScale,
    Tooltip,
    Title,
    Legend,
    Filler,
  );

  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------

  let cmsEnergy: number = 100;
  let numEvents: number = 50000;
  let nFinalState: number = 2;
  let maxWorkers: number = Math.max(
    1,
    (typeof navigator !== "undefined" ? navigator.hardwareConcurrency : 4) - 1,
  );
  let maxChunkSize: number = 100000;
  let taskTimeoutMs: number = 30000;
  let maxRetries: number = 3;

  let snapshot: GridJobSnapshot;
  let convergenceCanvasEl: HTMLCanvasElement;
  let convergenceChart: Chart | null = null;

  let errorMsg: string = "";
  let interopUnsub: (() => void) | null = null;

  function cssVar(name: string, fallback: string): string {
    if (typeof window === "undefined") return fallback;
    const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    return value || fallback;
  }

  // Subscribe to grid snapshot updates.
  const unsub = gridSnapshot.subscribe((snap) => {
    snapshot = snap;
    if (snap.convergence.length > 0) {
      updateConvergenceChart(snap.convergence);
    }
  });

  // ---------------------------------------------------------------------------
  // Computed
  // ---------------------------------------------------------------------------

  $: isRunning = snapshot?.status === "running" || snapshot?.status === "merging";
  $: isComplete = snapshot?.status === "complete";
  $: progressPercent = snapshot?.totalEvents > 0
    ? Math.min(100, Math.round((snapshot.eventsCompleted / snapshot.totalEvents) * 100))
    : 0;
  $: etaString = formatEta(snapshot?.estimatedRemainingMs ?? null);
  $: elapsedString = formatMs(snapshot?.elapsedMs ?? 0);

  // Cross-section in picobarns for display.
  $: mergedSigmaPb = snapshot?.mergedResult
    ? snapshot.mergedResult.cross_section * 0.3894e9
    : null;
  $: mergedSigmaErrPb = snapshot?.mergedResult
    ? snapshot.mergedResult.cross_section_error * 0.3894e9
    : null;

  // ---------------------------------------------------------------------------
  // Actions
  // ---------------------------------------------------------------------------

  async function handleLaunch(): Promise<void> {
    errorMsg = "";

    if (numEvents < 1) {
      errorMsg = "Number of events must be at least 1.";
      return;
    }
    if (cmsEnergy <= 0) {
      errorMsg = "CMS energy must be positive.";
      return;
    }

    logComputeGrid(
      `Compute Grid: launching ${numEvents.toLocaleString()} events across ≤${maxWorkers} workers ` +
        `at √s = ${cmsEnergy} GeV`,
    );

    try {
      const result = await gridManager.run({
        analysisConfig: {
          plots: [
            {
              name: "Leading pT (Grid)",
              observable_script: "event.momenta[0].pt()",
              n_bins: 30,
              min: 0,
              max: cmsEnergy / 2,
            },
          ],
          cut_scripts: [],
          num_events: numEvents,
          cms_energy: cmsEnergy,
          final_masses: Array(nFinalState).fill(0.0),
        },
        maxWorkers,
        maxChunkSize,
        taskTimeoutMs,
        maxRetries,
      });

      const xsecPb = result.cross_section * 0.3894e9;
      const errPb = result.cross_section_error * 0.3894e9;
      logComputeGrid(
        `Compute Grid: COMPLETE - σ ≈ ${xsecPb.toExponential(4)} ± ${errPb.toExponential(2)} pb ` +
          `(${result.events_generated.toLocaleString()} events, ` +
          `${result.events_passed.toLocaleString()} passed)`,
      );
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : String(err);
      logComputeGrid(`Compute Grid: ERROR - ${errorMsg}`);
    }
  }

  function handleCancel(): void {
    gridManager.cancel();
    logComputeGrid("Compute Grid: job cancelled by user.");
  }

  // ---------------------------------------------------------------------------
  // Convergence Chart
  // ---------------------------------------------------------------------------

  function updateConvergenceChart(points: ConvergencePoint[]): void {
    if (!convergenceCanvasEl) return;

    const labels = points.map((p) =>
      Math.sqrt(p.eventsMerged).toFixed(0),
    );
    const errorsGeV = points.map((p) => p.crossSectionError);
    const errorsPb = errorsGeV.map((e) => e * 0.3894e9);

    if (convergenceChart) {
      convergenceChart.data.labels = labels;
      convergenceChart.data.datasets[0].data = errorsPb;
      convergenceChart.update("none");
      return;
    }

    const accent = cssVar("--color-accent", "#5eb8ff");
    const accentRgb = cssVar("--color-accent-rgb", "94, 184, 255");
    const muted = cssVar("--color-text-muted", "#8a8a8a");
    const textPrimary = cssVar("--color-text-primary", "#e8e8e8");
    const textPrimaryRgb = cssVar("--color-text-primary-rgb", "232, 232, 232");

    convergenceChart = new Chart(convergenceCanvasEl, {
      type: "line",
      data: {
        labels,
        datasets: [
          {
            label: "δσ (pb)",
            data: errorsPb,
            borderColor: accent,
            backgroundColor: `rgba(${accentRgb}, 0.1)`,
            borderWidth: 2,
            pointRadius: 3,
            pointBackgroundColor: accent,
            fill: true,
            tension: 0.3,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          title: {
            display: true,
            text: "Convergence: δσ vs √N",
            color: textPrimary,
            font: { size: 13 },
          },
          legend: { display: false },
          tooltip: {
            callbacks: {
              title: (items) => `√N = ${items[0].label}`,
              label: (item) => `δσ = ${(item.raw as number).toExponential(3)} pb`,
            },
          },
        },
        scales: {
          x: {
            title: { display: true, text: "√N (events)", color: muted },
            ticks: { color: muted, maxTicksLimit: 8 },
            grid: { color: `rgba(${textPrimaryRgb}, 0.05)` },
          },
          y: {
            type: "logarithmic",
            title: { display: true, text: "δσ (pb)", color: muted },
            ticks: { color: muted },
            grid: { color: `rgba(${textPrimaryRgb}, 0.08)` },
          },
        },
      },
    });
  }

  // ---------------------------------------------------------------------------
  // Formatting Helpers
  // ---------------------------------------------------------------------------

  function formatMs(ms: number): string {
    if (ms < 1000) return `${Math.round(ms)} ms`;
    const s = ms / 1000;
    if (s < 60) return `${s.toFixed(1)} s`;
    const m = Math.floor(s / 60);
    const rem = s % 60;
    return `${m}m ${rem.toFixed(0)}s`;
  }

  function formatEta(ms: number | null): string {
    if (ms === null) return "-";
    return formatMs(ms);
  }

  function nodeStatusColor(status: string): string {
    switch (status) {
      case "idle": return cssVar("--color-success", "#2ecc71");
      case "computing": return cssVar("--color-accent", "#5eb8ff");
      case "merging": return cssVar("--hl-value", "#d4a017");
      case "error": return cssVar("--color-error", "#e74c3c");
      case "terminated": return cssVar("--color-text-muted", "#8a8a8a");
      default: return cssVar("--color-text-muted", "#8a8a8a");
    }
  }

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------

  onMount(() => {
    interopUnsub = widgetInteropState.subscribe((state) => {
      if (isRunning) return;

      const reactionPayload = state.reaction?.payload as
        | { cmsEnergy?: number; finalState?: string[] }
        | undefined;
      if (reactionPayload) {
        if (typeof reactionPayload.cmsEnergy === "number" && Number.isFinite(reactionPayload.cmsEnergy)) {
          cmsEnergy = reactionPayload.cmsEnergy;
        }
        if (Array.isArray(reactionPayload.finalState) && reactionPayload.finalState.length > 0) {
          nFinalState = reactionPayload.finalState.length;
        }
      }

      const analysisPayload = state.analysis?.payload as
        | { cmsEnergy?: number; numEvents?: number }
        | undefined;
      if (analysisPayload) {
        if (typeof analysisPayload.cmsEnergy === "number" && Number.isFinite(analysisPayload.cmsEnergy)) {
          cmsEnergy = analysisPayload.cmsEnergy;
        }
        if (typeof analysisPayload.numEvents === "number" && Number.isFinite(analysisPayload.numEvents)) {
          numEvents = Math.max(1, Math.floor(analysisPayload.numEvents));
        }
      }
    });
  });

  onDestroy(() => {
    interopUnsub?.();
    unsub();
    if (convergenceChart) {
      convergenceChart.destroy();
      convergenceChart = null;
    }
  });

  $: publishWidgetInterop("compute_grid", {
    status: snapshot?.status ?? "idle",
    cmsEnergy,
    numEvents,
    nFinalState,
    progressPercent,
    eventsCompleted: snapshot?.eventsCompleted ?? 0,
    eventsTotal: snapshot?.totalEvents ?? 0,
  });
</script>

<!-- ======================================================================= -->
<!-- Template                                                                 -->
<!-- ======================================================================= -->

<div class="grid-widget">
  <!-- ── Configuration Panel ──────────────────────────────────────────── -->
  <div class="config-panel">
    <h3 class="section-title">Compute Grid</h3>

    <div class="field-row">
      <div class="field compact">
        <label for="grid-cms">√s (GeV):</label>
        <SpireNumberInput inputId="grid-cms" bind:value={cmsEnergy} min={0.1} step={0.1} disabled={isRunning} ariaLabel="Grid CMS energy" />
      </div>
      <div class="field compact">
        <label for="grid-events">Total Events:</label>
        <SpireNumberInput inputId="grid-events" bind:value={numEvents} min={1} max={10000000} step={100} disabled={isRunning} ariaLabel="Total events" />
      </div>
      <div class="field compact">
        <label for="grid-final">Final N:</label>
        <SpireNumberInput inputId="grid-final" bind:value={nFinalState} min={2} max={8} step={1} disabled={isRunning} ariaLabel="Final-state multiplicity" />
      </div>
    </div>

    <div class="field-row">
      <div class="field compact">
        <label for="grid-workers">Max Workers:</label>
        <SpireNumberInput inputId="grid-workers" bind:value={maxWorkers} min={1} max={32} step={1} disabled={isRunning} ariaLabel="Max workers" />
      </div>
      <div class="field compact">
        <label for="grid-chunk">Chunk Size:</label>
        <SpireNumberInput inputId="grid-chunk" bind:value={maxChunkSize} min={100} max={1000000} step={100} disabled={isRunning} ariaLabel="Chunk size" />
      </div>
      <div class="field compact">
        <label for="grid-timeout">Timeout (ms):</label>
        <SpireNumberInput inputId="grid-timeout" bind:value={taskTimeoutMs} min={1000} max={300000} step={1000} disabled={isRunning} ariaLabel="Task timeout" />
      </div>
      <div class="field compact">
        <label for="grid-retries">Max Retries:</label>
        <SpireNumberInput inputId="grid-retries" bind:value={maxRetries} min={0} max={10} step={1} disabled={isRunning} ariaLabel="Max retries" />
      </div>
    </div>

    <!-- Action Buttons -->
    <div class="action-row">
      {#if isRunning}
        <button class="cancel-btn" on:click={handleCancel}><Icon name="stop" size={14} /> <span>Cancel</span></button>
      {:else}
        <button class="launch-btn" on:click={handleLaunch} disabled={isComplete && snapshot?.status === "complete"}>
          <Icon name="play" size={14} /> <span>Launch Grid</span>
        </button>
      {/if}
      {#if isComplete}
        <span class="complete-badge"><Icon name="check" size={13} /> <span>Complete</span></span>
      {/if}
    </div>

    {#if errorMsg}
      <div class="error-box">{errorMsg}</div>
    {/if}
  </div>

  <!-- ── Progress Panel ───────────────────────────────────────────────── -->
  {#if snapshot && snapshot.status !== "idle"}
    <div class="progress-panel">
      <div class="progress-header">
        <span class="progress-label">
          {snapshot.eventsCompleted.toLocaleString()} / {snapshot.totalEvents.toLocaleString()} events
          ({snapshot.chunksCompleted} / {snapshot.chunksTotal} chunks)
        </span>
        <span class="progress-eta">
          {elapsedString} elapsed · ETA: {etaString}
        </span>
      </div>
      <div class="progress-bar-track">
        <div
          class="progress-bar-fill"
          class:complete={isComplete}
          style="width: {progressPercent}%"
        ></div>
      </div>
    </div>
  {/if}

  <!-- ── Node Status Grid ─────────────────────────────────────────────── -->
  {#if snapshot && snapshot.nodes.length > 0}
    <div class="nodes-panel">
      <h4 class="sub-title">Worker Nodes</h4>
      <div class="node-grid">
        {#each snapshot.nodes as node}
          <div class="node-card">
            <div class="node-indicator" style="background: {nodeStatusColor(node.status)}"></div>
            <div class="node-info">
              <span class="node-id">{node.id}</span>
              <span class="node-status">{node.status}</span>
              {#if node.retries > 0}
                <span class="node-retries">r{node.retries}</span>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- ── Convergence Chart ────────────────────────────────────────────── -->
  <div class="convergence-panel">
    <canvas bind:this={convergenceCanvasEl}></canvas>
    {#if !snapshot || snapshot.convergence.length === 0}
      <div class="placeholder">
        Launch a grid job to see convergence diagnostics.
      </div>
    {/if}
  </div>

  <!-- ── Results Summary ──────────────────────────────────────────────── -->
  {#if snapshot?.mergedResult}
    <div class="results-panel">
      <h4 class="sub-title">Merged Result</h4>
      <div class="stats-row">
        <div class="stat">
          <span class="stat-label">σ (pb):</span>
          <span class="stat-value">
            {mergedSigmaPb?.toExponential(4)} ± {mergedSigmaErrPb?.toExponential(2)}
          </span>
        </div>
        <div class="stat">
          <span class="stat-label">Events:</span>
          <span class="stat-value">
            {snapshot.mergedResult.events_passed.toLocaleString()} / {snapshot.mergedResult.events_generated.toLocaleString()} passed
          </span>
        </div>
        <div class="stat">
          <span class="stat-label">Histograms:</span>
          <span class="stat-value">{snapshot.mergedResult.histograms.length}</span>
        </div>
      </div>
    </div>
  {/if}
</div>

<!-- ======================================================================= -->
<!-- Styles                                                                   -->
<!-- ======================================================================= -->

<style>
  .grid-widget {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    height: 100%;
    overflow-y: auto;
    font-size: 0.85rem;
  }

  .section-title {
    margin: 0 0 0.5rem 0;
    font-size: 1rem;
    color: var(--color-text-primary);
    font-weight: 700;
  }

  .sub-title {
    margin: 0 0 0.4rem 0;
    font-size: 0.85rem;
    color: var(--color-text-muted);
    font-weight: 600;
  }

  .config-panel {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.5rem;
    background: color-mix(in srgb, var(--color-bg-elevated) 55%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-border) 80%, transparent);
    border-radius: var(--radius-md);
  }

  .field-row {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .field.compact {
    flex: 1;
    min-width: 80px;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .field.compact label {
    color: var(--color-text-muted);
    font-size: 0.75rem;
  }

  :global(.spire-number-input):disabled {
    opacity: 0.4;
  }

  .action-row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .launch-btn {
    padding: 0.4rem 1.2rem;
    font-size: 0.85rem;
    font-weight: 600;
    border: none;
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--color-accent) 20%, var(--color-bg-surface));
    border: 1px solid color-mix(in srgb, var(--color-accent) 70%, var(--color-border));
    color: var(--color-text-primary);
    cursor: pointer;
    transition: opacity var(--transition-fast), background-color var(--transition-fast);
  }

  .launch-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .launch-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .cancel-btn {
    padding: 0.4rem 1.2rem;
    font-size: 0.85rem;
    font-weight: 600;
    border: none;
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--color-error) 20%, var(--color-bg-surface));
    border: 1px solid color-mix(in srgb, var(--color-error) 70%, var(--color-border));
    color: var(--color-text-primary);
    cursor: pointer;
  }

  .complete-badge {
    color: var(--color-success);
    font-weight: 600;
    font-size: 0.85rem;
  }

  .error-box {
    padding: 0.4rem 0.6rem;
    background: rgba(var(--color-error-rgb), 0.12);
    border: 1px solid rgba(var(--color-error-rgb), 0.3);
    border-radius: var(--radius-sm);
    color: var(--color-error);
    font-size: 0.8rem;
    word-break: break-word;
  }

  /* ── Progress Bar ── */

  .progress-panel {
    padding: 0.5rem;
    background: color-mix(in srgb, var(--color-bg-elevated) 55%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-border) 80%, transparent);
    border-radius: var(--radius-md);
  }

  .progress-header {
    display: flex;
    justify-content: space-between;
    margin-bottom: 0.3rem;
  }

  .progress-label {
    font-size: 0.78rem;
    color: var(--color-text-primary);
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  .progress-eta {
    font-size: 0.75rem;
    color: var(--color-text-muted);
  }

  .progress-bar-track {
    width: 100%;
    height: 8px;
    background: color-mix(in srgb, var(--color-text-primary) 10%, transparent);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .progress-bar-fill {
    height: 100%;
    background: color-mix(in srgb, var(--color-accent) 70%, var(--color-bg-surface));
    border-radius: var(--radius-sm);
    transition: width 0.3s ease;
  }

  .progress-bar-fill.complete {
    background: color-mix(in srgb, var(--color-success) 70%, var(--color-bg-surface));
  }

  /* ── Node Status Grid ── */

  .nodes-panel {
    padding: 0.5rem;
    background: color-mix(in srgb, var(--color-bg-elevated) 55%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-border) 80%, transparent);
    border-radius: var(--radius-md);
  }

  .node-grid {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .node-card {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.25rem 0.5rem;
    background: color-mix(in srgb, var(--color-bg-base) 75%, transparent);
    border-radius: var(--radius-sm);
    border: 1px solid color-mix(in srgb, var(--color-border) 40%, transparent);
  }

  .node-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .node-info {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .node-id {
    font-size: 0.72rem;
    color: var(--color-text-muted);
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  .node-status {
    font-size: 0.72rem;
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .node-retries {
    font-size: 0.68rem;
    color: var(--hl-value);
    font-weight: 600;
  }

  /* ── Convergence Chart ── */

  .convergence-panel {
    position: relative;
    min-height: 200px;
    flex: 1;
    padding: 0.5rem;
    background: color-mix(in srgb, var(--color-bg-elevated) 55%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-border) 80%, transparent);
    border-radius: var(--radius-md);
  }

  .convergence-panel canvas {
    width: 100% !important;
    height: 100% !important;
  }

  .placeholder {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-text-muted);
    font-size: 0.85rem;
    text-align: center;
    padding: 1rem;
  }

  /* ── Results Summary ── */

  .results-panel {
    padding: 0.5rem;
    background: color-mix(in srgb, var(--color-bg-elevated) 55%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-border) 80%, transparent);
    border-radius: var(--radius-md);
  }

  .stats-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1.5rem;
  }

  .stat {
    display: flex;
    gap: 0.3rem;
  }

  .stat-label {
    color: var(--color-text-muted);
    font-size: 0.78rem;
  }

  .stat-value {
    color: var(--color-text-primary);
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 0.78rem;
  }
</style>
