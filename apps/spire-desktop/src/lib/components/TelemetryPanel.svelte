<!--
  SPIRE - Telemetry Panel

  Live performance dashboard widget.  Displays kernel-level profiling
  data attached to computation results (cross-section, topology
  generation, amplitude derivation, analysis).

  ## Sections

  1. **Summary Strip** - total time, peak memory, thread count.
  2. **Stage Timings** - horizontal bar chart, fully dynamic (renders
     whatever keys the ComputeProfile provides).
  3. **Convergence Plot** - SVG polyline of relative error vs event
     count for Monte Carlo computations.
  4. **History** - scrollable list of the last 20 profiled runs.
-->
<script lang="ts">
  import { tooltip } from "$lib/actions/tooltip";
  import {
    latestProfile,
    latestLabel,
    stageTimings,
    convergenceData,
    historicalProfiles,
    profileCount,
    clearTelemetry,
    pushProfile,
  } from "$lib/core/services/TelemetryService";
  import { publishWidgetInterop } from "$lib/stores/widgetInteropStore";
  import type { ProfileEntry } from "$lib/core/services/TelemetryService";
  import { pdgIntegrationState } from "$lib/stores/pdgIntegrationStore";
  import EditionLockBadge from "$lib/components/ui/EditionLockBadge.svelte";

  // ── Formatting Helpers ──────────────────────────────────

  function fmtMs(ms: number): string {
    if (ms >= 1000) return `${(ms / 1000).toFixed(2)} s`;
    if (ms >= 1) return `${ms.toFixed(1)} ms`;
    return `${(ms * 1000).toFixed(0)} µs`;
  }

  function fmtMb(mb: number): string {
    if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`;
    if (mb >= 1) return `${mb.toFixed(1)} MB`;
    return `${(mb * 1024).toFixed(0)} KB`;
  }

  function fmtTimestamp(iso: string): string {
    const d = new Date(iso);
    return d.toLocaleTimeString(undefined, {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  }

  // ── Bar Chart Scaling ───────────────────────────────────

  function barWidth(value: number, max: number): string {
    if (max <= 0) return "0%";
    return `${Math.max(2, (value / max) * 100).toFixed(1)}%`;
  }

  // ── SVG Convergence Path ────────────────────────────────

  const SVG_W = 320;
  const SVG_H = 140;
  const SVG_PAD = 30;

  $: convergencePath = (() => {
    const pts = $convergenceData;
    if (pts.length < 2) return "";

    const xMin = Math.log10(Math.max(pts[0].events, 1));
    const xMax = Math.log10(Math.max(pts[pts.length - 1].events, 1));
    const yMin = Math.min(...pts.map((p) => p.error));
    const yMax = Math.max(...pts.map((p) => p.error));
    const xRange = xMax - xMin || 1;
    const yRange = yMax - yMin || 1;

    return pts
      .map((p, i) => {
        const x =
          SVG_PAD +
          ((Math.log10(Math.max(p.events, 1)) - xMin) / xRange) *
            (SVG_W - 2 * SVG_PAD);
        const y =
          SVG_PAD +
          (1 - (p.error - yMin) / yRange) * (SVG_H - 2 * SVG_PAD);
        return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
  })();

  // ── 1/√N Reference Curve ───────────────────────────────

  $: refPath = (() => {
    const pts = $convergenceData;
    if (pts.length < 2) return "";

    const xMin = Math.log10(Math.max(pts[0].events, 1));
    const xMax = Math.log10(Math.max(pts[pts.length - 1].events, 1));
    const yMin = Math.min(...pts.map((p) => p.error));
    const yMax = Math.max(...pts.map((p) => p.error));
    const xRange = xMax - xMin || 1;
    const yRange = yMax - yMin || 1;

    // Anchor to the first data point, scale as 1/√N
    const e0 = pts[0].error;
    const n0 = pts[0].events;

    const steps = 20;
    const segments: string[] = [];
    for (let i = 0; i <= steps; i++) {
      const logN = xMin + (i / steps) * xRange;
      const n = Math.pow(10, logN);
      const refError = e0 * Math.sqrt(n0 / n);
      const x = SVG_PAD + (i / steps) * (SVG_W - 2 * SVG_PAD);
      const y =
        SVG_PAD +
        (1 - (refError - yMin) / yRange) * (SVG_H - 2 * SVG_PAD);
      segments.push(`${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`);
    }
    return segments.join(" ");
  })();

  // ── History Selection ───────────────────────────────────

  function selectHistorical(entry: ProfileEntry): void {
    pushProfile(entry.profile, entry.label);
  }

  // ── Bar Max ─────────────────────────────────────────────

  $: barMax = $stageTimings.length > 0 ? $stageTimings[0][1] : 1;

  $: publishWidgetInterop("telemetry", {
    profileCount: $profileCount,
    label: $latestLabel || null,
    totalMs: $latestProfile?.total_time_ms ?? null,
    peakMemoryMb: $latestProfile?.peak_memory_mb ?? null,
    threadsUsed: $latestProfile?.threads_used ?? null,
    stageCount: $stageTimings.length,
    hasConvergence: $convergenceData.length >= 2,
  });
</script>

<div class="telemetry-panel">
  <!-- Empty State -->
  {#if !$latestProfile}
    <div class="empty-state">
      <p>No profiling data yet.</p>
      <p>Run a computation to collect performance metrics.</p>
    </div>
  {:else}
    <!-- Header -->
    <div class="section-header">
      <span class="section-title">{$latestLabel || "Profile"}</span>
      <button class="ref-btn" on:click={clearTelemetry}>Clear</button>
    </div>

    <!-- Summary Strip -->
    <div class="summary-strip">
      <div class="stat">
        <span class="stat-label">Total</span>
        <span class="stat-value">{fmtMs($latestProfile.total_time_ms)}</span>
      </div>
      <div class="stat">
        <span class="stat-label">Memory</span>
        <span class="stat-value">{fmtMb($latestProfile.peak_memory_mb)}</span>
      </div>
      <div class="stat">
        <span class="stat-label">Threads</span>
        <span class="stat-value">{$latestProfile.threads_used}</span>
      </div>
    </div>

    <div class="section">
      <div class="sub-title">PDG Data Plane</div>
      <EditionLockBadge edition={$pdgIntegrationState.editionLock} stale={$pdgIntegrationState.stale} mismatch={$pdgIntegrationState.mismatch} />
      <div class="status-grid">
        <span>Queries</span>
        <span>{$pdgIntegrationState.queryStats.queryCount}</span>
        <span>Cache hit/miss</span>
        <span>{$pdgIntegrationState.queryStats.cacheHits} / {$pdgIntegrationState.queryStats.cacheMisses}</span>
        <span>Avg latency</span>
        <span>{fmtMs($pdgIntegrationState.queryStats.averageLatencyMs)}</span>
        <span>Last latency</span>
        <span>{fmtMs($pdgIntegrationState.queryStats.lastLatencyMs)}</span>
      </div>
    </div>

    <!-- Stage Timings Bar Chart -->
    {#if $stageTimings.length > 0}
      <div class="section">
        <div class="sub-title">Stage Timings</div>
        <div class="bar-chart">
          {#each $stageTimings as [name, ms]}
            <div class="bar-row">
              <span class="bar-label" use:tooltip={{ text: name }}>{name}</span>
              <div class="bar-track">
                <div
                  class="bar-fill"
                  style="width: {barWidth(ms, barMax)}"
                ></div>
              </div>
              <span class="bar-value">{fmtMs(ms)}</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Convergence Plot -->
    {#if $convergenceData.length >= 2}
      <div class="section">
        <div class="sub-title">Monte Carlo Convergence</div>
        <svg
          class="convergence-svg"
          viewBox="0 0 {SVG_W} {SVG_H}"
          preserveAspectRatio="xMidYMid meet"
        >
          <!-- Axes -->
          <line
            x1={SVG_PAD}
            y1={SVG_H - SVG_PAD}
            x2={SVG_W - SVG_PAD}
            y2={SVG_H - SVG_PAD}
            stroke="var(--border)"
            stroke-width="1"
          />
          <line
            x1={SVG_PAD}
            y1={SVG_PAD}
            x2={SVG_PAD}
            y2={SVG_H - SVG_PAD}
            stroke="var(--border)"
            stroke-width="1"
          />

          <!-- 1/√N reference -->
          {#if refPath}
            <path
              d={refPath}
              fill="none"
              stroke="var(--fg-secondary)"
              stroke-width="1"
              stroke-dasharray="4,3"
              opacity="0.5"
            />
          {/if}

          <!-- Data polyline -->
          <path
            d={convergencePath}
            fill="none"
            stroke="var(--hl-symbol)"
            stroke-width="2"
            stroke-linejoin="round"
          />

          <!-- Data points -->
          {#each $convergenceData as pt, i}
            {@const xMin = Math.log10(Math.max($convergenceData[0].events, 1))}
            {@const xMax = Math.log10(Math.max($convergenceData[$convergenceData.length - 1].events, 1))}
            {@const yMin = Math.min(...$convergenceData.map((p) => p.error))}
            {@const yMax = Math.max(...$convergenceData.map((p) => p.error))}
            {@const xRange = xMax - xMin || 1}
            {@const yRange = yMax - yMin || 1}
            {@const cx = SVG_PAD + ((Math.log10(Math.max(pt.events, 1)) - xMin) / xRange) * (SVG_W - 2 * SVG_PAD)}
            {@const cy = SVG_PAD + (1 - (pt.error - yMin) / yRange) * (SVG_H - 2 * SVG_PAD)}
            <circle
              cx={cx}
              cy={cy}
              r="3"
              fill="var(--hl-symbol)"
            />
          {/each}

          <!-- Axis labels -->
          <text
            x={SVG_W / 2}
            y={SVG_H - 4}
            text-anchor="middle"
            fill="var(--fg-secondary)"
            font-size="9"
            font-family="var(--font-mono)"
          >Events (log scale)</text>
          <text
            x="6"
            y={SVG_H / 2}
            text-anchor="middle"
            fill="var(--fg-secondary)"
            font-size="9"
            font-family="var(--font-mono)"
            transform="rotate(-90, 6, {SVG_H / 2})"
          >Rel. Error</text>

          <!-- Legend -->
          <line x1={SVG_W - 85} y1="12" x2={SVG_W - 70} y2="12" stroke="var(--hl-symbol)" stroke-width="2" />
          <text x={SVG_W - 67} y="15" fill="var(--fg-secondary)" font-size="8" font-family="var(--font-mono)">data</text>
          <line x1={SVG_W - 45} y1="12" x2={SVG_W - 30} y2="12" stroke="var(--fg-secondary)" stroke-width="1" stroke-dasharray="4,3" opacity="0.5" />
          <text x={SVG_W - 27} y="15" fill="var(--fg-secondary)" font-size="8" font-family="var(--font-mono)">1/√N</text>
        </svg>
      </div>
    {/if}

    <!-- History -->
    {#if $profileCount > 1}
      <div class="section">
        <div class="sub-title">History ({$profileCount} runs)</div>
        <div class="history-list">
          {#each $historicalProfiles as entry, i}
            <button
              class="history-item"
              class:active={i === 0}
              on:click={() => selectHistorical(entry)}
            >
              <span class="hist-label">{entry.label || "Run"}</span>
              <span class="hist-meta">
                {fmtMs(entry.profile.total_time_ms)} · {fmtTimestamp(entry.timestamp)}
              </span>
            </button>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .telemetry-panel {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    height: 100%;
    overflow-y: auto;
    font-size: 0.8rem;
    padding: 0.25rem;
  }

  /* ── Empty State ───────────────────────────────── */

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 0.25rem;
    color: var(--fg-secondary);
    font-style: italic;
    font-size: 0.75rem;
    line-height: 1.5;
    text-align: center;
  }

  .empty-state p {
    margin: 0;
  }

  /* ── Section Header ────────────────────────────── */

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 0.25rem;
  }

  .section-title {
    font-size: 0.85rem;
    font-weight: 700;
    color: var(--fg-accent);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ref-btn {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--fg-secondary);
    font-family: var(--font-mono);
    font-size: 0.6rem;
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .ref-btn:hover {
    border-color: var(--border-focus);
    color: var(--fg-primary);
  }

  /* ── Summary Strip ─────────────────────────────── */

  .summary-strip {
    display: flex;
    gap: 0.5rem;
    padding: 0.35rem 0.5rem;
    background: var(--bg-surface);
    border-radius: 5px;
  }

  .stat {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }

  .stat-label {
    font-size: 0.6rem;
    color: var(--fg-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-family: var(--font-mono);
  }

  .stat-value {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--fg-primary);
    font-family: var(--font-mono);
  }

  /* ── Section ───────────────────────────────────── */

  .section {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 0.4rem 0.5rem;
    background: var(--bg-surface);
    border-radius: 5px;
  }

  .sub-title {
    font-size: 0.7rem;
    font-weight: 600;
    color: var(--fg-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-family: var(--font-mono);
  }

  .status-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.18rem 0.6rem;
    font-size: 0.68rem;
    color: var(--fg-secondary);
  }

  /* ── Bar Chart ─────────────────────────────────── */

  .bar-chart {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .bar-row {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .bar-label {
    flex: 0 0 auto;
    max-width: 110px;
    font-size: 0.68rem;
    font-family: var(--font-mono);
    color: var(--fg-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .bar-track {
    flex: 1;
    height: 8px;
    background: rgba(255, 255, 255, 0.06);
    border-radius: 4px;
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--hl-symbol), var(--fg-accent));
    border-radius: 4px;
    transition: width 0.3s ease;
  }

  .bar-value {
    flex: 0 0 auto;
    font-size: 0.65rem;
    font-family: var(--font-mono);
    color: var(--fg-primary);
    min-width: 52px;
    text-align: right;
  }

  /* ── Convergence SVG ───────────────────────────── */

  .convergence-svg {
    width: 100%;
    max-height: 160px;
  }

  /* ── History ───────────────────────────────────── */

  .history-list {
    display: flex;
    flex-direction: column;
    gap: 1px;
    max-height: 120px;
    overflow-y: auto;
  }

  .history-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.25rem 0.35rem;
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
    width: 100%;
    text-align: left;
  }

  .history-item:last-child {
    border-bottom: none;
  }

  .history-item:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .history-item.active {
    background: rgba(255, 255, 255, 0.06);
  }

  .hist-label {
    font-size: 0.7rem;
    color: var(--fg-primary);
    font-family: var(--font-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 55%;
  }

  .hist-meta {
    font-size: 0.6rem;
    color: var(--fg-secondary);
    font-family: var(--font-mono);
    white-space: nowrap;
  }
</style>
