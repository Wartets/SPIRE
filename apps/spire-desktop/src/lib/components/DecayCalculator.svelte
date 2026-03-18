<!--
  SPIRE - Decay Calculator Widget (Phase 45)

  Interactive branching-ratio and partial-width calculator.  The user
  selects a massive particle from the loaded model, triggers the decay
  table computation, and views the results as a Chart.js doughnut chart
  plus a sortable data table.  An SLHA export button copies or downloads
  the DECAY block in standard SLHA format.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { get } from "svelte/store";
  import { isolateEvents } from "$lib/actions/widgetEvents";
  import { showContextMenu } from "$lib/stores/contextMenuStore";
  import { theoreticalModel, appendLog } from "$lib/stores/physicsStore";
  import { calculateDecayTable, exportDecaySlha } from "$lib/api";
  import { pipelineGraph, pipelineRunState } from "$lib/stores/pipelineGraphStore";
  import { registerCommand, unregisterCommand } from "$lib/core/services/CommandRegistry";
  import { publishWidgetInterop, widgetInteropState } from "$lib/stores/widgetInteropStore";
  import HoverDef from "$lib/components/ui/HoverDef.svelte";
  import type { TheoreticalModel, CalcDecayTable, CalcDecayChannel } from "$lib/types/spire";
  import {
    Chart,
    DoughnutController,
    ArcElement,
    Tooltip,
    Title,
    Legend,
  } from "chart.js";

  const logDecay = (message: string): void => {
    appendLog(message, { category: "Decay" });
  };

  // Register Chart.js components (tree-shakeable).
  Chart.register(DoughnutController, ArcElement, Tooltip, Title, Legend);

  // -------------------------------------------------------------------------
  // Colour palette for pie slices
  // -------------------------------------------------------------------------

  const PALETTE = [
    "#00d4ff", "#ff6b6b", "#51cf66", "#fcc419", "#cc5de8",
    "#ff922b", "#20c997", "#748ffc", "#f06595", "#ffe066",
    "#69db7c", "#4dabf7", "#e599f7", "#ffa94d", "#63e6be",
    "#91a7ff", "#ffd43b", "#ff8787", "#38d9a9", "#da77f2",
  ];

  // -------------------------------------------------------------------------
  // State
  // -------------------------------------------------------------------------

  let modelFromStore: TheoreticalModel | null = null;
  let pipelineModelOverride: TheoreticalModel | null = null;
  let model: TheoreticalModel | null = null;
  let massiveParticles: { id: string; name: string; mass: number; pdgCode: number }[] = [];
  let selectedIdx = 0;
  let particleFilter = "";
  let particleSort: "mass_desc" | "mass_asc" | "name" = "mass_desc";
  let preferredParticleIds: string[] = [];

  let computing = false;
  let errorMsg = "";
  let decayTable: CalcDecayTable | null = null;

  let chartCanvas: HTMLCanvasElement;
  let chartInstance: Chart | null = null;
  let interopUnsub: (() => void) | null = null;

  // -------------------------------------------------------------------------
  // Model subscription
  // -------------------------------------------------------------------------

  const unsubModel = theoreticalModel.subscribe((m) => {
    modelFromStore = m;
  });

  $: model = pipelineModelOverride ?? modelFromStore;

  $: buildParticleList(model);

  function isTheoreticalModelLike(value: unknown): value is TheoreticalModel {
    return (
      typeof value === "object"
      && value !== null
      && Array.isArray((value as { fields?: unknown }).fields)
    );
  }

  $: {
    let nextPipelineModel: TheoreticalModel | null = null;
    for (const [edgeId, payload] of Object.entries($pipelineRunState.edgePayloads)) {
      const edge = $pipelineGraph.edges[edgeId];
      if (!edge) continue;
      const source = $pipelineGraph.nodes[edge.sourceNodeId];
      if (source?.type !== "model_source") continue;
      if (isTheoreticalModelLike(payload)) {
        nextPipelineModel = payload;
        break;
      }
    }
    pipelineModelOverride = nextPipelineModel;
  }

  /** Rebuild the list of massive particles when the model changes. */
  function buildParticleList(m: TheoreticalModel | null): void {
    const previousId = massiveParticles[selectedIdx]?.id ?? null;
    if (!m) {
      massiveParticles = [];
      return;
    }

    // Deduplicate by keeping only particles with mass > 0 and width ≥ 0.
    // Assign PDG-like codes based on position (placeholder - real PDG codes
    // would come from the model metadata).
    const seen = new Set<string>();
    const result: typeof massiveParticles = [];
    for (let i = 0; i < m.fields.length; i++) {
      const f = m.fields[i];
      if (f.mass > 0 && !seen.has(f.id)) {
        seen.add(f.id);
        result.push({
          id: f.id,
          name: f.name || f.id,
          mass: f.mass,
          pdgCode: i + 1,      // placeholder
        });
      }
    }

    if (particleSort === "name") {
      result.sort((a, b) => a.name.localeCompare(b.name));
    } else if (particleSort === "mass_asc") {
      result.sort((a, b) => a.mass - b.mass);
    } else {
      result.sort((a, b) => b.mass - a.mass);
    }

    massiveParticles = result;

    if (previousId) {
      const idx = massiveParticles.findIndex((p) => p.id === previousId);
      if (idx >= 0) {
        selectedIdx = idx;
        return;
      }
    }

    if (selectedIdx >= massiveParticles.length) {
      selectedIdx = Math.max(0, massiveParticles.length - 1);
    }
  }

  $: visibleParticles = massiveParticles.filter((p) => {
    if (!particleFilter.trim()) return true;
    const q = particleFilter.trim().toLowerCase();
    return p.id.toLowerCase().includes(q) || p.name.toLowerCase().includes(q);
  });

  $: visibleOptions = visibleParticles.map((p) => ({
    particle: p,
    idx: massiveParticles.findIndex((m) => m.id === p.id),
  })).filter((o) => o.idx >= 0);

  // -------------------------------------------------------------------------
  // Core logic
  // -------------------------------------------------------------------------

  async function computeDecay(): Promise<void> {
    const m = model;
    if (!m || massiveParticles.length === 0) {
      errorMsg = "Load a theoretical model first.";
      return;
    }

    const target = massiveParticles[selectedIdx];
    if (!target) return;

    computing = true;
    errorMsg = "";
    decayTable = null;

    try {
      logDecay(`[DecayCalc] Computing decay table for ${target.name} (${target.id})…`);
      const table = await calculateDecayTable(m, target.id);
      decayTable = table;
      logDecay(
        `[DecayCalc] ${target.name}: Γ_total = ${table.total_width.toExponential(4)} GeV, ` +
        `${table.channels.length} channel(s), τ = ${table.lifetime_seconds.toExponential(3)} s`,
      );

      // Render chart after DOM update.
      await tick();
      renderChart(table);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      errorMsg = msg;
      logDecay(`[DecayCalc] Error: ${msg}`);
    } finally {
      computing = false;
    }
  }

  /** Svelte tick - wait for DOM update. */
  function tick(): Promise<void> {
    return new Promise((r) => setTimeout(r, 0));
  }

  // -------------------------------------------------------------------------
  // Chart rendering
  // -------------------------------------------------------------------------

  function renderChart(table: CalcDecayTable): void {
    if (chartInstance) {
      chartInstance.destroy();
      chartInstance = null;
    }
    if (!chartCanvas || table.channels.length === 0) return;

    const labels = table.channels.map((ch) => ch.final_state_names.join(" + "));
    const data = table.channels.map((ch) => ch.branching_ratio * 100);
    const colours = table.channels.map((_, i) => PALETTE[i % PALETTE.length]);

    chartInstance = new Chart(chartCanvas, {
      type: "doughnut",
      data: {
        labels,
        datasets: [
          {
            data,
            backgroundColor: colours,
            borderColor: "rgba(0,0,0,0.6)",
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
            text: `${table.parent_name} Branching Ratios`,
            color: "var(--color-text-primary)",
            font: { size: 13, weight: "bold" },
          },
          legend: {
            position: "right",
            labels: {
              color: "var(--color-text-muted)",
              font: { size: 11, family: "'JetBrains Mono', 'Fira Code', monospace" },
              boxWidth: 12,
              padding: 6,
            },
          },
          tooltip: {
            callbacks: {
              label: (ctx) => {
                const ch = table.channels[ctx.dataIndex];
                return ` BR = ${(ch.branching_ratio * 100).toFixed(4)}%  |  Γ = ${ch.partial_width.toExponential(4)} GeV`;
              },
            },
          },
        },
      },
    });
  }

  // -------------------------------------------------------------------------
  // SLHA Export
  // -------------------------------------------------------------------------

  async function copySlha(): Promise<void> {
    const m = model;
    if (!m || !decayTable) return;
    const target = massiveParticles[selectedIdx];
    if (!target) return;

    try {
      const slha = await exportDecaySlha(m, target.id, target.pdgCode);
      await navigator.clipboard.writeText(slha);
      logDecay("[DecayCalc] SLHA DECAY block copied to clipboard.");
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      logDecay(`[DecayCalc] SLHA copy failed: ${msg}`);
    }
  }

  async function downloadSlha(): Promise<void> {
    const m = model;
    if (!m || !decayTable) return;
    const target = massiveParticles[selectedIdx];
    if (!target) return;

    try {
      const slha = await exportDecaySlha(m, target.id, target.pdgCode);
      const blob = new Blob([slha], { type: "text/plain" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${target.id}_decay.slha`;
      a.click();
      URL.revokeObjectURL(url);
      logDecay(`[DecayCalc] SLHA file downloaded: ${target.id}_decay.slha`);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      logDecay(`[DecayCalc] SLHA download failed: ${msg}`);
    }
  }

  // -------------------------------------------------------------------------
  // Formatting helpers
  // -------------------------------------------------------------------------

  function fmtWidth(w: number): string {
    if (w === 0) return "0";
    if (w < 1e-6 || w >= 1e4) return w.toExponential(4);
    return w.toPrecision(5);
  }

  function fmtBR(br: number): string {
    return (br * 100).toFixed(4) + "%";
  }

  function fmtLifetime(t: number): string {
    if (t <= 0) return "stable";
    if (t < 1e-20) return t.toExponential(3) + " s";
    if (t < 1e-6) return (t * 1e9).toFixed(2) + " ns";
    if (t < 1) return (t * 1e6).toFixed(2) + " µs";
    return t.toFixed(4) + " s";
  }

  // -------------------------------------------------------------------------
  // Lifecycle
  // -------------------------------------------------------------------------

  onMount(() => {
    interopUnsub = widgetInteropState.subscribe((state) => {
      if (computing || massiveParticles.length === 0) return;

      const modelPayload = state.model?.payload as { modelName?: string } | undefined;
      if (modelPayload?.modelName) {
        // Model updates are already reflected via store subscription.
        // This branch intentionally keeps the interop channel warm for future routing.
      }

      const reactionPayload = state.reaction?.payload as
        | { initialState?: string[]; finalState?: string[] }
        | undefined;
      if (reactionPayload?.initialState?.length || reactionPayload?.finalState?.length) {
        const merged = [
          ...(reactionPayload.initialState ?? []),
          ...(reactionPayload.finalState ?? []),
        ];
        preferredParticleIds = merged
          .map((s) => String(s).trim().toLowerCase())
          .filter((s, i, arr) => s.length > 0 && arr.indexOf(s) === i)
          .slice(0, 6);

        const matchIdx = massiveParticles.findIndex((p) => {
          const pid = p.id.toLowerCase();
          const pname = p.name.toLowerCase();
          return preferredParticleIds.some((candidate) =>
            pid.includes(candidate) || pname.includes(candidate),
          );
        });
        if (matchIdx >= 0) selectedIdx = matchIdx;
      }
    });

    registerCommand({
      id: "spire.calculateDecayTable",
      title: "Calculate Decay Table",
      category: "Decay",
      execute: computeDecay,
    });
  });

  onDestroy(() => {
    interopUnsub?.();
    unsubModel();
    unregisterCommand("spire.calculateDecayTable");
    if (chartInstance) {
      chartInstance.destroy();
      chartInstance = null;
    }
  });

  $: publishWidgetInterop("decay_calculator", {
    selectedParticle: massiveParticles[selectedIdx]?.id ?? null,
    selectedParticleName: massiveParticles[selectedIdx]?.name ?? null,
    computing,
    channels: decayTable?.channels.length ?? 0,
    totalWidth: decayTable?.total_width ?? null,
    thresholdEnergy: massiveParticles[selectedIdx]?.mass ?? null,
    lifetimeSeconds: decayTable?.lifetime_seconds ?? null,
  });

  function selectPreferredParticle(token: string): void {
    const q = token.trim().toLowerCase();
    if (!q) return;
    const idx = massiveParticles.findIndex((p) =>
      p.id.toLowerCase().includes(q) || p.name.toLowerCase().includes(q),
    );
    if (idx >= 0) {
      selectedIdx = idx;
    }
  }
</script>

<div class="decay-widget" use:isolateEvents>
  <h3 class="widget-title">
    <HoverDef term="decay_table">Decay Calculator</HoverDef>
  </h3>

  <!-- Configuration -->
  <div class="config-panel">
    <div class="config-top-row">
      <div class="field grow">
        <label for="decay-filter">Filter particles</label>
        <input
          id="decay-filter"
          type="text"
          bind:value={particleFilter}
          placeholder="Search by id or name"
        />
      </div>
      <div class="field compact">
        <label for="decay-sort">Sort</label>
        <select id="decay-sort" bind:value={particleSort}>
          <option value="mass_desc">Mass ↓</option>
          <option value="mass_asc">Mass ↑</option>
          <option value="name">Name</option>
        </select>
      </div>
    </div>

    <div class="field">
      <label for="decay-particle">
        <HoverDef term="parent_particle">Parent Particle</HoverDef>
      </label>
      <select id="decay-particle" bind:value={selectedIdx} disabled={computing}>
        {#each visibleOptions as option}
          <option value={option.idx}>{option.particle.name} ({option.particle.id}) - {option.particle.mass.toPrecision(4)} GeV</option>
        {/each}
        {#if visibleOptions.length === 0}
          <option value={selectedIdx} disabled>No particles match filter</option>
        {/if}
        {#if massiveParticles.length === 0}
          <option value={0} disabled>Load a model first</option>
        {/if}
      </select>
    </div>

    {#if preferredParticleIds.length > 0}
      <div class="preferred-row">
        <span class="preferred-label">From reaction</span>
        {#each preferredParticleIds as token}
          <button class="preferred-chip" on:click={() => selectPreferredParticle(token)}>{token}</button>
        {/each}
      </div>
    {/if}

    <button class="run-btn" on:click={computeDecay} disabled={computing || massiveParticles.length === 0}>
      {#if computing}
        <span class="spinner"></span> Computing…
      {:else}
        Compute Decays
      {/if}
    </button>

    {#if errorMsg}
      <div class="error-msg">{errorMsg}</div>
    {/if}
  </div>

  <!-- Results -->
  <div class="results-area">
    {#if decayTable}
      <!-- Summary banner -->
      <div class="summary-banner">
        <span class="summary-item">
          <strong>Γ_total</strong> = {fmtWidth(decayTable.total_width)} GeV
        </span>
        <span class="summary-item">
          <strong>τ</strong> = {fmtLifetime(decayTable.lifetime_seconds)}
        </span>
        <span class="summary-item">
          <strong>Channels</strong>: {decayTable.channels.length}
        </span>
      </div>

      <!-- Doughnut Chart -->
      {#if decayTable.channels.length > 0}
        <div class="chart-container">
          <canvas bind:this={chartCanvas}></canvas>
        </div>
      {/if}

      <!-- Data Table -->
      <div class="data-table-wrap">
        <table>
          <thead>
            <tr>
              <th class="left"
                use:tooltip={{ text: "Final-state particles of this decay channel" }}>
                Channel
              </th>
              <th use:tooltip={{ text: "Partial decay width Γ_i for this channel [GeV]; Γ_total = Σ Γ_i" }}>
                Γ<sub>partial</sub> (GeV)
              </th>
              <th use:tooltip={{ text: "Branching ratio BR_i = Γ_i / Γ_total (dimensionless fraction)" }}>
                BR
              </th>
              <th use:tooltip={{ text: "Interaction vertex identifier from the loaded model" }}>
                Vertex
              </th>
            </tr>
          </thead>
          <tbody>
            {#each decayTable.channels as ch, i}
              <tr on:contextmenu|preventDefault|stopPropagation={(e) => {
                  showContextMenu(e.clientX, e.clientY, [
                    { type: "action", id: "copy-channel", label: `Copy channel: ${ch.final_state_names.join(" + ")}`, icon: "CP",
                      action: () => navigator.clipboard.writeText(ch.final_state_names.join(" + ")) },
                    { type: "action", id: "copy-br", label: `Copy BR: ${fmtBR(ch.branching_ratio)}`, icon: "BR",
                      action: () => navigator.clipboard.writeText(fmtBR(ch.branching_ratio)) },
                    { type: "action", id: "copy-width", label: `Copy Γ: ${fmtWidth(ch.partial_width)} GeV`, icon: "Γ",
                      action: () => navigator.clipboard.writeText(`${fmtWidth(ch.partial_width)} GeV`) },
                    { type: "separator", id: "sep-1" },
                    { type: "action", id: "send-scanner", label: "Send BR to Parameter Scanner", icon: "→",
                      action: () => {
                        const val = (ch.branching_ratio * 100).toFixed(4);
                        navigator.clipboard.writeText(val);
                        logDecay(`[DecayCalc] Copied BR ${val}% for ${ch.final_state_names.join("+")} to clipboard.`);
                      }},
                    { type: "action", id: "copy-slha-line", label: "Copy as SLHA line", icon: "SL",
                      action: () => {
                        const line = `   ${ch.branching_ratio.toExponential(6)}   ${ch.final_state_names.length}   ${ch.final_state_names.join("   ")}   # ${ch.vertex_id}`;
                        navigator.clipboard.writeText(line);
                      }},
                  ]);
                }}>
                <td class="left">
                  <span class="colour-dot" style="background:{PALETTE[i % PALETTE.length]}"></span>
                  {ch.final_state_names.join(" + ")}
                </td>
                <td>{fmtWidth(ch.partial_width)}</td>
                <td>{fmtBR(ch.branching_ratio)}</td>
                <td class="mono">{ch.vertex_id}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      <!-- SLHA Export -->
      <div class="export-row">
        <button class="export-btn" on:click={copySlha} use:tooltip={{ text: "Copy SLHA DECAY block to clipboard" }}>
          Copy SLHA
        </button>
        <button class="export-btn" on:click={downloadSlha} use:tooltip={{ text: "Download SLHA DECAY block as .slha file" }}>
          Download .slha
        </button>
      </div>

    {:else if !computing}
      <div class="empty-state">
        <p>Select a parent particle above, then press <strong>Compute Decays</strong>.</p>
        <p class="hint">The engine discovers all kinematically allowed 2-body channels from the loaded model.</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .decay-widget {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    height: 100%;
    overflow-y: auto;
    padding: 0.75rem;
    font-size: 0.8rem;
    color: var(--color-text-primary);
    box-sizing: border-box;
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
    gap: 0.5rem;
  }

  .config-top-row {
    display: flex;
    gap: 0.5rem;
    align-items: end;
    flex-wrap: wrap;
  }

  .field.grow {
    flex: 1;
    min-width: 12rem;
  }

  .field.compact {
    min-width: 7.5rem;
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

  input[type="text"]:focus,
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
    transition: opacity 0.15s, background 0.15s, border-color 0.15s;
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

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .error-msg {
    color: var(--color-error);
    font-size: 0.75rem;
    padding: 0.3rem 0.5rem;
    background: rgba(var(--color-error-rgb), 0.1);
    border-radius: var(--radius-sm);
    border-left: 3px solid var(--color-error);
  }

  /* ── Summary Banner ───────────────────────────────────────────── */

  .summary-banner {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(8rem, 1fr));
    gap: 0.5rem;
    padding: 0.45rem 0.6rem;
    background: rgba(var(--color-accent-rgb), 0.06);
    border: 1px solid rgba(var(--color-accent-rgb), 0.15);
    border-radius: var(--radius-sm);
    font-size: 0.76rem;
  }

  .summary-item strong {
    color: var(--color-accent);
  }

  /* ── Chart ─────────────────────────────────────────────────────── */

  .results-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    min-height: 0;
  }

  .chart-container {
    position: relative;
    height: 240px;
    min-height: 180px;
  }

  .chart-container canvas {
    width: 100% !important;
    height: 100% !important;
  }

  /* ── Data Table ────────────────────────────────────────────────── */

  .data-table-wrap {
    max-height: 240px;
    overflow-y: auto;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }

  .preferred-row {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex-wrap: wrap;
  }

  .preferred-label {
    font-size: 0.68rem;
    color: var(--color-text-muted);
  }

  .preferred-chip {
    border: 1px solid var(--color-border);
    background: color-mix(in srgb, var(--color-bg-surface) 88%, transparent);
    color: var(--color-text-muted);
    font-size: 0.68rem;
    padding: 0.14rem 0.45rem;
    cursor: pointer;
  }

  .preferred-chip:hover {
    color: var(--color-text-primary);
    border-color: color-mix(in srgb, var(--color-accent) 55%, var(--color-border));
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

  th.left,
  td.left {
    text-align: left;
  }

  td {
    text-align: right;
    padding: 0.25rem 0.5rem;
    color: var(--color-text-primary);
    border-bottom: 1px solid rgba(255, 255, 255, 0.03);
  }

  td.mono {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 0.68rem;
    color: var(--color-text-muted);
  }

  .colour-dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    margin-right: 0.35rem;
    vertical-align: middle;
  }

  tr:hover td {
    background: rgba(var(--color-accent-rgb), 0.05);
  }

  /* ── Export ─────────────────────────────────────────────────────── */

  .export-row {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .export-btn {
    padding: 0.35rem 0.75rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-muted);
    font-size: 0.75rem;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
  }

  .export-btn:hover {
    background: rgba(var(--color-text-primary-rgb), 0.06);
    border-color: rgba(var(--color-accent-rgb), 0.3);
    color: #eee;
  }

  /* ── Empty State ───────────────────────────────────────────────── */

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    height: 200px;
    color: var(--color-text-muted);
    text-align: center;
  }

  .empty-state p {
    margin: 0;
  }

  .hint {
    font-size: 0.68rem;
    color: var(--color-text-muted);
    font-style: italic;
  }
</style>
