<!--
  SPIRE — Analysis Widget

  Interactive kinematic analysis and histogramming. Allows users to define
  Rhai observable scripts (e.g., pT, invariant mass), configure histogram
  binning, run Monte Carlo event generation, and visualise the resulting
  distributions as bar charts using Chart.js.

  Supports linear/logarithmic Y-axis scaling.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { appendLog } from "$lib/stores/physicsStore";
  import { runAnalysis, validateScript } from "$lib/api";
  import type { AnalysisResult, HistogramData } from "$lib/types/spire";
  import {
    Chart,
    BarController,
    BarElement,
    CategoryScale,
    LinearScale,
    LogarithmicScale,
    Tooltip,
    Title,
    Legend,
  } from "chart.js";

  // Register only the Chart.js components we need (tree-shakeable).
  Chart.register(
    BarController,
    BarElement,
    CategoryScale,
    LinearScale,
    LogarithmicScale,
    Tooltip,
    Title,
    Legend,
  );

  // ---------------------------------------------------------------------------
  // Observable Presets
  // ---------------------------------------------------------------------------
  interface ObservablePreset {
    label: string;
    script: string;
    min: number;
    max: number;
    nBins: number;
    description: string;
  }

  const PRESETS: ObservablePreset[] = [
    {
      label: "Leading pT",
      script: "event.momenta[0].pt()",
      min: 0, max: 60, nBins: 30,
      description: "Transverse momentum of the first final-state particle",
    },
    {
      label: "Subleading pT",
      script: "event.momenta[1].pt()",
      min: 0, max: 60, nBins: 30,
      description: "Transverse momentum of the second final-state particle",
    },
    {
      label: "Invariant Mass (pair)",
      script: "let p1 = event.momenta[0]; let p2 = event.momenta[1]; let s = p1 + p2; s.m()",
      min: 50, max: 150, nBins: 40,
      description: "Invariant mass of the first two final-state particles",
    },
    {
      label: "Pseudorapidity η",
      script: "event.momenta[0].eta()",
      min: -5, max: 5, nBins: 50,
      description: "Pseudorapidity of the first final-state particle",
    },
    {
      label: "Azimuthal Angle φ",
      script: "event.momenta[0].phi()",
      min: -3.15, max: 3.15, nBins: 32,
      description: "Azimuthal angle of the first final-state particle",
    },
    {
      label: "Energy",
      script: "event.momenta[0].e()",
      min: 0, max: 100, nBins: 25,
      description: "Energy of the first final-state particle",
    },
  ];

  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------
  let plotName: string = "Leading pT";
  let observableScript: string = "event.momenta[0].pt()";
  let histMin: number = 0;
  let histMax: number = 60;
  let nBins: number = 30;
  let cmsEnergy: number = 100;
  let numEvents: number = 5000;
  let nFinalState: number = 2;
  let cutScript: string = "";

  let loading: boolean = false;
  let errorMsg: string = "";
  let scriptValid: boolean = true;
  let validationMsg: string = "";

  let logScale: boolean = false;
  let result: AnalysisResult | null = null;
  let activeHistogram: HistogramData | null = null;

  let canvasEl: HTMLCanvasElement;
  let chart: Chart | null = null;

  // ---------------------------------------------------------------------------
  // Preset Selection
  // ---------------------------------------------------------------------------
  function applyPreset(preset: ObservablePreset): void {
    plotName = preset.label;
    observableScript = preset.script;
    histMin = preset.min;
    histMax = preset.max;
    nBins = preset.nBins;
    appendLog(`Analysis preset: ${preset.label}`);
  }

  // ---------------------------------------------------------------------------
  // Script Validation
  // ---------------------------------------------------------------------------
  let validationTimer: ReturnType<typeof setTimeout> | null = null;

  function handleScriptInput(): void {
    if (validationTimer) clearTimeout(validationTimer);
    validationTimer = setTimeout(async () => {
      if (!observableScript.trim()) {
        scriptValid = false;
        validationMsg = "Script cannot be empty";
        return;
      }
      try {
        await validateScript(observableScript);
        scriptValid = true;
        validationMsg = "✓ Valid";
      } catch (err) {
        scriptValid = false;
        validationMsg = String(err);
      }
    }, 400);
  }

  // ---------------------------------------------------------------------------
  // Run Analysis
  // ---------------------------------------------------------------------------
  async function handleRun(): Promise<void> {
    loading = true;
    errorMsg = "";
    result = null;
    activeHistogram = null;

    // Client-side validation.
    if (nBins < 1 || nBins > 1000) {
      errorMsg = "Bin count must be between 1 and 1000.";
      loading = false;
      return;
    }
    if (histMin >= histMax) {
      errorMsg = "Histogram min must be less than max.";
      loading = false;
      return;
    }
    if (cmsEnergy <= 0) {
      errorMsg = "CMS energy must be positive.";
      loading = false;
      return;
    }
    if (numEvents < 1 || numEvents > 1_000_000) {
      errorMsg = "Number of events must be between 1 and 1,000,000.";
      loading = false;
      return;
    }

    const finalMasses = Array(nFinalState).fill(0.0);
    const cutScripts = cutScript.trim() ? [cutScript.trim()] : [];

    try {
      appendLog(
        `Running analysis: ${plotName}, ${numEvents} events at √s = ${cmsEnergy} GeV`,
      );

      const analysisResult = await runAnalysis({
        plots: [
          {
            name: plotName,
            observable_script: observableScript,
            n_bins: nBins,
            min: histMin,
            max: histMax,
          },
        ],
        cut_scripts: cutScripts,
        num_events: numEvents,
        cms_energy: cmsEnergy,
        final_masses: finalMasses,
      });

      result = analysisResult;
      if (analysisResult.histograms.length > 0) {
        activeHistogram = analysisResult.histograms[0];
        renderChart(activeHistogram);
      }

      const xsecPb = analysisResult.cross_section * 0.3894e9;
      const errPb = analysisResult.cross_section_error * 0.3894e9;
      appendLog(
        `Analysis complete: ${analysisResult.events_passed}/${analysisResult.events_generated} events passed` +
          ` | σ ≈ ${xsecPb.toExponential(3)} ± ${errPb.toExponential(2)} pb`,
      );
    } catch (err) {
      errorMsg = String(err);
      appendLog(`Analysis error: ${errorMsg}`);
    } finally {
      loading = false;
    }
  }

  // ---------------------------------------------------------------------------
  // Chart Rendering
  // ---------------------------------------------------------------------------
  function renderChart(data: HistogramData): void {
    if (!canvasEl) return;

    // Destroy previous chart.
    if (chart) {
      chart.destroy();
      chart = null;
    }

    // Build bin-centre labels.
    const labels: string[] = [];
    for (let i = 0; i < data.bin_contents.length; i++) {
      const centre =
        (data.bin_edges[i] + data.bin_edges[i + 1]) / 2;
      labels.push(centre.toFixed(2));
    }

    // Colour gradient from cool (low) to warm (high).
    const maxVal = Math.max(...data.bin_contents, 1e-30);
    const bgColors = data.bin_contents.map((v) => {
      const ratio = v / maxVal;
      const r = Math.round(30 + 200 * ratio);
      const g = Math.round(100 + 60 * (1 - ratio));
      const b = Math.round(220 - 150 * ratio);
      return `rgba(${r}, ${g}, ${b}, 0.85)`;
    });

    const borderColors = bgColors.map((c) => c.replace("0.85", "1.0"));

    chart = new Chart(canvasEl, {
      type: "bar",
      data: {
        labels,
        datasets: [
          {
            label: data.name,
            data: data.bin_contents,
            backgroundColor: bgColors,
            borderColor: borderColors,
            borderWidth: 1,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          title: {
            display: true,
            text: data.name,
            color: "#e0e0e0",
            font: { size: 14 },
          },
          legend: {
            display: false,
          },
          tooltip: {
            callbacks: {
              title: (items) => {
                const idx = items[0].dataIndex;
                const lo = data.bin_edges[idx].toFixed(2);
                const hi = data.bin_edges[idx + 1].toFixed(2);
                return `[${lo}, ${hi})`;
              },
              label: (item) => {
                const val = (item.raw as number).toExponential(3);
                const err = data.bin_errors[item.dataIndex].toExponential(2);
                return `${val} ± ${err}`;
              },
            },
          },
        },
        scales: {
          x: {
            title: {
              display: true,
              text: data.name,
              color: "#aaa",
            },
            ticks: { color: "#999", maxTicksLimit: 10 },
            grid: { color: "rgba(255,255,255,0.05)" },
          },
          y: {
            type: logScale ? "logarithmic" : "linear",
            title: {
              display: true,
              text: "Weighted Counts",
              color: "#aaa",
            },
            ticks: { color: "#999" },
            grid: { color: "rgba(255,255,255,0.08)" },
          },
        },
      },
    });
  }

  // ---------------------------------------------------------------------------
  // Scale Toggle
  // ---------------------------------------------------------------------------
  function toggleScale(): void {
    logScale = !logScale;
    if (activeHistogram) {
      renderChart(activeHistogram);
    }
  }

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------
  onMount(() => {
    handleScriptInput();
  });

  onDestroy(() => {
    if (chart) {
      chart.destroy();
      chart = null;
    }
    if (validationTimer) clearTimeout(validationTimer);
  });
</script>

<!-- ======================================================================= -->
<!-- Template                                                                 -->
<!-- ======================================================================= -->

<div class="analysis-widget">
  <!-- Configuration Panel -->
  <div class="config-panel">
    <!-- Preset Selector -->
    <div class="preset-row">
      <span class="preset-label">Presets:</span>
      <div class="preset-buttons">
        {#each PRESETS as preset}
          <button
            class="preset-btn"
            title={preset.description}
            on:click={() => applyPreset(preset)}
          >
            {preset.label}
          </button>
        {/each}
      </div>
    </div>

    <!-- Observable Script -->
    <div class="field">
      <label for="obs-script">Observable Script (Rhai):</label>
      <textarea
        id="obs-script"
        bind:value={observableScript}
        on:input={handleScriptInput}
        rows="3"
        spellcheck="false"
        class:invalid={!scriptValid}
      ></textarea>
      <span class="validation-msg" class:valid={scriptValid} class:error={!scriptValid}>
        {validationMsg}
      </span>
    </div>

    <!-- Plot Name & Binning -->
    <div class="field-row">
      <div class="field compact">
        <label for="plot-name">Name:</label>
        <input id="plot-name" type="text" bind:value={plotName} />
      </div>
      <div class="field compact">
        <label for="n-bins">Bins:</label>
        <input id="n-bins" type="number" bind:value={nBins} min="1" max="1000" />
      </div>
      <div class="field compact">
        <label for="hist-min">Min:</label>
        <input id="hist-min" type="number" bind:value={histMin} step="any" />
      </div>
      <div class="field compact">
        <label for="hist-max">Max:</label>
        <input id="hist-max" type="number" bind:value={histMax} step="any" />
      </div>
    </div>

    <!-- Physics Parameters -->
    <div class="field-row">
      <div class="field compact">
        <label for="cms-energy">√s (GeV):</label>
        <input id="cms-energy" type="number" bind:value={cmsEnergy} min="0.1" step="any" />
      </div>
      <div class="field compact">
        <label for="num-events">Events:</label>
        <input id="num-events" type="number" bind:value={numEvents} min="1" max="1000000" />
      </div>
      <div class="field compact">
        <label for="n-final">Final-state N:</label>
        <input id="n-final" type="number" bind:value={nFinalState} min="2" max="8" />
      </div>
    </div>

    <!-- Optional Cut Script -->
    <div class="field">
      <label for="cut-script">Cut Script (optional):</label>
      <input
        id="cut-script"
        type="text"
        bind:value={cutScript}
        placeholder="e.g., event.momenta[0].pt() > 20.0"
      />
    </div>

    <!-- Run Button -->
    <div class="run-row">
      <button
        class="run-btn"
        on:click={handleRun}
        disabled={loading || !scriptValid}
      >
        {#if loading}
          ⏳ Running…
        {:else}
          ▶ Run Analysis
        {/if}
      </button>
      {#if activeHistogram}
        <button class="scale-btn" on:click={toggleScale}>
          {logScale ? "Linear" : "Log"} Y
        </button>
      {/if}
    </div>

    <!-- Error Display -->
    {#if errorMsg}
      <div class="error-box">{errorMsg}</div>
    {/if}

    <!-- Summary Statistics -->
    {#if result}
      <div class="stats-panel">
        <div class="stat">
          <span class="stat-label">Events:</span>
          <span class="stat-value">
            {result.events_passed.toLocaleString()} / {result.events_generated.toLocaleString()} passed
          </span>
        </div>
        <div class="stat">
          <span class="stat-label">σ (pb):</span>
          <span class="stat-value">
            {(result.cross_section * 0.3894e9).toExponential(4)}
            ± {(result.cross_section_error * 0.3894e9).toExponential(2)}
          </span>
        </div>
        {#if activeHistogram}
          <div class="stat">
            <span class="stat-label">Mean:</span>
            <span class="stat-value">{activeHistogram.mean.toFixed(3)}</span>
          </div>
          <div class="stat">
            <span class="stat-label">UF / OF:</span>
            <span class="stat-value">
              {activeHistogram.underflow.toExponential(2)} / {activeHistogram.overflow.toExponential(2)}
            </span>
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Chart Canvas -->
  <div class="chart-container">
    <canvas bind:this={canvasEl}></canvas>
    {#if !activeHistogram && !loading}
      <div class="placeholder">
        Configure an observable and click <strong>Run Analysis</strong> to generate a histogram.
      </div>
    {/if}
  </div>
</div>

<!-- ======================================================================= -->
<!-- Styles                                                                   -->
<!-- ======================================================================= -->

<style>
  .analysis-widget {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    height: 100%;
    overflow-y: auto;
    font-size: 0.85rem;
  }

  .config-panel {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.5rem;
    background: rgba(255, 255, 255, 0.03);
    border-radius: 6px;
  }

  .preset-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .preset-row .preset-label {
    color: var(--hl-accent, #8ab4f8);
    font-weight: 600;
    white-space: nowrap;
  }

  .preset-buttons {
    display: flex;
    gap: 0.3rem;
    flex-wrap: wrap;
  }

  .preset-btn {
    padding: 0.2rem 0.5rem;
    font-size: 0.75rem;
    border: 1px solid rgba(138, 180, 248, 0.3);
    border-radius: 4px;
    background: rgba(138, 180, 248, 0.08);
    color: #ccc;
    cursor: pointer;
    transition: background 0.15s;
  }

  .preset-btn:hover {
    background: rgba(138, 180, 248, 0.2);
    color: #fff;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .field label {
    color: #aaa;
    font-size: 0.78rem;
  }

  .field-row {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .field.compact {
    flex: 1;
    min-width: 80px;
  }

  input,
  textarea {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 4px;
    color: #e0e0e0;
    padding: 0.3rem 0.5rem;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 0.8rem;
  }

  textarea {
    resize: vertical;
    min-height: 2.5rem;
  }

  textarea.invalid {
    border-color: rgba(255, 80, 80, 0.5);
  }

  .validation-msg {
    font-size: 0.72rem;
  }

  .validation-msg.valid {
    color: #66bb6a;
  }

  .validation-msg.error {
    color: #ef5350;
  }

  .run-row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .run-btn {
    padding: 0.4rem 1.2rem;
    font-size: 0.85rem;
    font-weight: 600;
    border: none;
    border-radius: 5px;
    background: linear-gradient(135deg, #4caf50, #2e7d32);
    color: #fff;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .run-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .run-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .scale-btn {
    padding: 0.3rem 0.8rem;
    font-size: 0.78rem;
    border: 1px solid rgba(138, 180, 248, 0.3);
    border-radius: 4px;
    background: rgba(138, 180, 248, 0.1);
    color: #8ab4f8;
    cursor: pointer;
  }

  .scale-btn:hover {
    background: rgba(138, 180, 248, 0.25);
  }

  .error-box {
    padding: 0.4rem 0.6rem;
    background: rgba(255, 50, 50, 0.12);
    border: 1px solid rgba(255, 50, 50, 0.3);
    border-radius: 4px;
    color: #ef5350;
    font-size: 0.8rem;
    word-break: break-word;
  }

  .stats-panel {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1.5rem;
    padding: 0.4rem 0;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }

  .stat {
    display: flex;
    gap: 0.3rem;
  }

  .stat-label {
    color: #888;
    font-size: 0.78rem;
  }

  .stat-value {
    color: #e0e0e0;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 0.78rem;
  }

  .chart-container {
    position: relative;
    flex: 1;
    min-height: 250px;
  }

  .chart-container canvas {
    width: 100% !important;
    height: 100% !important;
  }

  .placeholder {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #666;
    font-size: 0.85rem;
    text-align: center;
    padding: 1rem;
  }
</style>
