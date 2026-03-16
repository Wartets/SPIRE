<!--
  GlobalFitDashboard.svelte — Phase 55
  
  Interactive dashboard for MCMC global fits:
  
  1. **Parameter table**: Define which fields to float (mass, width, coupling),
     set prior ranges, and toggle individual parameters.
  2. **Constraint table**: Add Gaussian measurement constraints with observed
     value and uncertainty.
  3. **MCMC controls**: Set n_walkers, n_steps, burn-in, then start/stop.
  4. **Status display**: Live progress bar, acceptance fraction, step counter.
  5. **Corner plot**: Canvas-based rendering with 1D histograms on diagonal
     and 2D scatter density on lower triangle.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import SpireNumberInput from "$lib/components/ui/SpireNumberInput.svelte";
  import {
    startMcmcFit,
    getMcmcStatus,
    stopMcmcFit,
  } from "$lib/api";
  import type {
    FitParameter,
    FitProperty,
    GaussianConstraint,
    McmcFitRequest,
    McmcFitStatus,
    McmcStatus,
    TheoreticalModel,
  } from "$lib/types/spire";

  // === Reactive state ===
  let parameters: FitParameter[] = [
    {
      name: "Higgs mass",
      min: 120.0,
      max: 130.0,
      field_id: "higgs",
      property: "Mass" as FitProperty,
    },
  ];

  let constraints: GaussianConstraint[] = [
    {
      name: "LHC Higgs mass",
      observed: 125.25,
      sigma: 0.17,
      field_id: "higgs",
      property: "Mass" as FitProperty,
    },
  ];

  let nWalkers = 32;
  let nSteps = 5000;
  let burnIn = 1000;

  // Fit state
  let status: McmcStatus | null = null;
  let paramNames: string[] = [];
  let flatSamples: number[][] | null = null;
  let isRunning = false;
  let errorMsg = "";

  // Poll timer
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  // Corner plot
  let cornerCanvas: HTMLCanvasElement;

  // === Parameter table helpers ===
  function addParameter() {
    parameters = [
      ...parameters,
      {
        name: `Param ${parameters.length + 1}`,
        min: 0,
        max: 100,
        field_id: "",
        property: "Mass" as FitProperty,
      },
    ];
  }

  function removeParameter(i: number) {
    parameters = parameters.filter((_, idx) => idx !== i);
  }

  function addConstraint() {
    constraints = [
      ...constraints,
      {
        name: `Constraint ${constraints.length + 1}`,
        observed: 0,
        sigma: 1,
        field_id: "",
        property: "Mass" as FitProperty,
      },
    ];
  }

  function removeConstraint(i: number) {
    constraints = constraints.filter((_, idx) => idx !== i);
  }

  // === MCMC controls ===
  async function startFit() {
    errorMsg = "";
    if (parameters.length === 0) {
      errorMsg = "Add at least one fit parameter.";
      return;
    }

    // Build a minimal model placeholder.
    // In a real workflow the model comes from the notebook store;
    // here we provide a skeleton that the backend will populate.
    const model: TheoreticalModel = {
      name: "Fit Model",
      description: "",
      fields: parameters.map((p) => ({
        id: p.field_id,
        name: p.name,
        symbol: p.field_id,
        mass: (p.min + p.max) / 2,
        width: 0,
        quantum_numbers: {
          electric_charge: 0,
          weak_isospin: 0,
          hypercharge: 0,
          baryon_number: 0,
          lepton_numbers: { electron: 0, muon: 0, tau: 0 },
          spin: 0,
          parity: "Even",
          charge_conjugation: "Even",
          color: "Singlet",
          weak_multiplet: "Singlet",
        },
        interactions: [],
      })),
      terms: [],
      vertex_factors: [],
      propagators: [],
    };

    const request: McmcFitRequest = {
      model,
      config: {
        parameters,
        gaussian_constraints: constraints,
        n_walkers: nWalkers,
        n_steps: nSteps,
        burn_in: burnIn,
      },
    };

    try {
      await startMcmcFit(request);
      isRunning = true;
      startPolling();
    } catch (e: unknown) {
      errorMsg = e instanceof Error ? e.message : String(e);
    }
  }

  async function stopFit() {
    try {
      await stopMcmcFit();
    } catch (e: unknown) {
      errorMsg = e instanceof Error ? e.message : String(e);
    }
    stopPolling();
    isRunning = false;
    // Final poll with samples
    await pollStatus(true);
  }

  function startPolling() {
    stopPolling();
    pollTimer = setInterval(() => pollStatus(false), 500);
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  }

  async function pollStatus(includeSamples: boolean) {
    try {
      const result: McmcFitStatus = await getMcmcStatus(includeSamples);
      status = result.status;
      paramNames = result.param_names;
      if (result.flat_samples) {
        flatSamples = result.flat_samples;
      }
      if (!result.status.running) {
        isRunning = false;
        stopPolling();
        // Get final samples
        if (!flatSamples) {
          const final = await getMcmcStatus(true);
          flatSamples = final.flat_samples;
        }
        drawCornerPlot();
      }
    } catch {
      // Ignore poll errors
    }
  }

  // === Corner plot rendering ===
  function drawCornerPlot() {
    if (!cornerCanvas || !flatSamples || flatSamples.length === 0) return;

    const ctx = cornerCanvas.getContext("2d");
    if (!ctx) return;

    const nDim = paramNames.length;
    if (nDim === 0) return;

    const padding = 40;
    const cellSize = Math.min(
      (cornerCanvas.width - padding * 2) / nDim,
      (cornerCanvas.height - padding * 2) / nDim,
      200,
    );

    cornerCanvas.width = padding * 2 + cellSize * nDim;
    cornerCanvas.height = padding * 2 + cellSize * nDim;

    ctx.fillStyle = "#1e1e2e";
    ctx.fillRect(0, 0, cornerCanvas.width, cornerCanvas.height);

    // Compute bounds per parameter
    const mins: number[] = [];
    const maxs: number[] = [];
    for (let d = 0; d < nDim; d++) {
      let lo = Infinity;
      let hi = -Infinity;
      for (const sample of flatSamples) {
        if (sample[d] < lo) lo = sample[d];
        if (sample[d] > hi) hi = sample[d];
      }
      const margin = (hi - lo) * 0.05 || 1;
      mins.push(lo - margin);
      maxs.push(hi + margin);
    }

    // Draw cells
    for (let row = 0; row < nDim; row++) {
      for (let col = 0; col <= row; col++) {
        const x0 = padding + col * cellSize;
        const y0 = padding + row * cellSize;

        // Cell border
        ctx.strokeStyle = "#444";
        ctx.strokeRect(x0, y0, cellSize, cellSize);

        if (row === col) {
          // Diagonal: 1D histogram
          draw1DHistogram(ctx, flatSamples!, row, x0, y0, cellSize, mins[row], maxs[row]);
        } else {
          // Off-diagonal: 2D scatter
          draw2DScatter(
            ctx,
            flatSamples!,
            col,
            row,
            x0,
            y0,
            cellSize,
            mins[col],
            maxs[col],
            mins[row],
            maxs[row],
          );
        }
      }
    }

    // Axis labels
    ctx.fillStyle = "#cdd6f4";
    ctx.font = "11px monospace";
    ctx.textAlign = "center";
    for (let d = 0; d < nDim; d++) {
      // Bottom labels
      ctx.fillText(
        paramNames[d] || `p${d}`,
        padding + d * cellSize + cellSize / 2,
        cornerCanvas.height - 8,
      );
      // Left labels (rotated)
      ctx.save();
      ctx.translate(12, padding + d * cellSize + cellSize / 2);
      ctx.rotate(-Math.PI / 2);
      ctx.fillText(paramNames[d] || `p${d}`, 0, 0);
      ctx.restore();
    }
  }

  function draw1DHistogram(
    ctx: CanvasRenderingContext2D,
    samples: number[][],
    dim: number,
    x0: number,
    y0: number,
    size: number,
    lo: number,
    hi: number,
  ) {
    const nBins = 30;
    const binWidth = (hi - lo) / nBins;
    const counts = new Array(nBins).fill(0);

    for (const s of samples) {
      const bin = Math.floor((s[dim] - lo) / binWidth);
      if (bin >= 0 && bin < nBins) counts[bin]++;
    }

    const maxCount = Math.max(...counts, 1);
    const barW = size / nBins;
    const inset = 4;

    ctx.fillStyle = "rgba(137, 180, 250, 0.6)";
    for (let i = 0; i < nBins; i++) {
      const barH = ((size - inset * 2) * counts[i]) / maxCount;
      ctx.fillRect(
        x0 + i * barW,
        y0 + size - inset - barH,
        barW - 1,
        barH,
      );
    }

    // Mean line
    let mean = 0;
    for (const s of samples) mean += s[dim];
    mean /= samples.length;
    const meanX = x0 + ((mean - lo) / (hi - lo)) * size;
    ctx.strokeStyle = "#f38ba8";
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(meanX, y0 + inset);
    ctx.lineTo(meanX, y0 + size - inset);
    ctx.stroke();
    ctx.lineWidth = 1;
  }

  function draw2DScatter(
    ctx: CanvasRenderingContext2D,
    samples: number[][],
    dimX: number,
    dimY: number,
    x0: number,
    y0: number,
    size: number,
    xLo: number,
    xHi: number,
    yLo: number,
    yHi: number,
  ) {
    // Draw as a 2D density heatmap
    const nBins = 25;
    const grid = new Array(nBins * nBins).fill(0);

    for (const s of samples) {
      const bx = Math.floor(((s[dimX] - xLo) / (xHi - xLo)) * nBins);
      const by = Math.floor(((s[dimY] - yLo) / (yHi - yLo)) * nBins);
      if (bx >= 0 && bx < nBins && by >= 0 && by < nBins) {
        grid[by * nBins + bx]++;
      }
    }

    const maxVal = Math.max(...grid, 1);
    const cellW = size / nBins;

    for (let iy = 0; iy < nBins; iy++) {
      for (let ix = 0; ix < nBins; ix++) {
        const val = grid[iy * nBins + ix];
        if (val === 0) continue;
        const frac = val / maxVal;
        // Blue-to-yellow colormap
        const r = Math.floor(frac * 250);
        const g = Math.floor(frac * 200);
        const b = Math.floor(250 - frac * 200);
        ctx.fillStyle = `rgb(${r},${g},${b})`;
        ctx.fillRect(
          x0 + ix * cellW,
          y0 + (nBins - 1 - iy) * cellW,
          cellW,
          cellW,
        );
      }
    }
  }

  onDestroy(() => {
    stopPolling();
  });
</script>

<div class="global-fit-dashboard">
  <h2>CFG Global Fit Dashboard</h2>

  <!-- Parameter Table -->
  <section class="params-section">
    <h3>Fit Parameters</h3>
    <table class="fit-table">
      <thead>
        <tr>
          <th>Name</th>
          <th>Field ID</th>
          <th>Property</th>
          <th>Min</th>
          <th>Max</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each parameters as p, i}
          <tr>
            <td><input bind:value={p.name} class="input-sm" /></td>
            <td><input bind:value={p.field_id} class="input-sm" /></td>
            <td>
              <select bind:value={p.property} class="input-sm">
                <option value="Mass">Mass</option>
                <option value="Width">Width</option>
              </select>
            </td>
            <td><SpireNumberInput bind:value={p.min} step={0.1} ariaLabel="Parameter minimum" /></td>
            <td><SpireNumberInput bind:value={p.max} step={0.1} ariaLabel="Parameter maximum" /></td>
            <td>
              <button class="btn-remove" on:click={() => removeParameter(i)}>✕</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
    <button class="btn-add" on:click={addParameter}>+ Add Parameter</button>
  </section>

  <!-- Constraint Table -->
  <section class="constraints-section">
    <h3>Gaussian Constraints</h3>
    <table class="fit-table">
      <thead>
        <tr>
          <th>Name</th>
          <th>Field ID</th>
          <th>Observed</th>
          <th>σ</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each constraints as c, i}
          <tr>
            <td><input bind:value={c.name} class="input-sm" /></td>
            <td><input bind:value={c.field_id} class="input-sm" /></td>
            <td><SpireNumberInput bind:value={c.observed} step={0.1} ariaLabel="Observed value" /></td>
            <td><SpireNumberInput bind:value={c.sigma} step={0.1} ariaLabel="Constraint sigma" /></td>
            <td>
              <button class="btn-remove" on:click={() => removeConstraint(i)}>✕</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
    <button class="btn-add" on:click={addConstraint}>+ Add Constraint</button>
  </section>

  <!-- MCMC Settings & Controls -->
  <section class="mcmc-controls">
    <h3>MCMC Settings</h3>
    <div class="settings-row">
      <label>
        Walkers
        <SpireNumberInput bind:value={nWalkers} min={4} step={2} ariaLabel="Number of walkers" />
      </label>
      <label>
        Steps
        <SpireNumberInput bind:value={nSteps} min={100} step={100} ariaLabel="MCMC steps" />
      </label>
      <label>
        Burn-in
        <SpireNumberInput bind:value={burnIn} min={0} step={100} ariaLabel="Burn-in steps" />
      </label>
    </div>

    <div class="action-row">
      {#if isRunning}
        <button class="btn-stop" on:click={stopFit}>⏹ Stop</button>
      {:else}
        <button class="btn-start" on:click={startFit}>▶ Start Fit</button>
      {/if}
    </div>

    {#if errorMsg}
      <div class="error-msg">{errorMsg}</div>
    {/if}
  </section>

  <!-- Status -->
  {#if status}
    <section class="status-section">
      <h3>Status</h3>
      <div class="status-grid">
        <span>Step:</span>
        <span>{status.current_step} / {status.total_steps}</span>
        <span>Acceptance:</span>
        <span>{(status.acceptance_fraction * 100).toFixed(1)}%</span>
        <span>State:</span>
        <span>
          {#if status.running}
            <span class="badge running">Running</span>
          {:else if status.stopped}
            <span class="badge stopped">Stopped</span>
          {:else}
            <span class="badge done">Done</span>
          {/if}
        </span>
      </div>
      {#if status.total_steps > 0}
        <div class="progress-bar">
          <div
            class="progress-fill"
            style="width: {(status.current_step / status.total_steps) * 100}%"
          ></div>
        </div>
      {/if}
    </section>
  {/if}

  <!-- Corner Plot -->
  <section class="corner-section">
    <h3>Corner Plot</h3>
    <canvas
      bind:this={cornerCanvas}
      width="600"
      height="600"
      class="corner-canvas"
    ></canvas>
    {#if flatSamples && flatSamples.length > 0}
      <p class="sample-count">{flatSamples.length.toLocaleString()} samples</p>
    {:else if !isRunning}
      <p class="placeholder-text">Run a fit to see results</p>
    {/if}
  </section>
</div>

<style>
  .global-fit-dashboard {
    padding: 1rem;
    color: var(--text-primary, #cdd6f4);
    display: flex;
    flex-direction: column;
    gap: 1rem;
    height: 100%;
    overflow-y: auto;
    min-height: 0;
  }

  h2 {
    margin: 0 0 0.5rem;
    font-size: 1.1rem;
    color: var(--hl-accent, #89b4fa);
  }

  h3 {
    margin: 0 0 0.5rem;
    font-size: 0.9rem;
    color: var(--text-secondary, #a6adc8);
  }

  .fit-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8rem;
  }

  .fit-table th {
    text-align: left;
    padding: 4px 6px;
    border-bottom: 1px solid var(--border, #45475a);
    color: var(--text-secondary, #a6adc8);
    font-weight: 500;
  }

  .fit-table td {
    padding: 3px 4px;
  }

  .input-sm {
    background: var(--bg-secondary, #313244);
    border: 1px solid var(--border, #45475a);
    color: var(--text-primary, #cdd6f4);
    border-radius: 3px;
    padding: 3px 6px;
    font-size: 0.8rem;
    width: 100%;
    box-sizing: border-box;
  }

  .input-sm:focus {
    border-color: var(--hl-accent, #89b4fa);
    outline: none;
  }

  select.input-sm {
    cursor: pointer;
  }

  .btn-add {
    background: transparent;
    border: 1px dashed var(--border, #45475a);
    color: var(--hl-accent, #89b4fa);
    padding: 4px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8rem;
    margin-top: 4px;
  }

  .btn-add:hover {
    border-color: var(--hl-accent, #89b4fa);
    background: rgba(137, 180, 250, 0.1);
  }

  .btn-remove {
    background: transparent;
    border: none;
    color: var(--hl-error, #f38ba8);
    cursor: pointer;
    font-size: 0.9rem;
    padding: 2px 6px;
  }

  .settings-row {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .settings-row label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 0.8rem;
    color: var(--text-secondary, #a6adc8);
  }

  .action-row {
    margin-top: 8px;
    display: flex;
    gap: 0.5rem;
  }

  .btn-start {
    background: var(--hl-accent, #89b4fa);
    color: var(--bg-primary, #1e1e2e);
    border: none;
    padding: 6px 20px;
    border-radius: 4px;
    font-weight: 600;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .btn-start:hover {
    filter: brightness(1.1);
  }

  .btn-stop {
    background: var(--hl-error, #f38ba8);
    color: var(--bg-primary, #1e1e2e);
    border: none;
    padding: 6px 20px;
    border-radius: 4px;
    font-weight: 600;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .error-msg {
    color: var(--hl-error, #f38ba8);
    font-size: 0.8rem;
    margin-top: 4px;
  }

  .status-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 2px 12px;
    font-size: 0.8rem;
  }

  .badge {
    padding: 1px 8px;
    border-radius: 3px;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .badge.running {
    background: rgba(166, 227, 161, 0.2);
    color: #a6e3a1;
  }

  .badge.stopped {
    background: rgba(243, 139, 168, 0.2);
    color: #f38ba8;
  }

  .badge.done {
    background: rgba(137, 180, 250, 0.2);
    color: #89b4fa;
  }

  .progress-bar {
    height: 6px;
    background: var(--bg-secondary, #313244);
    border-radius: 3px;
    margin-top: 6px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--hl-accent, #89b4fa);
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  .corner-canvas {
    width: 100%;
    max-width: 600px;
    height: auto;
    border: 1px solid var(--border, #45475a);
    border-radius: 4px;
  }

  .sample-count {
    font-size: 0.75rem;
    color: var(--text-secondary, #a6adc8);
    margin-top: 4px;
  }

  .placeholder-text {
    font-size: 0.8rem;
    color: var(--text-secondary, #a6adc8);
    font-style: italic;
  }
</style>
