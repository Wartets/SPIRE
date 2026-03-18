<!--
  SPIRE – Kinematics View  (Phase 65: Widget Polish)

  Displays kinematic analysis results including threshold energy,
  phase space information, Mandelstam variable boundaries, and
  whether the reaction is kinematically allowed.

  Phase 65 additions:
    • Event isolation  – wheel / pointer / mouse events contained inside
      the widget; scrolling the table never zooms the Infinite Canvas.
    • Tooltip coverage – every data label carries a physics description.
    • Granular context menus on each data row (Copy value, Send to Scanner).
    • Known-resonance reference panel (collapsible <details>).
    • Deep interop: reads decay_calculator broadcast for width annotation.
    • Richer publishWidgetInterop payload (phase-space & boundary data).
-->
<script lang="ts">
  import { kinematics, mandelstamVars } from "$lib/stores/physicsStore";
  import MathRenderer from "$lib/components/math/MathRenderer.svelte";
  import type { KinematicsReport, MandelstamVars } from "$lib/types/spire";
  import { publishWidgetInterop, widgetInteropState } from "$lib/stores/widgetInteropStore";
  import { showContextMenu } from "$lib/stores/contextMenuStore";
  import { isolateEvents } from "$lib/actions/widgetEvents";
  import { tooltip } from "$lib/actions/tooltip";

  $: report = $kinematics;
  $: mvars = $mandelstamVars;

  // ---------------------------------------------------------------------------
  // Deep interop – annotate decay pole data from the Decay Calculator widget
  // ---------------------------------------------------------------------------
  $: decayPayload = $widgetInteropState.decay_calculator?.payload as
    | {
        selectedParticle?: string | null;
        selectedParticleName?: string | null;
        totalWidth?: number | null;
        lifetimeSeconds?: number | null;
      }
    | undefined;

  // ---------------------------------------------------------------------------
  // Known resonance reference panel
  // ---------------------------------------------------------------------------
  interface ResonanceRef {
    label: string;
    sqrtS: number;
    description: string;
  }

  const KNOWN_THRESHOLDS: ResonanceRef[] = [
    { label: "J/ψ",    sqrtS: 3.097,  description: "Charmonium ground state (cc̄, JP=1−)" },
    { label: "Υ(1S)",  sqrtS: 9.460,  description: "Bottomonium ground state (bb̄, JP=1−)" },
    { label: "Z pole", sqrtS: 91.188, description: "Z boson resonance mass (LEP/SLC pole); Γ_Z = 2.495 GeV" },
    { label: "W⁺W⁻",  sqrtS: 160.8,  description: "W-pair production threshold: √s_thr ≈ 2 m_W" },
    { label: "tt̄",    sqrtS: 345.0,  description: "Top-quark pair production threshold: √s_thr ≈ 2 m_t" },
    { label: "H(125)", sqrtS: 125.25, description: "Higgs boson pole mass m_H (PDG 2024)" },
  ];

  const NEAR_FRAC = 0.02;
  function isNear(ref: ResonanceRef): boolean {
    if (!report) return false;
    return Math.abs(report.threshold.threshold_energy - ref.sqrtS) / ref.sqrtS < NEAR_FRAC;
  }

  // ---------------------------------------------------------------------------
  // Interop publish – richer payload broadcast to Parameter Scanner, etc.
  // ---------------------------------------------------------------------------
  $: publishWidgetInterop("kinematics", {
    hasReport:       Boolean(report),
    allowed:         report?.is_allowed ?? null,
    thresholdEnergy: report?.threshold.threshold_energy ?? null,
    labEnergy:       report?.threshold.lab_energy ?? null,
    finalMasses:     report?.threshold.final_masses ?? [],
    nFinal:          report?.phase_space?.n_final ?? null,
    nVariables:      report?.phase_space?.n_variables ?? null,
    sqrtSCm:         report?.phase_space?.total_energy_cm ?? null,
    sMin:            report?.mandelstam_boundaries?.s_min ?? null,
    tMin:            report?.mandelstam_boundaries?.t_min ?? null,
    tMax:            report?.mandelstam_boundaries?.t_max ?? null,
    mandelstamS:     mvars?.s ?? null,
    mandelstamT:     mvars?.t ?? null,
    mandelstamU:     mvars?.u ?? null,
  });

  // ---------------------------------------------------------------------------
  // Context menu helpers
  // ---------------------------------------------------------------------------
  function openValueContext(e: MouseEvent, label: string, value: string): void {
    e.preventDefault();
    e.stopPropagation();
    showContextMenu(e.clientX, e.clientY, [
      { type: "action", id: "copy-value",       label: `Copy: ${value}`,          icon: "CP",  action: () => navigator.clipboard.writeText(value) },
      { type: "separator", id: "sep-1" },
      { type: "action", id: "copy-label-value", label: `Copy "${label} = ${value}"`, icon: "TXT", action: () => navigator.clipboard.writeText(`${label} = ${value}`) },
    ]);
  }

  function openMandelstamContext(e: MouseEvent, variable: string, value: number): void {
    e.preventDefault();
    e.stopPropagation();
    const str = `${value.toFixed(6)} GeV²`;
    showContextMenu(e.clientX, e.clientY, [
      { type: "action", id: "copy-value", label: `Copy value: ${str}`,    icon: "CP",  action: () => navigator.clipboard.writeText(str) },
      { type: "action", id: "copy-latex", label: "Copy LaTeX expression", icon: "TEX", action: () => navigator.clipboard.writeText(`${variable} = ${value.toFixed(6)}\\,\\text{GeV}^2`) },
    ]);
  }

  // ---------------------------------------------------------------------------
  // Formatting helpers
  // ---------------------------------------------------------------------------
  function fmtLifetime(t: number): string {
    if (t <= 0) return "stable";
    if (t < 1e-15) return (t * 1e15).toFixed(3) + " fs";
    if (t < 1e-12) return (t * 1e12).toFixed(3) + " ps";
    if (t < 1e-9)  return (t * 1e9).toFixed(3)  + " ns";
    return t.toExponential(3) + " s";
  }
</script>

<div class="kinematics-view" use:isolateEvents>
  <h3>Kinematics</h3>

  {#if !report && !mvars}
    <p class="hint">
      No kinematics computed yet. Run the pipeline or click
      <strong>Compute Kinematics</strong> to analyse the reaction.
    </p>
  {:else}
    {#if report}
      <!-- ── Threshold ───────────────────────────────────────────── -->
      <div class="section">
        <h4>Threshold</h4>
        <div class="data-grid">
          <span class="data-label"
            use:tooltip={{ text: "Minimum centre-of-mass energy √s for the reaction to proceed, i.e. √s_thr = Σ m_final" }}>
            √s<sub>thr</sub>
          </span>
          <span class="data-value" role="button" tabindex="0"
            on:contextmenu|stopPropagation={(e) =>
              openValueContext(e, "√s_thr", `${report!.threshold.threshold_energy.toFixed(6)} GeV`)}>
            {report.threshold.threshold_energy.toFixed(6)} GeV
          </span>

          {#if report.threshold.lab_energy !== null}
            <span class="data-label"
              use:tooltip={{ text: "Equivalent fixed-target beam energy: E_lab = (s − m_beam² − m_target²) / (2 m_target)" }}>
              E<sub>lab</sub>
            </span>
            <span class="data-value" role="button" tabindex="0"
              on:contextmenu|stopPropagation={(e) =>
                openValueContext(e, "E_lab", `${report!.threshold.lab_energy!.toFixed(6)} GeV`)}>
              {report.threshold.lab_energy!.toFixed(6)} GeV
            </span>
          {/if}

          <span class="data-label"
            use:tooltip={{ text: "Invariant masses of all final-state particles [GeV/c²]" }}>
            m<sub>final</sub>
          </span>
          <span class="data-value mono" role="button" tabindex="0"
            on:contextmenu|stopPropagation={(e) =>
              openValueContext(e, "Final masses",
                `[${report!.threshold.final_masses.map((m) => m.toFixed(4)).join(", ")}] GeV`)}>
            [{report.threshold.final_masses.map((m) => m.toFixed(4)).join(", ")}] GeV
          </span>

          <span class="data-label"
            use:tooltip={{ text: "Whether √s at the current CM energy exceeds the kinematic threshold" }}>
            Allowed
          </span>
          <span class="data-value" class:allowed={report.is_allowed} class:forbidden={!report.is_allowed}>
            {report.is_allowed ? "✓ Yes" : "✗ No (below threshold)"}
          </span>
        </div>
      </div>

      <!-- ── Phase Space ─────────────────────────────────────────── -->
      {#if report.phase_space}
        <div class="section">
          <h4>Phase Space</h4>
          <div class="data-grid">
            <span class="data-label"
              use:tooltip={{ text: "Number of final-state particles" }}>
              n<sub>final</sub>
            </span>
            <span class="data-value" role="button" tabindex="0"
              on:contextmenu|stopPropagation={(e) =>
                openValueContext(e, "n_final", String(report!.phase_space!.n_final))}>
              {report.phase_space.n_final}
            </span>

            <span class="data-label"
              use:tooltip={{ text: "Independent kinematic degrees of freedom: 3n − 4 for n ≥ 2 (after 4-momentum conservation)" }}>
              n<sub>var</sub>
            </span>
            <span class="data-value">{report.phase_space.n_variables}</span>

            <span class="data-label"
              use:tooltip={{ text: "Total CM energy √s used for phase-space evaluation [GeV]" }}>
              √s<sub>CM</sub>
            </span>
            <span class="data-value" role="button" tabindex="0"
              on:contextmenu|stopPropagation={(e) =>
                openValueContext(e, "√s_CM", `${report!.phase_space!.total_energy_cm.toFixed(4)} GeV`)}>
              {report.phase_space.total_energy_cm.toFixed(4)} GeV
            </span>

            <span class="data-label"
              use:tooltip={{ text: "Symbolic Lorentz-invariant phase-space measure dΦ_n" }}>
              dΦ
            </span>
            <span class="data-value mono" role="button" tabindex="0"
              on:contextmenu|stopPropagation={(e) =>
                openValueContext(e, "dΦ", report!.phase_space!.measure_expression)}>
                <MathRenderer latex={report.phase_space.measure_expression} mode="rendered" block={false} />
            </span>
          </div>
        </div>
      {/if}

      <!-- ── Mandelstam Boundaries ───────────────────────────────── -->
      {#if report.mandelstam_boundaries}
        <div class="section">
          <h4>Mandelstam Boundaries</h4>
          <div class="data-grid">
            <span class="data-label"
              use:tooltip={{ text: "s_min = (Σ m_i)² — lower bound on s from final-state kinematics [GeV²]" }}>
              s<sub>min</sub>
            </span>
            <span class="data-value mono" role="button" tabindex="0"
              on:contextmenu|stopPropagation={(e) =>
                openValueContext(e, "s_min", `${report!.mandelstam_boundaries!.s_min.toFixed(6)} GeV²`)}>
              {report.mandelstam_boundaries.s_min.toFixed(6)} GeV²
            </span>

            <span class="data-label"
              use:tooltip={{ text: "Minimum t (cosθ = −1, most backward scattering in CM frame) [GeV²]" }}>
              t<sub>min</sub>
            </span>
            <span class="data-value mono" role="button" tabindex="0"
              on:contextmenu|stopPropagation={(e) =>
                openValueContext(e, "t_min", `${report!.mandelstam_boundaries!.t_min.toFixed(6)} GeV²`)}>
              {report.mandelstam_boundaries.t_min.toFixed(6)} GeV²
            </span>

            <span class="data-label"
              use:tooltip={{ text: "Maximum t (cosθ = +1, most forward scattering in CM frame) [GeV²]" }}>
              t<sub>max</sub>
            </span>
            <span class="data-value mono" role="button" tabindex="0"
              on:contextmenu|stopPropagation={(e) =>
                openValueContext(e, "t_max", `${report!.mandelstam_boundaries!.t_max.toFixed(6)} GeV²`)}>
              {report.mandelstam_boundaries.t_max.toFixed(6)} GeV²
            </span>

            <span class="data-label"
              use:tooltip={{ text: "External particle masses [m₁, m₂, m₃, m₄] entering the boundary calculation [GeV]" }}>
              masses
            </span>
            <span class="data-value mono" role="button" tabindex="0"
              on:contextmenu|stopPropagation={(e) =>
                openValueContext(
                  e,
                  "masses",
                  `[${report!.mandelstam_boundaries!.masses.map((m) => m.toFixed(4)).join(", ")}] GeV`,
                )}>
              [{report.mandelstam_boundaries.masses.map((m) => m.toFixed(4)).join(", ")}] GeV
            </span>
          </div>
        </div>
      {/if}
    {/if}

    <!-- ── Direct Mandelstam Variables ────────────────────────────── -->
    {#if mvars}
      <div class="section">
        <h4>Mandelstam Variables</h4>
        <div class="mandelstam-display">
          <div class="mvar" role="button" tabindex="0"
            on:contextmenu|stopPropagation={(e) => openMandelstamContext(e, "s", mvars!.s)}>
            <span class="mvar-label"
              use:tooltip={{ text: "s = (p₁ + p₂)² — square of total CM energy. s > 0 for a physical reaction." }}>
              s
            </span>
            <span class="mvar-value">{mvars.s.toFixed(6)}</span>
            <span class="mvar-unit">GeV²</span>
          </div>
          <div class="mvar" role="button" tabindex="0"
            on:contextmenu|stopPropagation={(e) => openMandelstamContext(e, "t", mvars!.t)}>
            <span class="mvar-label"
              use:tooltip={{ text: "t = (p₁ − p₃)² — square of 4-momentum transfer; t ≤ 0 for physical scattering." }}>
              t
            </span>
            <span class="mvar-value">{mvars.t.toFixed(6)}</span>
            <span class="mvar-unit">GeV²</span>
          </div>
          <div class="mvar" role="button" tabindex="0"
            on:contextmenu|stopPropagation={(e) => openMandelstamContext(e, "u", mvars!.u)}>
            <span class="mvar-label"
              use:tooltip={{ text: "u = (p₁ − p₄)² — crossed channel variable; s+t+u = Σ mᵢ² (Mandelstam identity)." }}>
              u
            </span>
            <span class="mvar-value">{mvars.u.toFixed(6)}</span>
            <span class="mvar-unit">GeV²</span>
          </div>
          <div class="mvar sum" role="button" tabindex="0"
            on:contextmenu|stopPropagation={(e) =>
              openMandelstamContext(e, "s+t+u", mvars!.s + mvars!.t + mvars!.u)}>
            <span class="mvar-label"
              use:tooltip={{ text: "Mandelstam identity check: s+t+u must equal Σ mᵢ² (sum of squared particle masses)" }}>
              s+t+u
            </span>
            <span class="mvar-value">{(mvars.s + mvars.t + mvars.u).toFixed(6)}</span>
            <span class="mvar-unit">= Σm²</span>
          </div>
        </div>
      </div>
    {/if}
  {/if}

  <!-- ── Decay Calculator Interop Annotation ────────────────────── -->
  {#if decayPayload?.selectedParticle}
    <div class="section interop-note">
      <h4>From Decay Calculator</h4>
      <div class="data-grid">
        <span class="data-label"
          use:tooltip={{ text: "Particle currently selected in the Decay Calculator widget" }}>
          Parent
        </span>
        <span class="data-value mono" role="button" tabindex="0"
          on:contextmenu|stopPropagation={(e) =>
            openValueContext(e, "Parent", decayPayload!.selectedParticle!)}>
          {decayPayload.selectedParticle}
        </span>

        {#if decayPayload.totalWidth != null}
          <span class="data-label"
            use:tooltip={{ text: "Total decay width Γ_total received from the Decay Calculator via the interop store" }}>
            Γ<sub>total</sub>
          </span>
          <span class="data-value mono" role="button" tabindex="0"
            on:contextmenu|stopPropagation={(e) =>
              openValueContext(e, "Γ_total", `${decayPayload!.totalWidth!.toExponential(4)} GeV`)}>
            {decayPayload.totalWidth.toExponential(4)} GeV
          </span>
        {/if}

        {#if decayPayload.lifetimeSeconds != null && decayPayload.lifetimeSeconds > 0}
          <span class="data-label"
            use:tooltip={{ text: "Mean lifetime τ = ℏ/Γ_total (ℏ = 6.582×10⁻²⁵ GeV·s)" }}>
            τ
          </span>
          <span class="data-value mono">{fmtLifetime(decayPayload.lifetimeSeconds)}</span>
        {/if}
      </div>
    </div>
  {/if}

  <!-- ── Known Thresholds & Resonances Reference ────────────────── -->
  <details class="resonance-panel">
    <summary use:tooltip={{ text: "Quick-reference table of well-known SM thresholds for comparison with computed threshold energy" }}>
      Known Thresholds &amp; Resonances
    </summary>
    <div class="resonance-grid">
      {#each KNOWN_THRESHOLDS as ref}
        <span class="res-label" use:tooltip={{ text: ref.description }}>{ref.label}</span>
        <span class="res-value" role="button" tabindex="0"
          on:contextmenu|stopPropagation={(e) =>
            openValueContext(e, ref.label, `${ref.sqrtS.toFixed(3)} GeV`)}>
          {ref.sqrtS.toFixed(3)} GeV
        </span>
        <span class="res-match" class:near={isNear(ref)}>{isNear(ref) ? "≈ match" : ""}</span>
      {/each}
    </div>
  </details>

</div>

<style>
  .kinematics-view {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    height: 100%;
    overflow-y: auto;
    min-height: 0;
    padding: 0.5rem;
    box-sizing: border-box;
  }

  h3 {
    margin: 0 0 0.25rem;
    font-size: 0.95rem;
    color: var(--color-text-primary);
    border-bottom: 1px solid var(--color-border);
    padding-bottom: 0.3rem;
    flex-shrink: 0;
  }

  h4 {
    margin: 0 0 0.2rem;
    font-size: 0.78rem;
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .hint {
    font-size: 0.78rem;
    color: var(--color-text-muted);
    font-style: italic;
    margin: 0;
  }

  .section {
    background: var(--color-bg-inset);
    border: 1px solid var(--color-border);
    padding: 0.4rem 0.5rem;
    border-radius: var(--radius-sm);
  }

  .section.interop-note {
    border-color: rgba(var(--color-accent-rgb), 0.3);
    background: rgba(var(--color-accent-rgb), 0.04);
  }

  .data-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.15rem 0.6rem;
    align-items: baseline;
  }

  .data-label {
    font-size: 0.72rem;
    color: var(--color-text-muted);
    text-align: right;
    white-space: nowrap;
    cursor: default;
    font-family: var(--font-mono);
    user-select: none;
  }

  .data-label sub { font-size: 0.6rem; }

  .data-value {
    font-size: 0.78rem;
    color: var(--color-text-primary);
    font-family: var(--font-mono);
    cursor: default;
    user-select: text;
  }

  .data-value.mono { font-size: 0.72rem; }

  .data-value.allowed {
    color: var(--color-success);
    font-weight: 600;
  }

  .data-value.forbidden {
    color: var(--color-error);
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
    background: var(--color-bg-surface);
    border: 1px solid var(--color-border);
    padding: 0.35rem 0.5rem;
    min-width: 4.2rem;
    border-radius: var(--radius-sm);
    cursor: default;
    transition: border-color var(--transition-fast);
  }

  .mvar:hover {
    border-color: var(--color-accent);
  }

  .mvar.sum {
    border-color: rgba(var(--color-accent-rgb), 0.4);
    background: rgba(var(--color-accent-rgb), 0.04);
  }

  .mvar-label {
    font-size: 0.82rem;
    font-weight: 700;
    color: var(--color-accent);
    font-family: var(--font-mono);
  }

  .mvar-value {
    font-size: 0.78rem;
    color: var(--hl-value);
    font-family: var(--font-mono);
  }

  .mvar-unit {
    font-size: 0.62rem;
    color: var(--color-text-muted);
  }

  /* ── Known Thresholds Collapsible ───────────────────────────────── */
  .resonance-panel {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: 0.72rem;
    flex-shrink: 0;
  }

  .resonance-panel summary {
    padding: 0.3rem 0.5rem;
    cursor: pointer;
    color: var(--color-text-muted);
    user-select: none;
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    list-style: none;
  }

  .resonance-panel summary::-webkit-details-marker { display: none; }
  .resonance-panel summary::before { content: "▶ "; font-size: 0.6rem; }
  .resonance-panel[open] summary::before { content: "▼ "; }
  .resonance-panel summary:hover { color: var(--color-text-primary); }

  .resonance-grid {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 0.1rem 0.5rem;
    padding: 0.35rem 0.5rem;
    align-items: baseline;
  }

  .res-label {
    color: var(--color-text-muted);
    font-family: var(--font-mono);
    white-space: nowrap;
    cursor: default;
  }

  .res-value {
    color: var(--color-text-primary);
    font-family: var(--font-mono);
    cursor: default;
    user-select: text;
  }

  .res-match { font-size: 0.62rem; color: var(--color-text-muted); min-width: 4rem; text-align: right; }
  .res-match.near { color: var(--color-success); font-weight: 600; }
</style>
