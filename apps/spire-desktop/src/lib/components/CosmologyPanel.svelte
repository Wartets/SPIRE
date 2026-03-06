<!--
  SPIRE - Cosmology Panel

  Dark matter relic density calculator widget. Configures DM candidate
  properties (mass, annihilation cross-section, degrees of freedom),
  invokes the Boltzmann equation solver, and displays:

    • Ω h² with Planck comparison
    • Freeze-out temperature x_f = m/T_f
    • Log-log Y(x) vs Y_eq(x) evolution plot via Chart.js
    • Classification badge (under-abundant / compatible / over-closes)
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { calculateRelicDensity } from "$lib/api";
  import type { RelicConfig, RelicDensityReport, FreezeOutPoint } from "$lib/types/spire";

  // ── State ──
  let dmMass = 100;
  let sigmaVA = 3e-26;
  let sigmaVB = 0;
  let gDm = 2;
  let gStar = 86.25;
  let gStarS = 86.25;

  let report: RelicDensityReport | null = null;
  let loading = false;
  let error = "";

  // ── Chart ──
  let canvasEl: HTMLCanvasElement;
  let chartInstance: any = null;

  async function compute(): Promise<void> {
    loading = true;
    error = "";
    report = null;

    try {
      const config: RelicConfig = {
        dm_mass: dmMass,
        sigma_v_a: sigmaVA,
        sigma_v_b: sigmaVB,
        g_dm: gDm,
        g_star: gStar,
        g_star_s: gStarS,
        x_start: 1.0,
        x_end: 1000.0,
      };
      report = await calculateRelicDensity(config);
      renderChart();
    } catch (e: any) {
      error = e?.message ?? String(e);
    } finally {
      loading = false;
    }
  }

  function renderChart(): void {
    if (!report || !canvasEl) return;

    // Dynamic import of Chart.js (already a dependency of the project)
    import("chart.js/auto").then(({ default: ChartLib }) => {
      if (chartInstance) {
        chartInstance.destroy();
      }

      const evo = report!.evolution;

      // Prepare data: log10(x), log10(Y), log10(Y_eq)
      const labels = evo.map((p: FreezeOutPoint) => p.x);
      const yData = evo.map((p: FreezeOutPoint) => Math.max(p.y, 1e-30));
      const yEqData = evo.map((p: FreezeOutPoint) => Math.max(p.y_eq, 1e-30));

      chartInstance = new ChartLib(canvasEl, {
        type: "line",
        data: {
          labels,
          datasets: [
            {
              label: "Y(x) - actual yield",
              data: yData,
              borderColor: "#4fc3f7",
              backgroundColor: "rgba(79, 195, 247, 0.1)",
              borderWidth: 2,
              pointRadius: 0,
              fill: false,
            },
            {
              label: "Y_eq(x) - equilibrium",
              data: yEqData,
              borderColor: "#ff8a65",
              backgroundColor: "rgba(255, 138, 101, 0.1)",
              borderWidth: 2,
              borderDash: [6, 3],
              pointRadius: 0,
              fill: false,
            },
          ],
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          interaction: {
            mode: "index",
            intersect: false,
          },
          plugins: {
            legend: {
              labels: { color: "#ccc", font: { size: 11 } },
            },
            tooltip: {
              callbacks: {
                label: (ctx: any) => `${ctx.dataset.label}: ${ctx.raw.toExponential(3)}`,
                title: (items: any[]) => `x = m/T = ${Number(items[0].label).toFixed(1)}`,
              },
            },
          },
          scales: {
            x: {
              type: "logarithmic",
              title: { display: true, text: "x = m_χ / T", color: "#aaa" },
              ticks: { color: "#888" },
              grid: { color: "rgba(255,255,255,0.05)" },
            },
            y: {
              type: "logarithmic",
              title: { display: true, text: "Y = n / s", color: "#aaa" },
              ticks: { color: "#888" },
              grid: { color: "rgba(255,255,255,0.05)" },
              min: 1e-20,
            },
          },
        },
      });
    }).catch(() => {
      // Chart.js not available - silent fallback
    });
  }

  function classColor(c: string): string {
    if (c === "compatible") return "var(--hl-green, #66bb6a)";
    if (c === "over-closes") return "var(--hl-error, #ef5350)";
    return "var(--hl-warn, #ffa726)";
  }

  function fmtSci(v: number, digits = 3): string {
    return v.toExponential(digits);
  }

  onDestroy(() => {
    if (chartInstance) chartInstance.destroy();
  });
</script>

<div class="cosmo-panel">
  <!-- ── Input Form ── -->
  <section class="cosmo-form">
    <h3>Dark Matter Candidate</h3>

    <div class="form-grid">
      <label for="dm-mass">Mass (GeV)</label>
      <input id="dm-mass" type="number" bind:value={dmMass} min="1" step="10" />

      <label for="sigma-a">σv (s-wave) [cm³/s]</label>
      <input id="sigma-a" type="number" bind:value={sigmaVA} step="1e-27" />

      <label for="sigma-b">σv (p-wave) [cm³/s]</label>
      <input id="sigma-b" type="number" bind:value={sigmaVB} step="1e-27" />

      <label for="g-dm">g_DM (d.o.f.)</label>
      <input id="g-dm" type="number" bind:value={gDm} min="1" step="1" />

      <label for="g-star">g_* (energy)</label>
      <input id="g-star" type="number" bind:value={gStar} step="0.25" />

      <label for="g-star-s">g_*s (entropy)</label>
      <input id="g-star-s" type="number" bind:value={gStarS} step="0.25" />
    </div>

    <button class="btn-compute" on:click={compute} disabled={loading}>
      {loading ? "Computing…" : "Compute Relic Density"}
    </button>

    {#if error}
      <p class="error-msg">{error}</p>
    {/if}
  </section>

  <!-- ── Results ── -->
  {#if report}
    <section class="cosmo-results">
      <div class="result-grid">
        <div class="result-card">
          <span class="result-label">Ω h²</span>
          <span class="result-value" style="color: {classColor(report.classification)}">
            {report.omega_h2.toFixed(4)}
          </span>
        </div>
        <div class="result-card">
          <span class="result-label">Planck</span>
          <span class="result-value">{report.planck_omega_h2.toFixed(4)}</span>
        </div>
        <div class="result-card">
          <span class="result-label">x_f</span>
          <span class="result-value">{report.x_freeze_out.toFixed(1)}</span>
        </div>
        <div class="result-card">
          <span class="result-label">Y_∞</span>
          <span class="result-value">{fmtSci(report.y_infinity)}</span>
        </div>
        <div class="result-card">
          <span class="result-label">⟨σv⟩_f</span>
          <span class="result-value">{fmtSci(report.sigma_v)}</span>
        </div>
        <div class="result-card">
          <span class="result-label">Status</span>
          <span class="result-badge" style="background: {classColor(report.classification)}">
            {report.classification}
          </span>
        </div>
      </div>
    </section>

    <!-- ── Freeze-Out Plot ── -->
    <section class="cosmo-chart">
      <h4>Freeze-Out Evolution: Y(x) vs Y_eq(x)</h4>
      <div class="chart-wrapper">
        <canvas bind:this={canvasEl}></canvas>
      </div>
    </section>
  {/if}
</div>

<style>
  .cosmo-panel {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 0.75rem;
    height: 100%;
    overflow-y: auto;
    font-family: var(--font-mono, "JetBrains Mono", monospace);
  }

  h3 {
    margin: 0 0 0.5rem;
    font-size: 0.9rem;
    color: var(--fg-secondary, #aaa);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  h4 {
    margin: 0 0 0.5rem;
    font-size: 0.85rem;
    color: var(--fg-secondary, #aaa);
  }

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.35rem 0.75rem;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .form-grid label {
    font-size: 0.8rem;
    color: var(--fg-secondary, #bbb);
  }

  .form-grid input {
    background: var(--bg-input, #1e1e2e);
    border: 1px solid var(--border, #333);
    border-radius: 4px;
    color: var(--fg-primary, #eee);
    padding: 0.3rem 0.5rem;
    font-size: 0.8rem;
    font-family: inherit;
  }

  .btn-compute {
    width: 100%;
    padding: 0.5rem;
    background: var(--accent, #4fc3f7);
    color: #000;
    border: none;
    border-radius: 4px;
    font-weight: 600;
    font-size: 0.85rem;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .btn-compute:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-compute:hover:not(:disabled) {
    opacity: 0.85;
  }

  .error-msg {
    color: var(--hl-error, #ef5350);
    font-size: 0.8rem;
    margin-top: 0.3rem;
  }

  .result-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.5rem;
  }

  .result-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    background: var(--bg-input, #1e1e2e);
    border: 1px solid var(--border, #333);
    border-radius: 6px;
    padding: 0.5rem;
  }

  .result-label {
    font-size: 0.7rem;
    color: var(--fg-secondary, #999);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 0.2rem;
  }

  .result-value {
    font-size: 1rem;
    font-weight: 700;
    color: var(--fg-primary, #eee);
  }

  .result-badge {
    font-size: 0.75rem;
    font-weight: 700;
    padding: 0.15rem 0.5rem;
    border-radius: 4px;
    color: #000;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .cosmo-chart {
    flex: 1;
    min-height: 200px;
  }

  .chart-wrapper {
    position: relative;
    height: calc(100% - 1.5rem);
    min-height: 180px;
  }

  .chart-wrapper canvas {
    width: 100% !important;
    height: 100% !important;
  }
</style>
