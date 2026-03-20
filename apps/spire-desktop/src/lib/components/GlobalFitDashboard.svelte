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
  import Icon from "$lib/components/ui/Icon.svelte";
  import SpireNumberInput from "$lib/components/ui/SpireNumberInput.svelte";
  import {
    startMcmcFit,
    getMcmcStatus,
    stopMcmcFit,
    computeGlobalFit,
  } from "$lib/api";
  import type {
    FitParameter,
    FitProperty,
    GaussianConstraint,
    McmcFitRequest,
    McmcFitStatus,
    McmcStatus,
    TheoreticalModel,
    ObservableFitInput,
    GlobalObservableFitResult,
  } from "$lib/types/spire";
  import EditionLockBadge from "$lib/components/ui/EditionLockBadge.svelte";
  import { pdgIntegrationState } from "$lib/stores/pdgIntegrationStore";

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
  let globalFitError = "";
  let globalNParams = 0;
  let globalFitResult: GlobalObservableFitResult | null = null;
  let globalObservables: ObservableFitInput[] = [
    {
      observable: "pT",
      theory_bin_contents: [10, 20],
      theory_bin_edges: [0, 1, 2],
      exp_csv: "x,y,dy\n0.5,10,1\n1.5,20,1",
    },
  ];

  // Poll timer
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  // Corner plot
  let cornerCanvas: HTMLCanvasElement;

  function cssVar(name: string, fallback: string): string {
    if (typeof window === "undefined") return fallback;
    const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    return value || fallback;
  }

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

  function addGlobalObservable() {
    globalObservables = [
      ...globalObservables,
      {
        observable: `obs_${globalObservables.length + 1}`,
        theory_bin_contents: [1, 2],
        theory_bin_edges: [0, 1, 2],
        exp_csv: "x,y,dy\n0.5,1,1\n1.5,2,1",
      },
    ];
  }

  function removeGlobalObservable(i: number) {
    globalObservables = globalObservables.filter((_, idx) => idx !== i);
  }

  async function runGlobalFitSummary() {
    globalFitError = "";
    try {
      globalFitResult = await computeGlobalFit(globalObservables, Math.max(0, Math.floor(globalNParams)));
    } catch (e: unknown) {
      globalFitError = e instanceof Error ? e.message : String(e);
    }
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

    const bgSurface = cssVar("--color-bg-surface", "#1e1e2e");
    const border = cssVar("--color-border", "#444");
    const accent = cssVar("--color-accent", "#89b4fa");
    const error = cssVar("--color-error", "#f38ba8");
    const textPrimary = cssVar("--color-text-primary", "#cdd6f4");

    ctx.fillStyle = bgSurface;
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
        ctx.strokeStyle = border;
        ctx.strokeRect(x0, y0, cellSize, cellSize);

        if (row === col) {
          // Diagonal: 1D histogram
          draw1DHistogram(ctx, flatSamples!, row, x0, y0, cellSize, mins[row], maxs[row], accent, error);
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
    ctx.fillStyle = textPrimary;
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
    accent: string,
    error: string,
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

    ctx.fillStyle = accent;
    ctx.globalAlpha = 0.6;
    for (let i = 0; i < nBins; i++) {
      const barH = ((size - inset * 2) * counts[i]) / maxCount;
      ctx.fillRect(
        x0 + i * barW,
        y0 + size - inset - barH,
        barW - 1,
        barH,
      );
    }
    ctx.globalAlpha = 1;

    // Mean line
    let mean = 0;
    for (const s of samples) mean += s[dim];
    mean /= samples.length;
    const meanX = x0 + ((mean - lo) / (hi - lo)) * size;
    ctx.strokeStyle = error;
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
  <h2>Global Fit Dashboard</h2>

  <section class="status-section">
    <h3>PDG Provenance</h3>
    <EditionLockBadge edition={$pdgIntegrationState.editionLock} stale={$pdgIntegrationState.stale} mismatch={$pdgIntegrationState.mismatch} />
    <div class="status-grid">
      <span>Merge mode:</span><span>{$pdgIntegrationState.mergeMode}</span>
      <span>Bootstrap:</span><span>{$pdgIntegrationState.bootstrapPreset ?? "none"}</span>
      <span>Source arbitration:</span><span>{$pdgIntegrationState.sourceArbitration}</span>
      <span>Authority:</span><span>{$pdgIntegrationState.authoritativeSource ?? "unknown"}</span>
      <span>Last sync:</span><span>{$pdgIntegrationState.lastSyncAt ?? "-"}</span>
    </div>
  </section>

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
              <button class="btn-remove" on:click={() => removeParameter(i)}><Icon name="close" size={13} /></button>
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
              <button class="btn-remove" on:click={() => removeConstraint(i)}><Icon name="close" size={13} /></button>
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
        <button class="btn-stop" on:click={stopFit}><Icon name="stop" size={14} /> <span>Stop</span></button>
      {:else}
        <button class="btn-start" on:click={startFit}><Icon name="play" size={14} /> <span>Start Fit</span></button>
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
      <div class="status-cards">
        <div class="status-card">
          <span class="status-card-label">Step</span>
          <span class="status-card-value">{status.current_step} / {status.total_steps}</span>
        </div>
        <div class="status-card">
          <span class="status-card-label">Acceptance</span>
          <span class="status-card-value">{(status.acceptance_fraction * 100).toFixed(1)}%</span>
        </div>
      </div>
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

  <section class="status-section">
    <h3>Global Fit Summary</h3>
    {#each globalObservables as obs, i}
      <div class="settings-row">
        <label>
          Observable
          <input bind:value={obs.observable} class="input-sm" />
        </label>
        <label>
          Theory bins (JSON)
          <input
            class="input-sm"
            value={JSON.stringify(obs.theory_bin_contents)}
            on:change={(e) => {
              try {
                const parsed = JSON.parse((e.currentTarget as HTMLInputElement).value);
                obs.theory_bin_contents = Array.isArray(parsed) ? parsed.map(Number) : obs.theory_bin_contents;
                globalObservables = [...globalObservables];
              } catch {}
            }}
          />
        </label>
        <label>
          Theory edges (JSON)
          <input
            class="input-sm"
            value={JSON.stringify(obs.theory_bin_edges)}
            on:change={(e) => {
              try {
                const parsed = JSON.parse((e.currentTarget as HTMLInputElement).value);
                obs.theory_bin_edges = Array.isArray(parsed) ? parsed.map(Number) : obs.theory_bin_edges;
                globalObservables = [...globalObservables];
              } catch {}
            }}
          />
        </label>
        <button class="btn-remove" on:click={() => removeGlobalObservable(i)}><Icon name="close" size={13} /></button>
      </div>
      <textarea class="input-sm" rows="3" bind:value={obs.exp_csv}></textarea>
    {/each}
    <div class="action-row">
      <button class="btn-add" on:click={addGlobalObservable}>+ Add Observable</button>
      <label>
        N params
        <SpireNumberInput bind:value={globalNParams} min={0} step={1} ariaLabel="Global fit parameter count" />
      </label>
      <button class="btn-start" on:click={runGlobalFitSummary}><Icon name="play" size={14} /> <span>Compute Global χ²</span></button>
    </div>
    {#if globalFitError}
      <div class="error-msg">{globalFitError}</div>
    {/if}
    {#if globalFitResult}
      <div class="status-grid">
        <span>χ² global:</span><span>{globalFitResult.chi_square_global.toFixed(4)}</span>
        <span>Ndf total:</span><span>{globalFitResult.ndf_total}</span>
        <span>χ²/ndf:</span><span>{globalFitResult.chi_square_reduced.toFixed(4)}</span>
        <span>p-value:</span><span>{globalFitResult.p_value.toFixed(5)}</span>
      </div>
    {/if}
  </section>
</div>

<style>
  .global-fit-dashboard {
    padding: 0.75rem;
    color: var(--color-text-primary, var(--fg-primary));
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
    height: 100%;
    overflow-y: auto;
    min-height: 0;
  }

  h2 {
    margin: 0 0 0.35rem;
    font-size: 0.98rem;
    color: var(--fg-accent);
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.3rem;
  }

  h3 {
    margin: 0 0 0.5rem;
    font-size: 0.85rem;
    color: var(--fg-secondary);
  }

  .params-section,
  .constraints-section,
  .mcmc-controls,
  .status-section,
  .corner-section {
    background: color-mix(in srgb, var(--color-bg-elevated) 55%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-border) 80%, transparent);
    padding: 0.55rem;
    border-radius: var(--radius-md);
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
    color: var(--fg-secondary);
    font-weight: 500;
  }

  .fit-table td {
    padding: 3px 4px;
  }

  .input-sm {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 3px 6px;
    font-size: 0.8rem;
    width: 100%;
    box-sizing: border-box;
  }

  .input-sm:focus {
    border-color: var(--border-focus);
    outline: none;
  }

  select.input-sm {
    cursor: pointer;
  }

  .btn-add {
    background: transparent;
    border: 1px dashed var(--border);
    color: var(--fg-accent);
    padding: 4px 12px;
    cursor: pointer;
    font-size: 0.8rem;
    margin-top: 4px;
    border-radius: var(--radius-sm);
  }

  .btn-add:hover {
    border-color: var(--fg-accent);
    background: rgba(var(--color-accent-rgb), 0.1);
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
    color: var(--fg-secondary);
  }

  .action-row {
    margin-top: 8px;
    display: flex;
    gap: 0.5rem;
  }

  .btn-start {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    background: color-mix(in srgb, var(--color-accent) 14%, var(--color-bg-surface));
    color: var(--color-text-primary);
    border: 1px solid color-mix(in srgb, var(--color-accent) 65%, var(--color-border));
    border-radius: var(--radius-sm);
    padding: 6px 20px;
    font-weight: 600;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .btn-start:hover {
    background: color-mix(in srgb, var(--color-accent) 24%, var(--color-bg-surface));
    border-color: color-mix(in srgb, var(--color-accent) 90%, var(--color-border));
  }

  .btn-stop {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    background: color-mix(in srgb, var(--color-error) 20%, var(--color-bg-surface));
    color: var(--color-text-primary);
    border: 1px solid color-mix(in srgb, var(--color-error) 70%, var(--color-border));
    border-radius: var(--radius-sm);
    padding: 6px 20px;
    font-weight: 600;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .btn-stop:hover {
    background: color-mix(in srgb, var(--color-error) 30%, var(--color-bg-surface));
  }

  .error-msg {
    color: var(--hl-error, var(--color-error));
    font-size: 0.8rem;
    margin-top: 4px;
  }

  .status-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 2px 12px;
    font-size: 0.8rem;
  }

  .status-cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
    gap: 0.45rem;
    margin-bottom: 0.45rem;
  }

  .status-card {
    border: 1px solid color-mix(in srgb, var(--color-border) 70%, transparent);
    background: color-mix(in srgb, var(--color-bg-base) 78%, transparent);
    border-radius: var(--radius-sm);
    padding: 0.3rem 0.45rem;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .status-card-label {
    font-size: 0.66rem;
    color: var(--fg-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .status-card-value {
    font-size: 0.82rem;
    color: var(--fg-primary);
    font-family: var(--font-mono);
  }

  .badge {
    padding: 1px 8px;
    border-radius: 3px;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .badge.running {
    background: rgba(var(--color-success-rgb), 0.2);
    color: var(--color-success);
  }

  .badge.stopped {
    background: rgba(var(--color-error-rgb), 0.2);
    color: var(--color-error);
  }

  .badge.done {
    background: rgba(var(--color-accent-rgb), 0.2);
    color: var(--color-accent);
  }

  .progress-bar {
    height: 6px;
    background: var(--bg-inset);
    margin-top: 6px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--color-accent);
    transition: width 0.3s ease;
  }

  .corner-canvas {
    width: 100%;
    max-width: 600px;
    height: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }

  .sample-count {
    font-size: 0.75rem;
    color: var(--fg-secondary);
    margin-top: 4px;
  }

  .placeholder-text {
    font-size: 0.8rem;
    color: var(--fg-secondary);
    font-style: italic;
  }
</style>
