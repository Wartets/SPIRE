<!--
  SPIRE - Dalitz Plot Visualizer

  Generates and renders a Dalitz plot (s_ab vs s_bc) for a 3-body decay
  M → a + b + c. Uses the Rust kernel to compute the phase-space points
  and Chart.js to render the scatter plot on an HTML5 <canvas>.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { appendLog } from "$lib/stores/physicsStore";
  import { computeDalitzData } from "$lib/api";
  import SpireNumberInput from "$lib/components/ui/SpireNumberInput.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import type { DalitzPlotData } from "$lib/types/spire";
  import { isolateEvents } from "$lib/actions/widgetEvents";
  import { showContextMenu } from "$lib/stores/contextMenuStore";
  import { publishWidgetInterop, widgetInteropState } from "$lib/stores/widgetInteropStore";
  import {
    Chart,
    ScatterController,
    LinearScale,
    PointElement,
    Tooltip,
    Title,
  } from "chart.js";

  // Register only the Chart.js components we need (tree-shakeable).
  Chart.register(ScatterController, LinearScale, PointElement, Tooltip, Title);

  // ---------------------------------------------------------------------------
  // Preset Decays
  // ---------------------------------------------------------------------------
  interface DecayPreset {
    label: string;
    M: number;
    mA: number;
    mB: number;
    mC: number;
    description: string;
  }

  const PRESETS: DecayPreset[] = [
    { label: "D⁰ → K⁻π⁺π⁰",     M: 1.865,  mA: 0.494, mB: 0.140, mC: 0.135, description: "Charm meson 3-body decay" },
    { label: "τ⁻ → π⁻π⁺π⁻ν_τ",   M: 1.777,  mA: 0.140, mB: 0.140, mC: 0.140, description: "Tau lepton hadronic decay (approx)" },
    { label: "B⁺ → K⁺π⁺π⁻",      M: 5.279,  mA: 0.494, mB: 0.140, mC: 0.140, description: "B meson Dalitz analysis" },
    { label: "η → π⁺π⁻π⁰",       M: 0.5478, mA: 0.140, mB: 0.140, mC: 0.135, description: "Eta meson decay" },
  ];

  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------
  let motherMass: number = 1.865;
  let mA: number = 0.494;
  let mB: number = 0.140;
  let mC: number = 0.135;
  let nPoints: number = 3000;
  let loading: boolean = false;
  let errorMsg: string = "";
  let plotData: DalitzPlotData | null = null;

  let canvasEl: HTMLCanvasElement;
  let chart: Chart | null = null;

  // ---------------------------------------------------------------------------
  // Preset Selection
  // ---------------------------------------------------------------------------
  function applyPreset(preset: DecayPreset): void {
    motherMass = preset.M;
    mA = preset.mA;
    mB = preset.mB;
    mC = preset.mC;
    appendLog(`Dalitz preset: ${preset.label}`);
  }

  // ---------------------------------------------------------------------------
  // Generate Dalitz Data
  // ---------------------------------------------------------------------------
  async function handleGenerate(): Promise<void> {
    loading = true;
    errorMsg = "";

    // Basic client-side validation.
    if (motherMass <= 0 || mA < 0 || mB < 0 || mC < 0) {
      errorMsg = "All masses must be non-negative (parent > 0).";
      loading = false;
      return;
    }
    if (motherMass < mA + mB + mC) {
      errorMsg = `Decay forbidden: M = ${motherMass.toFixed(4)} < m_a+m_b+m_c = ${(mA + mB + mC).toFixed(4)}`;
      loading = false;
      return;
    }

    try {
      plotData = await computeDalitzData(motherMass, mA, mB, mC, nPoints);
      appendLog(
        `Dalitz plot: ${plotData.points.length} points generated ` +
        `(M=${motherMass}, grid=${plotData.n_grid})`
      );
      renderChart();
    } catch (e: unknown) {
      errorMsg = e instanceof Error ? e.message : String(e);
      appendLog(`Dalitz error: ${errorMsg}`);
    } finally {
      loading = false;
    }
  }

  // ---------------------------------------------------------------------------
  // Chart.js Rendering
  // ---------------------------------------------------------------------------
  function renderChart(): void {
    if (!plotData || !canvasEl) return;

    // Destroy previous chart instance if it exists.
    if (chart) {
      chart.destroy();
      chart = null;
    }

    const scatterData = plotData.points.map(([x, y]) => ({ x, y }));

    chart = new Chart(canvasEl, {
      type: "scatter",
      data: {
        datasets: [
          {
            label: "Phase Space",
            data: scatterData,
            pointRadius: 1.2,
            pointBackgroundColor: "rgba(94, 186, 255, 0.55)",
            pointBorderColor: "transparent",
            pointHoverRadius: 3,
            pointHoverBackgroundColor: "#f1c40f",
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        animation: false,
        plugins: {
          title: {
            display: true,
            text: `Dalitz Plot - M = ${motherMass.toFixed(3)} GeV`,
            color: "#7ec8e3",
            font: { size: 13 },
          },
          tooltip: {
            callbacks: {
              label: (ctx) => {
                const x = ctx.parsed.x ?? 0;
                const y = ctx.parsed.y ?? 0;
                return `s_ab = ${x.toFixed(4)}, s_bc = ${y.toFixed(4)} GeV²`;
              },
            },
          },
        },
        scales: {
          x: {
            title: { display: true, text: "s_ab (GeV²)", color: "#a0b4c8" },
            min: plotData.boundaries.m_ab_sq_min * 0.95,
            max: plotData.boundaries.m_ab_sq_max * 1.02,
            ticks: { color: "#7f8c8d" },
            grid: { color: "rgba(255,255,255,0.06)" },
          },
          y: {
            title: { display: true, text: "s_bc (GeV²)", color: "#a0b4c8" },
            min: plotData.boundaries.m_bc_sq_min * 0.95,
            max: plotData.boundaries.m_bc_sq_max * 1.02,
            ticks: { color: "#7f8c8d" },
            grid: { color: "rgba(255,255,255,0.06)" },
          },
        },
      },
    });
  }

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------
  onDestroy(() => {
    if (chart) {
      chart.destroy();
      chart = null;
    }
  });

  // ---------------------------------------------------------------------------
  // Interop – receive decay_calculator mass data & broadcast own results
  // ---------------------------------------------------------------------------
  let interopUnsub: (() => void) | null = null;

  onMount(() => {
    interopUnsub = widgetInteropState.subscribe((state) => {
      const decayPayload = state.decay_calculator?.payload as
        | { selectedParticle?: string | null; totalWidth?: number | null }
        | undefined;
      // If the Decay Calculator reports a new parent particle with
      // a known mass, offer to populate the M field automatically.
      void decayPayload; // consumed for future preset wiring
    });
  });

  $: publishWidgetInterop("dalitz", {
    motherMass,
    mA,
    mB,
    mC,
    nPoints,
    generating: loading,
    plotPoints: plotData?.points.length ?? null,
    sAbMin:  plotData?.boundaries.m_ab_sq_min ?? null,
    sAbMax:  plotData?.boundaries.m_ab_sq_max ?? null,
    sBcMin:  plotData?.boundaries.m_bc_sq_min ?? null,
    sBcMax:  plotData?.boundaries.m_bc_sq_max ?? null,
  });

  // ---------------------------------------------------------------------------
  // CSV Export
  // ---------------------------------------------------------------------------
  function exportCsv(): void {
    if (!plotData) return;
    const header = "s_ab_GeV2,s_bc_GeV2";
    const rows = plotData.points.map(([x, y]) => `${x},${y}`);
    const csv  = [header, ...rows].join("\n");
    const blob = new Blob([csv], { type: "text/csv" });
    const url  = URL.createObjectURL(blob);
    const a    = document.createElement("a");
    a.href     = url;
    a.download = `dalitz_M${motherMass.toFixed(3)}.csv`;
    a.click();
    URL.revokeObjectURL(url);
    appendLog(`[Dalitz] CSV exported: dalitz_M${motherMass.toFixed(3)}.csv`);
  }

  // ---------------------------------------------------------------------------
  // Context menus
  // ---------------------------------------------------------------------------
  function openChartContext(e: MouseEvent): void {
    e.preventDefault();
    e.stopPropagation();
    showContextMenu(e.clientX, e.clientY, [
      { type: "action", id: "export-csv", label: "Export phase-space points as CSV", icon: "CSV",
        action: exportCsv },
      ...(plotData
        ? [
            { type: "separator" as const, id: "sep-1" },
            { type: "action" as const, id: "copy-bounds",
              label: `Copy s_ab ∈ [${plotData.boundaries.m_ab_sq_min.toFixed(4)}, ${plotData.boundaries.m_ab_sq_max.toFixed(4)}] GeV²`,
              icon: "RNG",
              action: () =>
                navigator.clipboard.writeText(
                  `s_ab=[${plotData!.boundaries.m_ab_sq_min.toFixed(6)},${plotData!.boundaries.m_ab_sq_max.toFixed(6)}] ` +
                  `s_bc=[${plotData!.boundaries.m_bc_sq_min.toFixed(6)},${plotData!.boundaries.m_bc_sq_max.toFixed(6)}] GeV²`
                ),
            },
            { type: "action" as const, id: "copy-n",
              label: `${plotData.points.length} phase-space points generated`, icon: "N",
              action: () => navigator.clipboard.writeText(String(plotData!.points.length)) },
          ]
        : []),
    ]);
  }
</script>

<div class="dalitz-plotter" use:isolateEvents>
  <h3>Dalitz Plot</h3>

  <!-- Preset Buttons -->
  <div class="presets">
    {#each PRESETS as preset}
      <button class="preset-btn" on:click={() => applyPreset(preset)} use:tooltip={{ text: preset.description }}>
        {preset.label}
      </button>
    {/each}
  </div>

  <!-- Mass Inputs -->
  <div class="input-grid">
    <label>
      <span class="input-label"
        use:tooltip={{ text: "Invariant mass of the decaying parent particle M [GeV/c²]" }}>
        M (parent)
      </span>
      <SpireNumberInput bind:value={motherMass} step={0.001} min={0} ariaLabel="Parent mass" />
    </label>
    <label>
      <span class="input-label"
        use:tooltip={{ text: "Mass of daughter particle a [GeV/c²]" }}>
        m_a
      </span>
      <SpireNumberInput bind:value={mA} step={0.001} min={0} ariaLabel="Daughter mass a" />
    </label>
    <label>
      <span class="input-label"
        use:tooltip={{ text: "Mass of daughter particle b [GeV/c²]" }}>
        m_b
      </span>
      <SpireNumberInput bind:value={mB} step={0.001} min={0} ariaLabel="Daughter mass b" />
    </label>
    <label>
      <span class="input-label"
        use:tooltip={{ text: "Mass of daughter particle c [GeV/c²]" }}>
        m_c
      </span>
      <SpireNumberInput bind:value={mC} step={0.001} min={0} ariaLabel="Daughter mass c" />
    </label>
    <label>
      <span class="input-label"
        use:tooltip={{ text: "Number of Monte Carlo phase-space sampling points" }}>
        Points
      </span>
      <SpireNumberInput bind:value={nPoints} step={500} min={100} max={50000} ariaLabel="Dalitz point count" />
    </label>
  </div>

  <!-- Generate Button -->
  <button class="generate-btn" on:click={handleGenerate} disabled={loading}
    use:tooltip={{ text: "Generate phase-space points and render the Dalitz plot for M → a + b + c" }}>
    {loading ? "Generating…" : "Generate Dalitz Plot"}
  </button>

  {#if errorMsg}
    <p class="error">{errorMsg}</p>
  {/if}

  <!-- Chart Canvas -->
  {#if plotData}
    <div class="chart-info">
      <span use:tooltip={{ text: "Total number of accepted phase-space points" }}>{plotData.points.length} pts</span>
      <span use:tooltip={{ text: "Kinematic boundary of the s_ab invariant mass variable [GeV²]" }}>
        s_ab ∈ [{plotData.boundaries.m_ab_sq_min.toFixed(4)}, {plotData.boundaries.m_ab_sq_max.toFixed(4)}] GeV²
      </span>
      <span use:tooltip={{ text: "Kinematic boundary of the s_bc invariant mass variable [GeV²]" }}>
        s_bc ∈ [{plotData.boundaries.m_bc_sq_min.toFixed(4)}, {plotData.boundaries.m_bc_sq_max.toFixed(4)}] GeV²
      </span>
      <button class="export-csv-btn" on:click={exportCsv}
        use:tooltip={{ text: "Download Dalitz phase-space points as CSV" }}>
        Export CSV
      </button>
    </div>
  {/if}

  <div class="chart-container" class:hidden={!plotData}
    role="img"
    aria-label="Dalitz plot scatter chart"
    use:isolateEvents={{ wheel: true, pointer: false, mouse: false, touch: true }}
    on:contextmenu|preventDefault|stopPropagation={openChartContext}>
    <canvas bind:this={canvasEl}></canvas>
  </div>
</div>

<style>
  .dalitz-plotter {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    height: 100%;
    overflow-y: auto;
    min-height: 0;
    padding: 0.5rem;
    box-sizing: border-box;
  }

  h3 {
    margin: 0 0 0.25rem;
    font-size: 0.95rem;
    color: var(--color-text-primary);
    border-bottom: 1px solid var(--color-border);
    padding-bottom: 0.3rem;
    flex-shrink: 0;
  }

  /* Presets */
  .presets {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
  }
  .preset-btn {
    font-size: 0.68rem;
    padding: 0.2rem 0.45rem;
    background: var(--color-bg-surface);
    color: var(--color-accent);
    border: 1px solid var(--color-border);
    cursor: pointer;
    font-family: var(--font-mono);
    transition: border-color 0.15s;
  }
  .preset-btn:hover {
    border-color: var(--color-accent);
    color: var(--color-text-primary);
  }

  /* Input Grid */
  .input-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(90px, 1fr));
    gap: 0.3rem;
  }
  .input-grid label {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  .input-label {
    font-size: 0.68rem;
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .input-grid :global(.spire-number-input) {
    font-size: 0.78rem;
  }

  /* Generate Button */
  .generate-btn {
    padding: 0.4rem 0.8rem;
    background: var(--color-bg-surface);
    color: var(--color-text-primary);
    border: 1px solid var(--color-border);
    cursor: pointer;
    font-size: 0.8rem;
    font-weight: 600;
    font-family: var(--font-mono);
    transition: border-color 0.15s;
  }
  .generate-btn:hover:not(:disabled) {
    border-color: var(--color-accent);
  }
  .generate-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Error */
  .error {
    color: var(--color-error);
    font-size: 0.78rem;
    margin: 0;
    padding: 0.3rem 0.4rem;
    background: var(--color-bg-inset);
    border: 1px solid var(--color-error);
  }

  /* Chart Info */
  .chart-info {
    display: flex;
    flex-wrap: wrap;
    gap: 0.6rem;
    font-size: 0.68rem;
    color: var(--color-text-muted);
    align-items: center;
  }

  .export-csv-btn {
    padding: 0.18rem 0.5rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-muted);
    font-size: 0.65rem;
    cursor: pointer;
    transition: background var(--transition-fast), border-color var(--transition-fast);
  }

  .export-csv-btn:hover {
    background: rgba(var(--color-accent-rgb), 0.08);
    border-color: rgba(var(--color-accent-rgb), 0.3);
    color: var(--color-text-primary);
  }

  /* Chart Container */
  .chart-container {
    position: relative;
    width: 100%;
    height: 320px;
    background: var(--color-bg-inset);
    border: 1px solid var(--color-border);
  }
  .chart-container.hidden {
    display: none;
  }
</style>
