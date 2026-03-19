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
  import { onDestroy } from "svelte";
  import { calculateRelicDensity } from "$lib/api";
  import SpireNumberInput from "$lib/components/ui/SpireNumberInput.svelte";
  import HoverDef from "$lib/components/ui/HoverDef.svelte";
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
              labels: { color: "var(--color-text-primary)", font: { size: 11 } },
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
              title: { display: true, text: "x = m_χ / T", color: "var(--color-text-muted)" },
              ticks: { color: "var(--color-text-muted)" },
              grid: { color: "rgba(255,255,255,0.05)" },
            },
            y: {
              type: "logarithmic",
              title: { display: true, text: "Y = n / s", color: "var(--color-text-muted)" },
              ticks: { color: "var(--color-text-muted)" },
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
    if (c === "compatible") return "var(--hl-green, var(--color-success))";
    if (c === "over-closes") return "var(--hl-error, var(--color-error))";
    return "var(--hl-warn, var(--hl-value))";
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
    <h3><HoverDef term="relic_density">Dark Matter Candidate</HoverDef></h3>

    <div class="form-grid">
      <label for="dm-mass">Mass (GeV)</label>
      <SpireNumberInput inputId="dm-mass" bind:value={dmMass} min={1} step={10} ariaLabel="Dark matter mass" />

      <label for="sigma-a">σv (s-wave) [cm³/s]</label>
      <SpireNumberInput inputId="sigma-a" bind:value={sigmaVA} step={1e-27} ariaLabel="s-wave cross section" />

      <label for="sigma-b">σv (p-wave) [cm³/s]</label>
      <SpireNumberInput inputId="sigma-b" bind:value={sigmaVB} step={1e-27} ariaLabel="p-wave cross section" />

      <label for="g-dm">g_DM (d.o.f.)</label>
      <SpireNumberInput inputId="g-dm" bind:value={gDm} min={1} step={1} ariaLabel="Dark matter degrees of freedom" />

      <label for="g-star">g_* (energy)</label>
      <SpireNumberInput inputId="g-star" bind:value={gStar} step={0.25} ariaLabel="Energy degrees of freedom" />

      <label for="g-star-s">g_*s (entropy)</label>
      <SpireNumberInput inputId="g-star-s" bind:value={gStarS} step={0.25} ariaLabel="Entropy degrees of freedom" />
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
          <span class="result-label"><HoverDef term="relic_density">Ω h²</HoverDef></span>
          <span class="result-value" style="color: {classColor(report.classification)}">
            {report.omega_h2.toFixed(4)}
          </span>
        </div>
        <div class="result-card">
          <span class="result-label">Planck</span>
          <span class="result-value">{report.planck_omega_h2.toFixed(4)}</span>
        </div>
        <div class="result-card">
          <span class="result-label"><HoverDef term="freeze_out">x_f</HoverDef></span>
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
    gap: 0.75rem;
    padding: 0.75rem;
    height: 100%;
    overflow-y: auto;
    font-family: var(--font-mono, "JetBrains Mono", monospace);
    font-size: 0.82rem;
  }

  .cosmo-form,
  .cosmo-results,
  .cosmo-chart {
    background: color-mix(in srgb, var(--color-bg-elevated) 55%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-border) 80%, transparent);
    padding: 0.55rem;
    border-radius: var(--radius-md);
  }

  h3 {
    margin: 0 0 0.5rem;
    font-size: 0.9rem;
    color: var(--fg-secondary, var(--color-text-muted));
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  h4 {
    margin: 0 0 0.5rem;
    font-size: 0.85rem;
    color: var(--fg-secondary, var(--color-text-muted));
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
    color: var(--fg-secondary, var(--color-text-muted));
  }

  .form-grid :global(.spire-number-input) {
    font-size: 0.8rem;
  }

  .btn-compute {
    width: 100%;
    padding: 0.5rem;
    background: color-mix(in srgb, var(--color-accent) 14%, var(--color-bg-surface));
    color: var(--color-text-primary, var(--fg-primary));
    border: 1px solid color-mix(in srgb, var(--color-accent) 65%, var(--color-border));
    border-radius: var(--radius-sm);
    font-weight: 600;
    font-size: 0.85rem;
    cursor: pointer;
    transition: opacity 0.15s, background 0.15s, border-color 0.15s;
  }

  .btn-compute:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-compute:hover:not(:disabled) {
    opacity: 1;
    background: color-mix(in srgb, var(--color-accent) 24%, var(--color-bg-surface));
    border-color: color-mix(in srgb, var(--color-accent) 90%, var(--color-border));
  }

  .error-msg {
    color: var(--hl-error, var(--color-error));
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
    background: color-mix(in srgb, var(--color-bg-base) 75%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-border) 80%, transparent);
    padding: 0.5rem;
    border-radius: var(--radius-sm);
  }

  .result-label {
    font-size: 0.7rem;
    color: var(--fg-secondary, var(--color-text-muted));
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
    color: var(--color-bg-base);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    border-radius: 999px;
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
