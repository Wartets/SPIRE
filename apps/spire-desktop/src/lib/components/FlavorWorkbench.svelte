<!--
  SPIRE — Flavor Physics Workbench

  Precision flavor observable calculator widget. Provides a tabbed interface
  for configuring:

    1. Lattice QCD inputs (decay constants, bag parameters, z-expansion
       form factor coefficients) with FLAG 2023 defaults.
    2. Wilson coefficients (C7, C9, C10 and their primed counterparts)
       with BSM shift sliders.
    3. Observable results: ΔMd, ΔMs, integrated branching ratio, and
       a q²-dependent dΓ/dq² distribution plot via Chart.js.
-->
<script lang="ts">
  import { onDestroy } from "svelte";
  import { calculateBToKll } from "$lib/api";
  import type {
    LatticeInputs,
    WilsonCoefficients,
    FlavorObservableReport,
    DifferentialPoint,
  } from "$lib/types/spire";

  // ── Tab State ──
  type TabId = "lattice" | "wilson" | "observables";
  let activeTab: TabId = "lattice";

  // ── Lattice Inputs (FLAG 2023 Defaults) ──
  let fBValue = 0.1903;
  let fBError = 0.0013;
  let fBsValue = 0.2303;
  let fBsError = 0.0013;
  let fKValue = 0.1557;
  let fKError = 0.0003;
  let bHatD = 1.222;
  let bHatDErr = 0.061;
  let bHatS = 1.270;
  let bHatSErr = 0.054;

  // f+ coefficients
  let fpA0 = 0.466;
  let fpA1 = -0.885;
  let fpA2 = -0.213;
  // f0 coefficients
  let f0A0 = 0.292;
  let f0A1 = 0.281;
  let f0A2 = 0.150;
  // fT coefficients
  let ftA0 = 0.460;
  let ftA1 = -0.798;
  let ftA2 = -0.470;

  // ── Wilson Coefficients ──
  let c7Eff = -0.304;
  let c7Prime = 0.0;
  let c9Eff = 4.211;
  let c9Prime = 0.0;
  let c10 = -4.103;
  let c10Prime = 0.0;
  let deltaC9 = 0.0;
  let deltaC10 = 0.0;

  // ── q² Window ──
  let q2Min = 1.0;
  let q2Max = 6.0;

  // ── Results ──
  let report: FlavorObservableReport | null = null;
  let loading = false;
  let error = "";

  // ── Chart ──
  let canvasEl: HTMLCanvasElement;
  let chartInstance: any = null;

  // Physical constants for z-expansion
  const M_B = 5.27934;
  const M_K = 0.49368;
  const M_B_STAR_S = 5.3252;
  const M_B_STAR_S0 = 5.711;

  function buildLatticeInputs(): LatticeInputs {
    const tPlus = (M_B + M_K) ** 2;
    const t0 = (M_B + M_K) * (Math.sqrt(M_B) - Math.sqrt(M_K)) ** 2;

    return {
      f_b: { value: fBValue, error: fBError, scale_mu: 4.18 },
      f_bs: { value: fBsValue, error: fBsError, scale_mu: 4.18 },
      f_k: { value: fKValue, error: fKError, scale_mu: 2.0 },
      b_hat_d: { value: bHatD, error: bHatDErr },
      b_hat_s: { value: bHatS, error: bHatSErr },
      b_to_k: {
        f_plus: {
          coefficients: [fpA0, fpA1, fpA2],
          m_pole: M_B_STAR_S,
          t_plus: tPlus,
          t_0: t0,
        },
        f_zero: {
          coefficients: [f0A0, f0A1, f0A2],
          m_pole: M_B_STAR_S0,
          t_plus: tPlus,
          t_0: t0,
        },
        f_tensor: {
          coefficients: [ftA0, ftA1, ftA2],
          m_pole: M_B_STAR_S,
          t_plus: tPlus,
          t_0: t0,
        },
      },
    };
  }

  function buildWilsonCoeffs(): WilsonCoefficients {
    return {
      c7_eff: c7Eff,
      c7_prime: c7Prime,
      c9_eff: c9Eff + deltaC9,
      c9_prime: c9Prime,
      c10: c10 + deltaC10,
      c10_prime: c10Prime,
      scale_mu: 4.18,
    };
  }

  async function compute(): Promise<void> {
    loading = true;
    error = "";
    report = null;

    try {
      const lattice = buildLatticeInputs();
      const wilson = buildWilsonCoeffs();
      report = await calculateBToKll(q2Min, q2Max, wilson, lattice, 100);
      activeTab = "observables";
      renderChart();
    } catch (e: any) {
      error = e?.message ?? String(e);
    } finally {
      loading = false;
    }
  }

  function renderChart(): void {
    if (!report || !canvasEl) return;

    import("chart.js/auto").then(({ default: ChartLib }) => {
      if (chartInstance) {
        chartInstance.destroy();
      }

      const spec = report!.differential_spectrum;
      const labels = spec.map((p: DifferentialPoint) => p.q2.toFixed(2));
      const data = spec.map((p: DifferentialPoint) => p.dgamma_dq2);

      chartInstance = new ChartLib(canvasEl, {
        type: "line",
        data: {
          labels,
          datasets: [
            {
              label: "dΓ/dq² (GeV⁻¹)",
              data,
              borderColor: "#4fc3f7",
              backgroundColor: "rgba(79, 195, 247, 0.1)",
              borderWidth: 2,
              pointRadius: 0,
              fill: true,
            },
          ],
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          interaction: { mode: "index", intersect: false },
          plugins: {
            legend: {
              labels: { color: "#ccc", font: { size: 11 } },
            },
            tooltip: {
              callbacks: {
                label: (ctx: any) => `dΓ/dq² = ${ctx.raw.toExponential(3)}`,
                title: (items: any[]) => `q² = ${items[0].label} GeV²`,
              },
            },
          },
          scales: {
            x: {
              title: { display: true, text: "q² (GeV²)", color: "#aaa" },
              ticks: { color: "#888", maxTicksLimit: 12 },
              grid: { color: "rgba(255,255,255,0.05)" },
            },
            y: {
              title: { display: true, text: "dΓ/dq² (GeV⁻¹)", color: "#aaa" },
              ticks: {
                color: "#888",
                callback: (v: any) => Number(v).toExponential(1),
              },
              grid: { color: "rgba(255,255,255,0.05)" },
            },
          },
        },
      });
    }).catch(() => {
      // Chart.js not available — silent fallback
    });
  }

  function fmtSci(v: number, digits = 3): string {
    return v.toExponential(digits);
  }

  function resetDefaults(): void {
    fBValue = 0.1903; fBError = 0.0013;
    fBsValue = 0.2303; fBsError = 0.0013;
    fKValue = 0.1557; fKError = 0.0003;
    bHatD = 1.222; bHatDErr = 0.061;
    bHatS = 1.270; bHatSErr = 0.054;
    fpA0 = 0.466; fpA1 = -0.885; fpA2 = -0.213;
    f0A0 = 0.292; f0A1 = 0.281; f0A2 = 0.150;
    ftA0 = 0.460; ftA1 = -0.798; ftA2 = -0.470;
    c7Eff = -0.304; c7Prime = 0.0;
    c9Eff = 4.211; c9Prime = 0.0;
    c10 = -4.103; c10Prime = 0.0;
    deltaC9 = 0.0; deltaC10 = 0.0;
    q2Min = 1.0; q2Max = 6.0;
  }

  onDestroy(() => {
    if (chartInstance) chartInstance.destroy();
  });
</script>

<div class="flavor-panel">
  <!-- ── Tab Bar ── -->
  <nav class="tab-bar">
    <button
      class="tab-btn" class:active={activeTab === "lattice"}
      on:click={() => (activeTab = "lattice")}
    >Lattice Inputs</button>
    <button
      class="tab-btn" class:active={activeTab === "wilson"}
      on:click={() => (activeTab = "wilson")}
    >Wilson Coefficients</button>
    <button
      class="tab-btn" class:active={activeTab === "observables"}
      on:click={() => (activeTab = "observables")}
    >Observables</button>
  </nav>

  <!-- ══════════════ Lattice Tab ══════════════ -->
  {#if activeTab === "lattice"}
    <section class="tab-content">
      <h3>Decay Constants (FLAG 2023)</h3>
      <div class="form-grid">
        <label>f<sub>B</sub> (GeV)</label>
        <input type="number" bind:value={fBValue} step="0.001" />
        <label>± σ</label>
        <input type="number" bind:value={fBError} step="0.0001" />

        <label>f<sub>Bs</sub> (GeV)</label>
        <input type="number" bind:value={fBsValue} step="0.001" />
        <label>± σ</label>
        <input type="number" bind:value={fBsError} step="0.0001" />

        <label>f<sub>K</sub> (GeV)</label>
        <input type="number" bind:value={fKValue} step="0.001" />
        <label>± σ</label>
        <input type="number" bind:value={fKError} step="0.0001" />
      </div>

      <h3>Bag Parameters</h3>
      <div class="form-grid">
        <label>B̂<sub>d</sub></label>
        <input type="number" bind:value={bHatD} step="0.01" />
        <label>± σ</label>
        <input type="number" bind:value={bHatDErr} step="0.01" />

        <label>B̂<sub>s</sub></label>
        <input type="number" bind:value={bHatS} step="0.01" />
        <label>± σ</label>
        <input type="number" bind:value={bHatSErr} step="0.01" />
      </div>

      <h3>B → K Form Factors (BCL z-expansion)</h3>

      <h4>f<sub>+</sub>(q²)</h4>
      <div class="form-grid three-col">
        <label>a₀</label><input type="number" bind:value={fpA0} step="0.01" />
        <label>a₁</label><input type="number" bind:value={fpA1} step="0.01" />
        <label>a₂</label><input type="number" bind:value={fpA2} step="0.01" />
      </div>

      <h4>f<sub>0</sub>(q²)</h4>
      <div class="form-grid three-col">
        <label>a₀</label><input type="number" bind:value={f0A0} step="0.01" />
        <label>a₁</label><input type="number" bind:value={f0A1} step="0.01" />
        <label>a₂</label><input type="number" bind:value={f0A2} step="0.01" />
      </div>

      <h4>f<sub>T</sub>(q²)</h4>
      <div class="form-grid three-col">
        <label>a₀</label><input type="number" bind:value={ftA0} step="0.01" />
        <label>a₁</label><input type="number" bind:value={ftA1} step="0.01" />
        <label>a₂</label><input type="number" bind:value={ftA2} step="0.01" />
      </div>

      <button class="btn-secondary" on:click={resetDefaults}>Reset to FLAG Defaults</button>
    </section>
  {/if}

  <!-- ══════════════ Wilson Tab ══════════════ -->
  {#if activeTab === "wilson"}
    <section class="tab-content">
      <h3>SM Wilson Coefficients (μ = m<sub>b</sub>)</h3>
      <div class="form-grid">
        <label>C<sub>7</sub><sup>eff</sup></label>
        <input type="number" bind:value={c7Eff} step="0.01" />
        <label>C<sub>7</sub>'</label>
        <input type="number" bind:value={c7Prime} step="0.01" />

        <label>C<sub>9</sub><sup>eff</sup></label>
        <input type="number" bind:value={c9Eff} step="0.01" />
        <label>C<sub>9</sub>'</label>
        <input type="number" bind:value={c9Prime} step="0.01" />

        <label>C<sub>10</sub></label>
        <input type="number" bind:value={c10} step="0.01" />
        <label>C<sub>10</sub>'</label>
        <input type="number" bind:value={c10Prime} step="0.01" />
      </div>

      <h3>BSM Shifts</h3>
      <div class="slider-group">
        <label>ΔC<sub>9</sub>: <strong>{deltaC9.toFixed(2)}</strong></label>
        <input type="range" bind:value={deltaC9} min="-3" max="3" step="0.05" />
      </div>
      <div class="slider-group">
        <label>ΔC<sub>10</sub>: <strong>{deltaC10.toFixed(2)}</strong></label>
        <input type="range" bind:value={deltaC10} min="-3" max="3" step="0.05" />
      </div>

      <h3>q² Integration Window (GeV²)</h3>
      <div class="form-grid">
        <label>q²<sub>min</sub></label>
        <input type="number" bind:value={q2Min} step="0.5" min="0.05" />
        <label>q²<sub>max</sub></label>
        <input type="number" bind:value={q2Max} step="0.5" max="22.9" />
      </div>
    </section>
  {/if}

  <!-- ══════════════ Observables Tab ══════════════ -->
  {#if activeTab === "observables"}
    <section class="tab-content">
      {#if report}
        <div class="result-grid">
          <div class="result-card">
            <span class="result-label">ΔM<sub>d</sub> (ps⁻¹)</span>
            <span class="result-value">{report.delta_m_d.toFixed(4)}</span>
            <span class="result-exp">exp: {report.exp_delta_m_d.toFixed(4)}</span>
          </div>
          <div class="result-card">
            <span class="result-label">ΔM<sub>s</sub> (ps⁻¹)</span>
            <span class="result-value">{report.delta_m_s.toFixed(2)}</span>
            <span class="result-exp">exp: {report.exp_delta_m_s.toFixed(2)}</span>
          </div>
          <div class="result-card">
            <span class="result-label">BR(B→Kμμ)</span>
            <span class="result-value">{fmtSci(report.branching_ratio)}</span>
            <span class="result-exp">[{report.q2_min.toFixed(1)}, {report.q2_max.toFixed(1)}] GeV²</span>
          </div>
        </div>

        <!-- ── dΓ/dq² Plot ── -->
        <section class="flavor-chart">
          <h4>Differential Decay Rate: dΓ/dq² vs q²</h4>
          <div class="chart-wrapper">
            <canvas bind:this={canvasEl}></canvas>
          </div>
        </section>
      {:else if !loading}
        <p class="info-msg">Configure inputs and press "Compute" to see results.</p>
      {/if}
    </section>
  {/if}

  <!-- ── Compute Button (always visible) ── -->
  <div class="action-bar">
    <button class="btn-compute" on:click={compute} disabled={loading}>
      {loading ? "Computing…" : "Compute Flavor Observables"}
    </button>
    {#if error}
      <p class="error-msg">{error}</p>
    {/if}
  </div>
</div>

<style>
  .flavor-panel {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem;
    height: 100%;
    overflow-y: auto;
    font-family: var(--font-mono, "JetBrains Mono", monospace);
  }

  /* ── Tabs ── */
  .tab-bar {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--border, #333);
    margin-bottom: 0.5rem;
  }

  .tab-btn {
    flex: 1;
    padding: 0.45rem 0.5rem;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--fg-secondary, #999);
    font-size: 0.78rem;
    font-family: inherit;
    cursor: pointer;
    transition: all 0.15s;
  }

  .tab-btn:hover {
    color: var(--fg-primary, #eee);
  }

  .tab-btn.active {
    color: var(--accent, #4fc3f7);
    border-bottom-color: var(--accent, #4fc3f7);
  }

  .tab-content {
    flex: 1;
    overflow-y: auto;
  }

  /* ── Headings ── */
  h3 {
    margin: 0.5rem 0 0.35rem;
    font-size: 0.85rem;
    color: var(--fg-secondary, #aaa);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  h4 {
    margin: 0.3rem 0 0.2rem;
    font-size: 0.78rem;
    color: var(--fg-secondary, #bbb);
  }

  /* ── Form Layouts ── */
  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr 1fr;
    gap: 0.3rem 0.5rem;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .form-grid.three-col {
    grid-template-columns: auto 1fr auto 1fr auto 1fr;
  }

  .form-grid label {
    font-size: 0.76rem;
    color: var(--fg-secondary, #bbb);
  }

  .form-grid input {
    background: var(--bg-input, #1e1e2e);
    border: 1px solid var(--border, #333);
    border-radius: 4px;
    color: var(--fg-primary, #eee);
    padding: 0.25rem 0.4rem;
    font-size: 0.78rem;
    font-family: inherit;
    width: 100%;
    box-sizing: border-box;
  }

  /* ── Slider ── */
  .slider-group {
    margin-bottom: 0.5rem;
  }

  .slider-group label {
    display: block;
    font-size: 0.78rem;
    color: var(--fg-secondary, #bbb);
    margin-bottom: 0.15rem;
  }

  .slider-group input[type="range"] {
    width: 100%;
    accent-color: var(--accent, #4fc3f7);
  }

  /* ── Buttons ── */
  .action-bar {
    padding-top: 0.5rem;
    border-top: 1px solid var(--border, #333);
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

  .btn-secondary {
    padding: 0.35rem 0.75rem;
    background: transparent;
    border: 1px solid var(--border, #555);
    border-radius: 4px;
    color: var(--fg-secondary, #ccc);
    font-size: 0.78rem;
    cursor: pointer;
    margin-top: 0.5rem;
  }

  .btn-secondary:hover {
    border-color: var(--accent, #4fc3f7);
    color: var(--accent, #4fc3f7);
  }

  /* ── Results ── */
  .result-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.5rem;
    margin-bottom: 0.75rem;
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
    margin-bottom: 0.15rem;
  }

  .result-value {
    font-size: 1rem;
    font-weight: 700;
    color: var(--fg-primary, #eee);
  }

  .result-exp {
    font-size: 0.65rem;
    color: var(--fg-secondary, #777);
    margin-top: 0.1rem;
  }

  .error-msg {
    color: var(--hl-error, #ef5350);
    font-size: 0.8rem;
    margin-top: 0.3rem;
  }

  .info-msg {
    color: var(--fg-secondary, #999);
    font-size: 0.8rem;
    text-align: center;
    padding: 2rem 1rem;
  }

  /* ── Chart ── */
  .flavor-chart {
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
