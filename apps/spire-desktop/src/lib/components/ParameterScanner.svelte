<!--
  SPIRE - Parameter Scanner Widget (Phase 44)

  Interactive 1D parameter sweep interface. The user selects a scan target
  (field mass, decay width, vertex coupling, or CMS energy), configures
  the range and number of evaluation points, and runs a parallel Monte Carlo
  scan. Results are plotted as a cross-section vs. parameter chart using
  Chart.js.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { get } from "svelte/store";
  import { theoreticalModel, activeReaction, appendLog } from "$lib/stores/physicsStore";
  import { runParameterScan1D } from "$lib/api";
  import { registerCommand, unregisterCommand } from "$lib/core/services/CommandRegistry";
  import { publishWidgetInterop, widgetInteropState } from "$lib/stores/widgetInteropStore";
  import HoverDef from "$lib/components/ui/HoverDef.svelte";
  import SpireNumberInput from "$lib/components/ui/SpireNumberInput.svelte";
  import { isolateEvents } from "$lib/actions/widgetEvents";
  import { tooltip } from "$lib/actions/tooltip";
  import { showContextMenu } from "$lib/stores/contextMenuStore";
  import type { ScanScale, ScanResult1D, TheoreticalModel } from "$lib/types/spire";
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

  const logScanner = (message: string): void => {
    appendLog(message, { category: "Scanner" });
  };

  // Register Chart.js components (tree-shakeable).
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

  // -------------------------------------------------------------------------
  // Scan Target Presets
  // -------------------------------------------------------------------------

  interface TargetPreset {
    label: string;
    target: string;
    defaultMin: number;
    defaultMax: number;
    description: string;
  }

  /** Build target presets dynamically from the loaded model. */
  function buildPresets(model: TheoreticalModel | null): TargetPreset[] {
    const presets: TargetPreset[] = [
      {
        label: "CMS Energy",
        target: "cms_energy",
        defaultMin: 10,
        defaultMax: 1000,
        description: "Centre-of-mass energy √s (GeV)",
      },
    ];

    if (model) {
      for (const field of model.fields) {
        if (field.mass > 0) {
          presets.push({
            label: `${field.name} mass`,
            target: `field.${field.id}.mass`,
            defaultMin: field.mass * 0.1,
            defaultMax: field.mass * 10,
            description: `Mass of ${field.name} (GeV)`,
          });
        }
        if (field.width > 0) {
          presets.push({
            label: `${field.name} width`,
            target: `field.${field.id}.width`,
            defaultMin: field.width * 0.1,
            defaultMax: field.width * 10,
            description: `Decay width of ${field.name} (GeV)`,
          });
        }
      }
      for (const vf of model.vertex_factors) {
        if (vf.coupling_value != null) {
          presets.push({
            label: `${vf.term_id} coupling`,
            target: `vertex.${vf.term_id}.coupling`,
            defaultMin: vf.coupling_value * 0.01,
            defaultMax: vf.coupling_value * 10,
            description: `Coupling constant for ${vf.term_id}`,
          });
        }
      }
    }

    return presets;
  }

  // -------------------------------------------------------------------------
  // State
  // -------------------------------------------------------------------------

  let presets: TargetPreset[] = [];
  let selectedPresetIdx = 0;
  let customTarget = "";
  let useCustomTarget = false;

  let scanMin = 10;
  let scanMax = 1000;
  let scanSteps = 20;
  let scanScale: ScanScale = "Linear";
  let eventsPerPoint = 1000;

  let scanning = false;
  let scanError = "";
  let result: ScanResult1D | null = null;

  let suggestedThresholdEnergy: number | null = null;
  let suggestedTotalWidth: number | null = null;
  let suggestedEventsPerPoint: number | null = null;

  let chartCanvas: HTMLCanvasElement;
  let chartInstance: Chart | null = null;
  let interopUnsub: (() => void) | null = null;

  function applyThresholdWindow(spanMultiplier = 0.2): void {
    if (!Number.isFinite(suggestedThresholdEnergy ?? NaN) || !suggestedThresholdEnergy) return;
    const centre = suggestedThresholdEnergy;
    const halfSpan = Math.max(centre * spanMultiplier, 1e-3);
    scanMin = Math.max(1e-3, +(centre - halfSpan).toPrecision(6));
    scanMax = +(centre + halfSpan).toPrecision(6);
    if (!useCustomTarget) {
      const cmsIdx = presets.findIndex((p) => p.target === "cms_energy");
      if (cmsIdx >= 0) selectedPresetIdx = cmsIdx;
    }
  }

  function applyResonanceWindow(widthMultiplier = 5): void {
    if (
      !Number.isFinite(suggestedThresholdEnergy ?? NaN)
      || !Number.isFinite(suggestedTotalWidth ?? NaN)
      || !suggestedThresholdEnergy
      || !suggestedTotalWidth
      || suggestedTotalWidth <= 0
    ) return;

    const centre = suggestedThresholdEnergy;
    const halfSpan = Math.max(widthMultiplier * suggestedTotalWidth, 1e-3);
    scanMin = Math.max(1e-3, +(centre - halfSpan).toPrecision(6));
    scanMax = +(centre + halfSpan).toPrecision(6);
    if (!useCustomTarget) {
      const cmsIdx = presets.findIndex((p) => p.target === "cms_energy");
      if (cmsIdx >= 0) selectedPresetIdx = cmsIdx;
    }
  }

  function applySuggestedEvents(): void {
    if (!Number.isFinite(suggestedEventsPerPoint ?? NaN) || !suggestedEventsPerPoint) return;
    eventsPerPoint = Math.max(100, Math.floor(suggestedEventsPerPoint));
  }

  // -------------------------------------------------------------------------
  // Reactive Model Subscription
  // -------------------------------------------------------------------------

  const unsubModel = theoreticalModel.subscribe((m) => {
    presets = buildPresets(m);
    if (presets.length > 0 && selectedPresetIdx >= presets.length) {
      selectedPresetIdx = 0;
    }
  });

  // When the preset selection changes, update min/max defaults.
  $: {
    if (!useCustomTarget && presets[selectedPresetIdx]) {
      const p = presets[selectedPresetIdx];
      scanMin = +p.defaultMin.toPrecision(4);
      scanMax = +p.defaultMax.toPrecision(4);
    }
  }

  // -------------------------------------------------------------------------
  // Scan Execution
  // -------------------------------------------------------------------------

  async function runScan() {
    const model = get(theoreticalModel);
    if (!model) {
      scanError = "No model loaded. Load a theoretical model first.";
      return;
    }

    const reaction = get(activeReaction);
    let cmsEnergy = 100.0;
    let finalMasses = [0.000511, 0.000511]; // default: e⁺e⁻

    if (reaction) {
      cmsEnergy = Math.sqrt(Math.abs(reaction.initial.invariant_mass_sq));
      finalMasses = reaction.final_state.states.map((s) => s.particle.field.mass);
    }

    const target = useCustomTarget
      ? customTarget
      : presets[selectedPresetIdx]?.target ?? "cms_energy";

    scanning = true;
    scanError = "";
    result = null;

    try {
      logScanner(`[Scanner] Starting 1D scan: ${target} ∈ [${scanMin}, ${scanMax}], ${scanSteps} points, ${eventsPerPoint} events/pt`);

      const scanResult = await runParameterScan1D({
        variable: {
          target,
          min: scanMin,
          max: scanMax,
          steps: scanSteps,
          scale: scanScale,
        },
        model,
        final_masses: finalMasses,
        cms_energy: cmsEnergy,
        events_per_point: eventsPerPoint,
      });

      result = scanResult;
      logScanner(`[Scanner] Scan complete: ${scanResult.x_values.length} points evaluated.`);
      renderChart();
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      scanError = msg;
      logScanner(`[Scanner] Error: ${msg}`);
    } finally {
      scanning = false;
    }
  }

  // -------------------------------------------------------------------------
  // Chart Rendering
  // -------------------------------------------------------------------------

  function renderChart() {
    if (!result || !chartCanvas) return;

    if (chartInstance) {
      chartInstance.destroy();
      chartInstance = null;
    }

    const ctx = chartCanvas.getContext("2d");
    if (!ctx) return;

    const labels = result.x_values.map((x) =>
      x >= 1000 || x < 0.01 ? x.toExponential(2) : x.toPrecision(4),
    );

    // Build error band (y ± error).
    const upperBand = result.y_values.map((y, i) => y + result!.y_errors[i]);
    const lowerBand = result.y_values.map((y, i) => Math.max(0, y - result!.y_errors[i]));

    chartInstance = new Chart(ctx, {
      type: "line",
      data: {
        labels,
        datasets: [
          {
            label: "σ (pb)",
            data: result.y_values,
            borderColor: "#00d4ff",
            backgroundColor: "rgba(0, 212, 255, 0.15)",
            borderWidth: 2,
            pointRadius: 3,
            pointBackgroundColor: "#00d4ff",
            tension: 0.2,
            fill: false,
          },
          {
            label: "σ + δσ",
            data: upperBand,
            borderColor: "rgba(0, 212, 255, 0.3)",
            borderWidth: 1,
            borderDash: [4, 4],
            pointRadius: 0,
            fill: "+1",
            backgroundColor: "rgba(0, 212, 255, 0.08)",
          },
          {
            label: "σ − δσ",
            data: lowerBand,
            borderColor: "rgba(0, 212, 255, 0.3)",
            borderWidth: 1,
            borderDash: [4, 4],
            pointRadius: 0,
            fill: false,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          title: {
            display: true,
            text: `σ vs ${result.variable.target}`,
            color: "var(--color-text-primary)",
            font: { size: 13 },
          },
          legend: {
            display: true,
            labels: { color: "var(--color-text-muted)", boxWidth: 12, font: { size: 10 } },
          },
          tooltip: {
            callbacks: {
              label(ctx) {
                const i = ctx.dataIndex;
                if (ctx.datasetIndex === 0 && result) {
                  const y = result.y_values[i];
                  const e = result.y_errors[i];
                  return `σ = ${y.toExponential(3)} ± ${e.toExponential(2)} pb`;
                }
                return "";
              },
            },
          },
        },
        scales: {
          x: {
            title: {
              display: true,
              text: result.variable.target,
              color: "var(--color-text-muted)",
            },
            ticks: { color: "var(--color-text-muted)", maxRotation: 45 },
            grid: { color: "rgba(255,255,255,0.05)" },
          },
          y: {
            title: {
              display: true,
              text: "σ (pb)",
              color: "var(--color-text-muted)",
            },
            ticks: { color: "var(--color-text-muted)" },
            grid: { color: "rgba(255,255,255,0.05)" },
          },
        },
      },
    });
  }

  // -------------------------------------------------------------------------
  // Lifecycle
  // -------------------------------------------------------------------------

  onMount(() => {
    interopUnsub = widgetInteropState.subscribe((state) => {
      if (scanning) return;

      const reactionPayload = state.reaction?.payload as
        | { cmsEnergy?: number }
        | undefined;
      if (reactionPayload?.cmsEnergy && Number.isFinite(reactionPayload.cmsEnergy)) {
        const current = presets.find((p) => p.target === "cms_energy");
        if (current && !useCustomTarget) {
          scanMin = Math.max(1, reactionPayload.cmsEnergy * 0.2);
          scanMax = reactionPayload.cmsEnergy * 2;
        }
      }

      const analysisPayload = state.analysis?.payload as
        | { numEvents?: number; cmsEnergy?: number }
        | undefined;
      if (analysisPayload?.numEvents && Number.isFinite(analysisPayload.numEvents)) {
        suggestedEventsPerPoint = Math.max(100, Math.floor(Math.max(analysisPayload.numEvents / 10, 100)));
      }

      const kinematicsPayload = state.kinematics?.payload as
        | { thresholdEnergy?: number | null }
        | undefined;
      if (
        kinematicsPayload?.thresholdEnergy != null
        && Number.isFinite(kinematicsPayload.thresholdEnergy)
      ) {
        suggestedThresholdEnergy = kinematicsPayload.thresholdEnergy;
      }

      // Auto-range around a decay resonance reported by Decay Calculator.
      const decayPayload = state.decay_calculator?.payload as
        | { totalWidth?: number | null; thresholdEnergy?: number | null }
        | undefined;
      if (decayPayload?.totalWidth && Number.isFinite(decayPayload.totalWidth)) {
        suggestedTotalWidth = decayPayload.totalWidth;
      }
      if (decayPayload?.thresholdEnergy && Number.isFinite(decayPayload.thresholdEnergy)) {
        suggestedThresholdEnergy = decayPayload.thresholdEnergy;
      }
      if (decayPayload?.totalWidth && decayPayload.totalWidth > 0) {
        const w = decayPayload.totalWidth;
        const kinPayload = state.kinematics?.payload as { thresholdEnergy?: number | null } | undefined;
        if (kinPayload?.thresholdEnergy && Number.isFinite(kinPayload.thresholdEnergy)) {
          const E0 = kinPayload.thresholdEnergy;
          if (!useCustomTarget && presets[selectedPresetIdx]?.target === "cms_energy") {
            scanMin = Math.max(0.1, +(E0 - 5 * w).toPrecision(5));
            scanMax = +(E0 + 5 * w).toPrecision(5);
          }
        }
      }
    });

    registerCommand({
      id: "spire.runParameterScan",
      title: "Run Parameter Scan",
      category: "Scanner",
      execute: runScan,
    });
  });

  onDestroy(() => {
    interopUnsub?.();
    unsubModel();
    unregisterCommand("spire.runParameterScan");
    if (chartInstance) {
      chartInstance.destroy();
      chartInstance = null;
    }
  });

  $: publishWidgetInterop("parameter_scanner", {
    target: useCustomTarget ? customTarget : (presets[selectedPresetIdx]?.target ?? "cms_energy"),
    range: [scanMin, scanMax],
    steps: scanSteps,
    scale: scanScale,
    eventsPerPoint,
    scanning,
    points: result?.x_values.length ?? 0,
  });

  // ---------------------------------------------------------------------------
  // CSV Export
  // ---------------------------------------------------------------------------
  function exportCsv(): void {
    if (!result) return;
    const header = `${result.variable.target},sigma_pb,error_pb`;
    const rows = result.x_values.map((x, i) =>
      `${x},${result!.y_values[i]},${result!.y_errors[i]}`
    );
    const csv = [header, ...rows].join("\n");
    const blob = new Blob([csv], { type: "text/csv" });
    const url  = URL.createObjectURL(blob);
    const a    = document.createElement("a");
    a.href     = url;
    a.download = "spire_scan.csv";
    a.click();
    URL.revokeObjectURL(url);
    logScanner("[Scanner] CSV exported: spire_scan.csv");
  }

  // ---------------------------------------------------------------------------
  // Context menus
  // ---------------------------------------------------------------------------
  function openChartContext(e: MouseEvent): void {
    e.preventDefault();
    e.stopPropagation();
    if (!result) return;
    const maxSigma = Math.max(...result.y_values);
    showContextMenu(e.clientX, e.clientY, [
      { type: "action", id: "export-csv",  label: "Export scan data as CSV", icon: "CSV",
        action: exportCsv },
      { type: "action", id: "copy-max",    label: `Copy peak σ = ${maxSigma.toExponential(4)} pb`, icon: "CP",
        action: () => navigator.clipboard.writeText(`${maxSigma.toExponential(4)} pb`) },
      { type: "separator", id: "sep-1" },
      { type: "action", id: "copy-range",  label: "Copy scan range", icon: "RNG",
        action: () =>
          navigator.clipboard.writeText(`[${scanMin}, ${scanMax}] (${scanSteps} pts, ${scanScale})`) },
    ]);
  }

  function openRowContext(e: MouseEvent, x: number, y: number, err: number): void {
    e.preventDefault();
    e.stopPropagation();
    const label = result?.variable.target ?? "x";
    showContextMenu(e.clientX, e.clientY, [
      { type: "action", id: "copy-point", label: `Copy: σ = ${y.toExponential(4)} ± ${err.toExponential(2)} pb`, icon: "CP",
        action: () => navigator.clipboard.writeText(`${x} → ${y.toExponential(4)} ± ${err.toExponential(2)} pb`) },
      { type: "action", id: "copy-x",    label: `Copy ${label} = ${x}`, icon: "X",
        action: () => navigator.clipboard.writeText(String(x)) },
    ]);
  }
</script>

<div class="scanner-widget" use:isolateEvents>
  <h3 class="widget-title">
    <HoverDef term="parameter_scan">Parameter Scanner</HoverDef>
  </h3>

  <!-- Configuration Panel -->
  <div class="config-panel">

    <!-- Target Selection -->
    <div class="field">
      <label for="scan-target">
        <HoverDef term="scan_target">Scan Target</HoverDef>
      </label>
      <div class="target-row">
        <select id="scan-target" bind:value={selectedPresetIdx} disabled={useCustomTarget}>
          {#each presets as preset, i}
            <option value={i}>{preset.label}</option>
          {/each}
        </select>
        <label class="custom-toggle">
          <input type="checkbox" bind:checked={useCustomTarget} />
          custom
        </label>
      </div>
      {#if useCustomTarget}
        <input
          type="text"
          class="custom-input"
          placeholder="e.g. field.Z_prime.mass"
          bind:value={customTarget}
        />
      {:else if presets[selectedPresetIdx]}
        <span class="hint">{presets[selectedPresetIdx].description}</span>
      {/if}
    </div>

    <!-- Range -->
    <div class="row-2col">
      <div class="field">
        <label for="scan-min">Min</label>
        <SpireNumberInput inputId="scan-min" step={0.1} bind:value={scanMin} ariaLabel="Scan minimum" />
      </div>
      <div class="field">
        <label for="scan-max">Max</label>
        <SpireNumberInput inputId="scan-max" step={0.1} bind:value={scanMax} ariaLabel="Scan maximum" />
      </div>
    </div>

    <!-- Steps & Scale -->
    <div class="row-2col">
      <div class="field">
        <label for="scan-steps">Steps</label>
        <SpireNumberInput inputId="scan-steps" min={2} max={500} step={1} bind:value={scanSteps} ariaLabel="Scan steps" />
      </div>
      <div class="field">
        <label for="scan-scale">
          <HoverDef term="scan_scale">Scale</HoverDef>
        </label>
        <select id="scan-scale" bind:value={scanScale}>
          <option value="Linear">Linear</option>
          <option value="Logarithmic">Logarithmic</option>
        </select>
      </div>
    </div>

    <!-- Events per point -->
    <div class="field">
      <label for="scan-events">
        <HoverDef term="mc_events">Events / Point</HoverDef>
      </label>
      <SpireNumberInput inputId="scan-events" min={100} step={100} bind:value={eventsPerPoint} ariaLabel="Events per point" />
    </div>

    <div class="quick-actions">
      <span class="quick-actions-label">Quick presets</span>
      <button class="quick-btn" on:click={() => applyThresholdWindow(0.2)} disabled={!suggestedThresholdEnergy}>
        Threshold ±20%
      </button>
      <button class="quick-btn" on:click={() => applyResonanceWindow(5)} disabled={!suggestedThresholdEnergy || !suggestedTotalWidth}>
        Resonance ±5Γ
      </button>
      <button class="quick-btn" on:click={applySuggestedEvents} disabled={!suggestedEventsPerPoint}>
        Suggested events
      </button>
    </div>

    <!-- Run Button -->
    <button class="run-btn" on:click={runScan} disabled={scanning}
      use:tooltip={{ text: "Execute the 1D parameter scan and render σ vs parameter curve" }}>
      {#if scanning}
        <span class="spinner"></span> Scanning…
      {:else}
        Run Scan
      {/if}
    </button>

    {#if scanError}
      <div class="error-msg">{scanError}</div>
    {/if}
  </div>

  <!-- Chart Area -->
  <div class="chart-area">
    {#if result}
      <div class="canvas-wrap"
        role="img"
        aria-label="Parameter scan chart"
        use:isolateEvents={{ wheel: true, pointer: false, mouse: false, touch: true }}
        on:contextmenu|preventDefault|stopPropagation={openChartContext}>
        <canvas bind:this={chartCanvas}></canvas>
      </div>

      <!-- Summary Table -->
      <div class="summary-table"
        use:isolateEvents={{ wheel: true, pointer: false, mouse: false, touch: true }}>
        <table>
          <thead>
            <tr>
              <th use:tooltip={{ text: "Scanned parameter value" }}>{result.variable.target}</th>
              <th use:tooltip={{ text: "Monte Carlo cross-section estimate σ [pb]" }}>σ (pb)</th>
              <th use:tooltip={{ text: "Statistical MC uncertainty on σ [pb]" }}>δσ (pb)</th>
            </tr>
          </thead>
          <tbody>
            {#each result.x_values as x, i}
              <tr on:contextmenu|preventDefault|stopPropagation={(e) =>
                openRowContext(e, x, result?.y_values?.[i] ?? 0, result?.y_errors?.[i] ?? 0)}>
                <td>{x >= 1000 || x < 0.01 ? x.toExponential(3) : x.toPrecision(4)}</td>
                <td>{result.y_values[i].toExponential(4)}</td>
                <td>{result.y_errors[i].toExponential(2)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      <div class="export-row">
        <button class="export-btn" on:click={exportCsv}
          use:tooltip={{ text: "Download scan results as a CSV file" }}>
          Export CSV
        </button>
      </div>
    {:else if !scanning}
      <div class="empty-state">
        <p>Configure a scan target and range above, then press <strong>Run Scan</strong>.</p>
        <p class="hint">Tip: Load a model and set up a reaction for realistic final-state masses.</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .scanner-widget {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    height: 100%;
    overflow-y: auto;
    padding: 0.75rem;
    font-size: 0.8rem;
    color: var(--color-text-primary);
    align-content: flex-start;
  }

  .widget-title {
    margin: 0 0 0.25rem;
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--color-text-primary);
    border-bottom: 1px solid var(--color-border);
    padding-bottom: 0.35rem;
  }

  .config-panel {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    padding-bottom: 0.4rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .field label {
    color: var(--color-text-muted);
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .target-row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .target-row select {
    flex: 1;
  }

  .custom-toggle {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.7rem;
    color: var(--color-text-muted);
    cursor: pointer;
    text-transform: none;
  }

  .custom-input {
    margin-top: 0.2rem;
  }

  .hint {
    font-size: 0.68rem;
    color: var(--color-text-muted);
    font-style: italic;
  }

  .row-2col {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }

  input[type="text"],
  select {
    background: var(--color-bg-inset);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    color: var(--color-text-primary);
    padding: 0.3rem 0.45rem;
    font-size: 0.78rem;
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  input:focus,
  select:focus {
    outline: none;
    border-color: var(--color-accent);
    box-shadow: 0 0 0 1px rgba(var(--color-accent-rgb), 0.25);
  }

  .run-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 0.5rem 1rem;
    border: 1px solid color-mix(in srgb, var(--color-accent) 65%, var(--color-border));
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--color-accent) 14%, var(--color-bg-surface));
    color: var(--color-text-primary);
    font-weight: 600;
    font-size: 0.82rem;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s, opacity 0.15s;
  }

  .run-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--color-accent) 24%, var(--color-bg-surface));
    border-color: color-mix(in srgb, var(--color-accent) 90%, var(--color-border));
  }

  .run-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .spinner {
    display: inline-block;
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: var(--color-text-primary);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  .quick-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem;
    margin-top: 0.1rem;
  }

  .quick-actions-label {
    font-size: 0.68rem;
    color: var(--color-text-muted);
    margin-right: 0.2rem;
  }

  .quick-btn {
    border: 1px solid var(--color-border);
    background: color-mix(in srgb, var(--color-bg-surface) 88%, transparent);
    color: var(--color-text-muted);
    font-size: 0.68rem;
    padding: 0.18rem 0.48rem;
    cursor: pointer;
  }

  .quick-btn:hover:not(:disabled) {
    color: var(--color-text-primary);
    border-color: color-mix(in srgb, var(--color-accent) 55%, var(--color-border));
  }

  .quick-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error-msg {
    color: var(--color-error);
    font-size: 0.75rem;
    padding: 0.3rem 0.5rem;
    background: rgba(var(--color-error-rgb), 0.1);
    border-radius: var(--radius-sm);
    border-left: 3px solid var(--color-error);
  }

  /* ── Chart Area ───────────────────────────────────────────────────── */

  .chart-area {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding-top: 0;
  }

  .chart-area canvas {
    width: 100% !important;
    height: 260px !important;
  }

  .canvas-wrap {
    position: relative;
    cursor: default;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 0.25rem;
    background: var(--color-bg-inset);
  }

  .canvas-wrap canvas {
    width: 100% !important;
    height: 260px !important;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    min-height: 140px;
    color: var(--color-text-muted);
    text-align: center;
  }

  .empty-state p {
    margin: 0;
  }

  /* ── Summary Table ────────────────────────────────────────────────── */

  .summary-table {
    max-height: 180px;
    overflow-y: auto;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.72rem;
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  thead {
    position: sticky;
    top: 0;
    background: var(--color-bg-inset);
  }

  th {
    text-align: right;
    padding: 0.3rem 0.5rem;
    color: var(--color-text-muted);
    border-bottom: 1px solid var(--color-border);
    font-weight: 500;
  }

  td {
    text-align: right;
    padding: 0.25rem 0.5rem;
    color: var(--color-text-primary);
    border-bottom: 1px solid rgba(255, 255, 255, 0.03);
  }

  tr:hover td {
    background: rgba(var(--color-accent-rgb), 0.05);
  }

  /* ── Export Row ─────────────────────────────────────────────────── */
  .export-row {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .export-btn {
    padding: 0.3rem 0.65rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-muted);
    font-size: 0.72rem;
    cursor: pointer;
    transition: background var(--transition-fast), border-color var(--transition-fast);
  }

  .export-btn:hover {
    background: rgba(var(--color-text-primary-rgb), 0.06);
    border-color: rgba(var(--color-accent-rgb), 0.3);
    color: var(--color-text-primary);
  }
</style>
