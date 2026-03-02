<!--
  SPIRE — Kinematics View Component

  Displays kinematic analysis results including threshold energy,
  phase space information, Mandelstam variable boundaries, and
  whether the reaction is kinematically allowed.
-->
<script lang="ts">
  import { kinematics, mandelstamVars } from "$lib/stores/physicsStore";
  import type { KinematicsReport, MandelstamVars } from "$lib/types/spire";

  $: report = $kinematics;
  $: mvars = $mandelstamVars;
</script>

<div class="kinematics-view">
  <h3>Kinematics</h3>

  {#if !report && !mvars}
    <p class="hint">No kinematics computed yet. Run the pipeline or click "Kinematics" to analyse.</p>
  {:else}
    {#if report}
      <!-- Threshold -->
      <div class="section">
        <h4>Threshold</h4>
        <div class="data-grid">
          <span class="data-label">√s_threshold</span>
          <span class="data-value">{report.threshold.threshold_energy.toFixed(6)} GeV</span>

          {#if report.threshold.lab_energy !== null}
            <span class="data-label">E_lab</span>
            <span class="data-value">{report.threshold.lab_energy.toFixed(6)} GeV</span>
          {/if}

          <span class="data-label">Final masses</span>
          <span class="data-value mono">
            [{report.threshold.final_masses.map((m) => m.toFixed(4)).join(", ")}] GeV
          </span>

          <span class="data-label">Allowed</span>
          <span class="data-value" class:allowed={report.is_allowed} class:forbidden={!report.is_allowed}>
            {report.is_allowed ? "Yes" : "No (below threshold)"}
          </span>
        </div>
      </div>

      <!-- Phase Space -->
      {#if report.phase_space}
        <div class="section">
          <h4>Phase Space</h4>
          <div class="data-grid">
            <span class="data-label">n_final</span>
            <span class="data-value">{report.phase_space.n_final}</span>

            <span class="data-label">n_variables</span>
            <span class="data-value">{report.phase_space.n_variables}</span>

            <span class="data-label">√s_CM</span>
            <span class="data-value">{report.phase_space.total_energy_cm.toFixed(4)} GeV</span>

            <span class="data-label">dΦ</span>
            <span class="data-value mono">{report.phase_space.measure_expression}</span>
          </div>
        </div>
      {/if}

      <!-- Mandelstam Boundaries -->
      {#if report.mandelstam_boundaries}
        <div class="section">
          <h4>Mandelstam Boundaries</h4>
          <div class="data-grid">
            <span class="data-label">s_min</span>
            <span class="data-value mono">{report.mandelstam_boundaries.s_min.toFixed(6)} GeV²</span>

            <span class="data-label">t_min</span>
            <span class="data-value mono">{report.mandelstam_boundaries.t_min.toFixed(6)} GeV²</span>

            <span class="data-label">t_max</span>
            <span class="data-value mono">{report.mandelstam_boundaries.t_max.toFixed(6)} GeV²</span>

            <span class="data-label">Masses</span>
            <span class="data-value mono">
              [{report.mandelstam_boundaries.masses.map((m) => m.toFixed(4)).join(", ")}]
            </span>
          </div>
        </div>
      {/if}
    {/if}

    <!-- Direct Mandelstam Variables -->
    {#if mvars}
      <div class="section">
        <h4>Mandelstam Variables</h4>
        <div class="mandelstam-display">
          <div class="mvar">
            <span class="mvar-label">s</span>
            <span class="mvar-value">{mvars.s.toFixed(6)}</span>
            <span class="mvar-unit">GeV²</span>
          </div>
          <div class="mvar">
            <span class="mvar-label">t</span>
            <span class="mvar-value">{mvars.t.toFixed(6)}</span>
            <span class="mvar-unit">GeV²</span>
          </div>
          <div class="mvar">
            <span class="mvar-label">u</span>
            <span class="mvar-value">{mvars.u.toFixed(6)}</span>
            <span class="mvar-unit">GeV²</span>
          </div>
          <div class="mvar sum">
            <span class="mvar-label">s+t+u</span>
            <span class="mvar-value">{(mvars.s + mvars.t + mvars.u).toFixed(6)}</span>
            <span class="mvar-unit">= Σm²</span>
          </div>
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .kinematics-view {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  h3 {
    margin: 0 0 0.25rem;
    font-size: 0.95rem;
    color: #7ec8e3;
    border-bottom: 1px solid #0f3460;
    padding-bottom: 0.3rem;
  }
  h4 {
    margin: 0 0 0.2rem;
    font-size: 0.78rem;
    color: #a0b4c8;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .hint {
    font-size: 0.8rem;
    opacity: 0.5;
    font-style: italic;
    margin: 0;
  }
  .section {
    background: #0d1b2a;
    border: 1px solid #1b2838;
    border-radius: 4px;
    padding: 0.4rem 0.5rem;
  }
  .data-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.15rem 0.6rem;
    align-items: baseline;
  }
  .data-label {
    font-size: 0.72rem;
    color: #7f8c8d;
    text-align: right;
    white-space: nowrap;
  }
  .data-value {
    font-size: 0.78rem;
    color: #e0e0e0;
  }
  .data-value.mono {
    font-family: "Fira Code", monospace;
    font-size: 0.75rem;
  }
  .data-value.allowed {
    color: #2ecc71;
    font-weight: 600;
  }
  .data-value.forbidden {
    color: #e74c3c;
    font-weight: 600;
  }
  .mandelstam-display {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .mvar {
    display: flex;
    flex-direction: column;
    align-items: center;
    background: #162447;
    border: 1px solid #1a5276;
    border-radius: 6px;
    padding: 0.4rem 0.6rem;
    min-width: 4.5rem;
  }
  .mvar.sum {
    border-color: #0f3460;
    background: #0a0f1a;
  }
  .mvar-label {
    font-size: 0.82rem;
    font-weight: 700;
    color: #5dade2;
    font-family: "Fira Code", monospace;
  }
  .mvar-value {
    font-size: 0.78rem;
    color: #f39c12;
    font-family: "Fira Code", monospace;
  }
  .mvar-unit {
    font-size: 0.62rem;
    color: #7f8c8d;
  }
</style>
