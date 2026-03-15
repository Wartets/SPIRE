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
</script>

<div class="dalitz-plotter">
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
      <span class="input-label">M (parent)</span>
      <SpireNumberInput bind:value={motherMass} step={0.001} min={0} ariaLabel="Parent mass" />
    </label>
    <label>
      <span class="input-label">m_a</span>
      <SpireNumberInput bind:value={mA} step={0.001} min={0} ariaLabel="Daughter mass a" />
    </label>
    <label>
      <span class="input-label">m_b</span>
      <SpireNumberInput bind:value={mB} step={0.001} min={0} ariaLabel="Daughter mass b" />
    </label>
    <label>
      <span class="input-label">m_c</span>
      <SpireNumberInput bind:value={mC} step={0.001} min={0} ariaLabel="Daughter mass c" />
    </label>
    <label>
      <span class="input-label">Points</span>
      <SpireNumberInput bind:value={nPoints} step={500} min={100} max={50000} ariaLabel="Dalitz point count" />
    </label>
  </div>

  <!-- Generate Button -->
  <button class="generate-btn" on:click={handleGenerate} disabled={loading}>
    {loading ? "Generating…" : "Generate Dalitz Plot"}
  </button>

  {#if errorMsg}
    <p class="error">{errorMsg}</p>
  {/if}

  <!-- Chart Canvas -->
  {#if plotData}
    <div class="chart-info">
      <span>{plotData.points.length} points</span>
      <span>s_ab ∈ [{plotData.boundaries.m_ab_sq_min.toFixed(4)}, {plotData.boundaries.m_ab_sq_max.toFixed(4)}] GeV²</span>
      <span>s_bc ∈ [{plotData.boundaries.m_bc_sq_min.toFixed(4)}, {plotData.boundaries.m_bc_sq_max.toFixed(4)}] GeV²</span>
    </div>
  {/if}

  <div class="chart-container" class:hidden={!plotData}>
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
  }
  h3 {
    margin: 0 0 0.25rem;
    font-size: 0.95rem;
    color: var(--fg-accent);
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.3rem;
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
    background: var(--bg-surface);
    color: var(--hl-symbol);
    border: 1px solid var(--border);
    cursor: pointer;
    font-family: var(--font-mono);
    transition: border-color 0.15s;
  }
  .preset-btn:hover {
    border-color: var(--border-focus);
    color: var(--fg-accent);
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
    color: var(--fg-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .input-grid :global(.spire-number-input) {
    font-size: 0.78rem;
  }

  /* Generate Button */
  .generate-btn {
    padding: 0.4rem 0.8rem;
    background: var(--bg-surface);
    color: var(--fg-primary);
    border: 1px solid var(--border);
    cursor: pointer;
    font-size: 0.8rem;
    font-weight: 600;
    font-family: var(--font-mono);
    transition: border-color 0.15s;
  }
  .generate-btn:hover:not(:disabled) {
    border-color: var(--border-focus);
  }
  .generate-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Error */
  .error {
    color: var(--hl-error);
    font-size: 0.78rem;
    margin: 0;
    padding: 0.3rem 0.4rem;
    background: var(--bg-inset);
    border: 1px solid var(--hl-error);
  }

  /* Chart Info */
  .chart-info {
    display: flex;
    flex-wrap: wrap;
    gap: 0.6rem;
    font-size: 0.68rem;
    color: var(--fg-secondary);
  }

  /* Chart Container */
  .chart-container {
    position: relative;
    width: 100%;
    height: 320px;
    background: var(--bg-inset);
    border: 1px solid var(--border);
  }
  .chart-container.hidden {
    display: none;
  }
</style>
