<!--
  SPIRE - Lagrangian Workbench

  Interactive workbench for defining Lagrangian interaction terms,
  validating theoretical consistency (Lorentz invariance, gauge singlet,
  Hermiticity), deriving Feynman rules via functional differentiation,
  and computing RGE coupling flow.

  Features:
    - Term builder with known-field definitions
    - Live parsing + LaTeX rendering
    - Validation badges (Lorentz ✓, Gauge ✓, Hermitian ✓, Renormalisable ✓)
    - Vertex rule derivation with external field selection
    - RGE β-function plotter with Chart.js
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { appendLog } from "$lib/stores/physicsStore";
  import {
    parseLagrangianTerm,
    validateLagrangianTerm,
    deriveVertexRuleFromAst,
    runRgeFlow,
  } from "$lib/api";
  import { registerCommand, unregisterCommand } from "$lib/core/services/CommandRegistry";
  import SpireNumberInput from "$lib/components/ui/SpireNumberInput.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { addCitations } from "$lib/core/services/CitationRegistry";
  import type {
    FieldSpin,
    LagrangianExpr,
    ExternalField,
    DerivedVertexRule,
    ValidationResult,
    RgeFlowConfig,
    RgeFlowResult,
  } from "$lib/types/spire";
  import {
    Chart,
    LineController,
    LineElement,
    PointElement,
    CategoryScale,
    LinearScale,
    LogarithmicScale,
    Tooltip,
    Title,
    Legend,
  } from "chart.js";

  Chart.register(
    LineController,
    LineElement,
    PointElement,
    CategoryScale,
    LinearScale,
    LogarithmicScale,
    Tooltip,
    Title,
    Legend,
  );

  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------

  /** Known fields (field_id → spin). */
  let knownFields: { id: string; spin: FieldSpin }[] = [
    { id: "psi", spin: "Fermion" },
    { id: "A", spin: "Vector" },
    { id: "phi", spin: "Scalar" },
  ];

  /** Current Lagrangian term input. */
  let termInput = "e * psibar * gamma^mu * psi * A_mu";

  /** Parsed AST (if successful). */
  let parsedExpr: LagrangianExpr | null = null;
  let parseError = "";

  /** Validation result. */
  let validation: ValidationResult | null = null;

  /** External fields for vertex derivation. */
  let externals: ExternalField[] = [
    { field_id: "psi", is_adjoint: true },
    { field_id: "psi", is_adjoint: false },
    { field_id: "A", is_adjoint: false },
  ];

  /** Derived vertex rule. */
  let derivedRule: DerivedVertexRule | null = null;
  let deriveError = "";

  // --- RGE ---
  let rgeCouplingName = "alpha_s";
  let rgeBetaScript = "let b0 = 7.0; -b0 * g * g * g / (16.0 * PI * PI)";
  let rgeInitialValue = 0.3;
  let rgeMuMin = 2;
  let rgeMuMax = 1000;
  let rgeNPoints = 200;
  let rgeResult: RgeFlowResult | null = null;
  let rgeError = "";

  /** Active tab: "term" | "rge" */
  let activeTab: "term" | "rge" = "term";

  // --- New field form ---
  let newFieldId = "";
  let newFieldSpin: FieldSpin = "Scalar";

  // Chart reference
  let rgeCanvas: HTMLCanvasElement;
  let rgeChart: Chart | null = null;

  // ---------------------------------------------------------------------------
  // Field Management
  // ---------------------------------------------------------------------------

  function addField(): void {
    if (!newFieldId.trim()) return;
    if (knownFields.some((f) => f.id === newFieldId.trim())) return;
    knownFields = [...knownFields, { id: newFieldId.trim(), spin: newFieldSpin }];
    newFieldId = "";
  }

  function removeField(id: string): void {
    knownFields = knownFields.filter((f) => f.id !== id);
  }

  function buildFieldMap(): Record<string, FieldSpin> {
    const m: Record<string, FieldSpin> = {};
    for (const f of knownFields) m[f.id] = f.spin;
    return m;
  }

  // ---------------------------------------------------------------------------
  // Parse + Validate
  // ---------------------------------------------------------------------------

  async function parseAndValidate(): Promise<void> {
    parseError = "";
    parsedExpr = null;
    validation = null;
    derivedRule = null;
    deriveError = "";

    const fields = buildFieldMap();

    try {
      parsedExpr = await parseLagrangianTerm(termInput, fields);
      appendLog(`✓ Parsed: ${termInput}`);
    } catch (e: unknown) {
      parseError = e instanceof Error ? e.message : String(e);
      appendLog(`✗ Parse error: ${parseError}`);
      return;
    }

    try {
      validation = await validateLagrangianTerm(termInput, fields);
      appendLog(
        `Validation: Lorentz=${validation.is_lorentz_scalar} Gauge=${validation.is_gauge_singlet} Hermitian=${validation.is_hermitian} Dim=${validation.mass_dimension}`,
      );
    } catch (e: unknown) {
      appendLog(`✗ Validation error: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  // ---------------------------------------------------------------------------
  // Derive Vertex Rule
  // ---------------------------------------------------------------------------

  async function deriveVertex(): Promise<void> {
    deriveError = "";
    derivedRule = null;

    if (!parsedExpr) {
      deriveError = "Parse the term first.";
      return;
    }

    const fields = buildFieldMap();

    try {
      derivedRule = await deriveVertexRuleFromAst(termInput, fields, externals);
      appendLog(`✓ Derived vertex: ${derivedRule.latex}`);
    } catch (e: unknown) {
      deriveError = e instanceof Error ? e.message : String(e);
      appendLog(`✗ Derivation error: ${deriveError}`);
    }
  }

  // ---------------------------------------------------------------------------
  // External Fields
  // ---------------------------------------------------------------------------

  function addExternal(): void {
    externals = [...externals, { field_id: "", is_adjoint: false }];
  }

  function removeExternal(idx: number): void {
    externals = externals.filter((_, i) => i !== idx);
  }

  // ---------------------------------------------------------------------------
  // RGE Flow
  // ---------------------------------------------------------------------------

  async function computeRge(): Promise<void> {
    rgeError = "";
    rgeResult = null;

    const config: RgeFlowConfig = {
      coupling_name: rgeCouplingName,
      beta_script: rgeBetaScript,
      initial_value: rgeInitialValue,
      mu_min: rgeMuMin,
      mu_max: rgeMuMax,
      n_points: rgeNPoints,
    };

    try {
      rgeResult = await runRgeFlow(config);
      addCitations(["machacek1984", "gross1973"]);
      appendLog(`✓ RGE flow computed: ${rgeResult.coupling_name}, ${rgeResult.mu_values.length} points`);
      plotRge();
    } catch (e: unknown) {
      rgeError = e instanceof Error ? e.message : String(e);
      appendLog(`✗ RGE error: ${rgeError}`);
    }
  }

  function plotRge(): void {
    if (!rgeResult || !rgeCanvas) return;

    if (rgeChart) rgeChart.destroy();

    const ctx = rgeCanvas.getContext("2d");
    if (!ctx) return;

    rgeChart = new Chart(ctx, {
      type: "line",
      data: {
        labels: rgeResult.mu_values.map((mu) => mu.toFixed(1)),
        datasets: [
          {
            label: rgeResult.coupling_name,
            data: rgeResult.coupling_values,
            borderColor: "#4fc3f7",
            backgroundColor: "rgba(79,195,247,0.1)",
            borderWidth: 2,
            pointRadius: 0,
            fill: true,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        scales: {
          x: {
            type: "logarithmic",
            title: { display: true, text: "μ (GeV)", color: "var(--color-text-muted)" },
            ticks: { color: "var(--color-text-muted)" },
            grid: { color: "rgba(255,255,255,0.05)" },
          },
          y: {
            title: {
              display: true,
              text: rgeResult.coupling_name,
              color: "var(--color-text-muted)",
            },
            ticks: { color: "var(--color-text-muted)" },
            grid: { color: "rgba(255,255,255,0.05)" },
          },
        },
        plugins: {
          legend: { labels: { color: "var(--color-text-primary)" } },
          title: {
            display: true,
            text: `RGE Flow: ${rgeResult.coupling_name}(μ)`,
            color: "#eee",
          },
        },
      },
    });
  }

  // ---------------------------------------------------------------------------
  // Lifecycle - Command Registration
  // ---------------------------------------------------------------------------

  const LAGRANGIAN_CMD_IDS = [
    "spire.lagrangian.parse",
    "spire.lagrangian.derive_vertex",
    "spire.lagrangian.run_rge",
  ];

  onMount(() => {
    registerCommand({
      id: "spire.lagrangian.parse",
      title: "Parse Lagrangian Term",
      category: "Lagrangian",
      execute: () => parseAndValidate(),
    });
    registerCommand({
      id: "spire.lagrangian.derive_vertex",
      title: "Derive Vertex Rule",
      category: "Lagrangian",
      execute: () => deriveVertex(),
    });
    registerCommand({
      id: "spire.lagrangian.run_rge",
      title: "Compute RGE Flow",
      category: "Lagrangian",
      execute: () => computeRge(),
    });
  });

  onDestroy(() => {
    for (const id of LAGRANGIAN_CMD_IDS) unregisterCommand(id);
    if (rgeChart) rgeChart.destroy();
  });
</script>

<!-- ====================================================================== -->
<!-- TEMPLATE                                                                -->
<!-- ====================================================================== -->

<div class="lagrangian-workbench">
  <!-- Tab Selector -->
  <div class="tabs">
    <button
      class="tab"
      class:active={activeTab === "term"}
      on:click={() => (activeTab = "term")}
    >
      Term Builder
    </button>
    <button
      class="tab"
      class:active={activeTab === "rge"}
      on:click={() => (activeTab = "rge")}
    >
      RGE Flow
    </button>
  </div>

  <!-- ═══════════════════ Term Builder Tab ═══════════════════ -->
  {#if activeTab === "term"}
    <!-- Known Fields -->
    <section class="section">
      <h3>Known Fields</h3>
      <div class="field-list">
        {#each knownFields as field}
          <span class="field-chip" use:tooltip={{ text: field.spin }}>
            {field.id}
            <small>({field.spin})</small>
            <button class="chip-remove" on:click={() => removeField(field.id)}>×</button>
          </span>
        {/each}
      </div>
      <div class="add-field-row">
        <input
          type="text"
          bind:value={newFieldId}
          placeholder="Field ID"
          class="input-sm"
        />
        <select bind:value={newFieldSpin} class="input-sm">
          <option value="Scalar">Scalar</option>
          <option value="Fermion">Fermion</option>
          <option value="Vector">Vector</option>
          <option value="ThreeHalf">3/2</option>
          <option value="Tensor2">Tensor-2</option>
        </select>
        <button class="btn-sm" on:click={addField}>Add</button>
      </div>
    </section>

    <!-- Term Input -->
    <section class="section">
      <h3>Lagrangian Term</h3>
      <div class="term-input-row">
        <input
          type="text"
          bind:value={termInput}
          placeholder="e.g.  e * psibar * gamma^mu * psi * A_mu"
          class="input-term"
          on:keydown={(e) => e.key === "Enter" && parseAndValidate()}
        />
        <button class="btn-primary" on:click={parseAndValidate}>Parse &amp; Validate</button>
      </div>
      {#if parseError}
        <p class="error">{parseError}</p>
      {/if}
    </section>

    <!-- Validation Badges -->
    {#if validation}
      <section class="section">
        <h3>Consistency Checks</h3>
        <div class="badges">
          <span class="badge" class:pass={validation.is_lorentz_scalar} class:fail={!validation.is_lorentz_scalar}>
            {validation.is_lorentz_scalar ? "✓" : "✗"} Lorentz
          </span>
          <span class="badge" class:pass={validation.is_gauge_singlet} class:fail={!validation.is_gauge_singlet}>
            {validation.is_gauge_singlet ? "✓" : "✗"} Gauge
          </span>
          <span class="badge" class:pass={validation.is_hermitian} class:fail={!validation.is_hermitian}>
            {validation.is_hermitian ? "✓" : "✗"} Hermitian
          </span>
          <span class="badge" class:pass={validation.is_renormalisable} class:warn={!validation.is_renormalisable}>
            {validation.is_renormalisable ? "✓" : "!"} Dim-{validation.mass_dimension}
          </span>
        </div>
        <!-- Diagnostic messages -->
        {#if validation.messages.length > 0}
          <details class="diagnostics">
            <summary>Diagnostics ({validation.messages.length})</summary>
            <ul>
              {#each validation.messages as msg}
                <li class="diag-{msg.severity}">
                  <strong>[{msg.check}]</strong> {msg.message}
                </li>
              {/each}
            </ul>
          </details>
        {/if}
      </section>
    {/if}

    <!-- Vertex Derivation -->
    <section class="section">
      <h3>Derive Vertex Rule</h3>
      <p class="hint">Select the external fields to differentiate with respect to:</p>
      <div class="external-fields">
        {#each externals as ext, idx}
          <div class="ext-row">
            <input type="text" bind:value={ext.field_id} placeholder="field_id" class="input-sm" />
            <label class="adj-label">
              <input type="checkbox" bind:checked={ext.is_adjoint} />
              adjoint
            </label>
            <button class="chip-remove" on:click={() => removeExternal(idx)}>×</button>
          </div>
        {/each}
        <button class="btn-sm" on:click={addExternal}>+ Add External</button>
      </div>
      <button class="btn-primary" on:click={deriveVertex} disabled={!parsedExpr}>
        Derive Vertex
      </button>
      {#if deriveError}
        <p class="error">{deriveError}</p>
      {/if}
      {#if derivedRule}
        <div class="derived-rule">
          <div class="rule-latex">
            <strong>Vertex Factor:</strong>
            <code>{derivedRule.latex}</code>
          </div>
          <div class="rule-meta">
            <span>Legs: {derivedRule.n_legs}</span>
            <span>Symmetry factor: {derivedRule.symmetry_factor}</span>
            <span>Grassmann sign: {derivedRule.grassmann_sign > 0 ? "+" : "−"}</span>
          </div>
        </div>
      {/if}
    </section>

  <!-- ═══════════════════ RGE Flow Tab ═══════════════════ -->
  {:else if activeTab === "rge"}
    <section class="section">
      <h3>RGE Coupling Flow</h3>
      <div class="rge-form">
        <label>
          Coupling name
          <input type="text" bind:value={rgeCouplingName} class="input-sm" />
        </label>
        <label>
          Initial value
          <SpireNumberInput bind:value={rgeInitialValue} step={0.01} ariaLabel="RGE initial value" />
        </label>
        <label>
          μ min (GeV)
          <SpireNumberInput bind:value={rgeMuMin} step={0.1} ariaLabel="RGE minimum scale" />
        </label>
        <label>
          μ max (GeV)
          <SpireNumberInput bind:value={rgeMuMax} step={10} ariaLabel="RGE maximum scale" />
        </label>
        <label>
          Points
          <SpireNumberInput bind:value={rgeNPoints} step={10} ariaLabel="RGE point count" />
        </label>
      </div>
      <div class="beta-input">
        <label>
          β-function (Rhai script - variable: <code>g</code>, constant: <code>PI</code>)
          <textarea
            bind:value={rgeBetaScript}
            rows="4"
            class="input-beta"
            placeholder="let b0 = 7.0; -b0 * g * g * g / (16.0 * PI * PI)"
          ></textarea>
        </label>
      </div>
      <button class="btn-primary" on:click={computeRge}>Run RGE</button>
      {#if rgeError}
        <p class="error">{rgeError}</p>
      {/if}
    </section>

    <!-- RGE Chart -->
    <section class="section chart-section">
      <canvas bind:this={rgeCanvas} class="rge-canvas"></canvas>
      {#if !rgeResult}
        <p class="hint">Configure and run the RGE to see the coupling flow.</p>
      {/if}
    </section>
  {/if}
</div>

<!-- ====================================================================== -->
<!-- STYLES                                                                  -->
<!-- ====================================================================== -->

<style>
  .lagrangian-workbench {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.5rem;
    height: 100%;
    overflow-y: auto;
    font-size: 0.85rem;
    color: var(--fg, #ddd);
  }

  /* Tabs */
  .tabs {
    display: flex;
    gap: 2px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    margin-bottom: 0.25rem;
  }
  .tab {
    background: transparent;
    border: none;
    color: var(--color-text-muted);
    padding: 0.4rem 1rem;
    cursor: pointer;
    font-size: 0.8rem;
    border-bottom: 2px solid transparent;
    transition: all 0.15s;
  }
  .tab:hover {
    color: var(--color-text-primary);
  }
  .tab.active {
    color: #4fc3f7;
    border-bottom-color: #4fc3f7;
  }

  /* Sections */
  .section {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 6px;
    padding: 0.6rem;
  }
  .section h3 {
    margin: 0 0 0.4rem;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-text-muted);
  }

  /* Field chips */
  .field-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    margin-bottom: 0.4rem;
  }
  .field-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    background: rgba(79, 195, 247, 0.1);
    border: 1px solid rgba(79, 195, 247, 0.3);
    border-radius: 12px;
    padding: 0.15rem 0.5rem;
    font-size: 0.75rem;
    color: #4fc3f7;
  }
  .field-chip small {
    color: var(--color-text-muted);
    font-size: 0.65rem;
  }
  .chip-remove {
    background: none;
    border: none;
    color: var(--color-text-muted);
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0 2px;
    line-height: 1;
  }
  .chip-remove:hover {
    color: #ff5252;
  }

  /* Inputs */
  .add-field-row {
    display: flex;
    gap: 0.3rem;
    align-items: center;
  }
  .input-sm {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    color: #ddd;
    padding: 0.25rem 0.4rem;
    font-size: 0.75rem;
    font-family: inherit;
  }
  .input-term {
    flex: 1;
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    color: #eee;
    padding: 0.4rem 0.6rem;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 0.8rem;
  }
  .input-beta {
    width: 100%;
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 4px;
    color: #eee;
    padding: 0.4rem;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 0.75rem;
    resize: vertical;
  }
  .term-input-row {
    display: flex;
    gap: 0.4rem;
    align-items: center;
  }

  /* Buttons */
  .btn-sm {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 4px;
    color: var(--color-text-muted);
    padding: 0.2rem 0.5rem;
    font-size: 0.72rem;
    cursor: pointer;
    transition: background 0.15s;
  }
  .btn-sm:hover {
    background: rgba(255, 255, 255, 0.1);
  }
  .btn-primary {
    background: rgba(79, 195, 247, 0.15);
    border: 1px solid rgba(79, 195, 247, 0.4);
    border-radius: 4px;
    color: #4fc3f7;
    padding: 0.3rem 0.8rem;
    font-size: 0.78rem;
    cursor: pointer;
    transition: background 0.15s;
  }
  .btn-primary:hover {
    background: rgba(79, 195, 247, 0.25);
  }
  .btn-primary:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  /* Badges */
  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 0.2rem;
    padding: 0.2rem 0.6rem;
    border-radius: 12px;
    font-size: 0.72rem;
    font-weight: 500;
  }
  .badge.pass {
    background: rgba(76, 175, 80, 0.15);
    border: 1px solid rgba(76, 175, 80, 0.4);
    color: var(--color-success);
  }
  .badge.fail {
    background: rgba(244, 67, 54, 0.15);
    border: 1px solid rgba(244, 67, 54, 0.4);
    color: #f44336;
  }
  .badge.warn {
    background: rgba(255, 193, 7, 0.15);
    border: 1px solid rgba(255, 193, 7, 0.4);
    color: #ffc107;
  }

  /* Diagnostics */
  .diagnostics {
    margin-top: 0.4rem;
    font-size: 0.72rem;
  }
  .diagnostics summary {
    cursor: pointer;
    color: var(--color-text-muted);
  }
  .diagnostics ul {
    margin: 0.3rem 0;
    padding-left: 1.2rem;
  }
  .diagnostics li {
    margin-bottom: 0.15rem;
  }
  .diag-error {
    color: #f44336;
  }
  .diag-warning {
    color: #ffc107;
  }
  .diag-info {
    color: var(--color-text-muted);
  }

  /* External fields */
  .external-fields {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    margin-bottom: 0.4rem;
  }
  .ext-row {
    display: flex;
    gap: 0.3rem;
    align-items: center;
  }
  .adj-label {
    display: flex;
    align-items: center;
    gap: 0.2rem;
    font-size: 0.72rem;
    color: var(--color-text-muted);
  }

  /* Derived Rule */
  .derived-rule {
    margin-top: 0.5rem;
    background: rgba(76, 175, 80, 0.06);
    border: 1px solid rgba(76, 175, 80, 0.2);
    border-radius: 6px;
    padding: 0.5rem;
  }
  .rule-latex code {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 0.78rem;
    color: var(--color-success);
  }
  .rule-meta {
    display: flex;
    gap: 1rem;
    margin-top: 0.3rem;
    font-size: 0.7rem;
    color: var(--color-text-muted);
  }

  /* RGE form */
  .rge-form {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 0.4rem;
    margin-bottom: 0.5rem;
  }
  .rge-form label {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    font-size: 0.72rem;
    color: var(--color-text-muted);
  }
  .beta-input {
    margin-bottom: 0.5rem;
  }
  .beta-input label {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.72rem;
    color: var(--color-text-muted);
  }
  .beta-input code {
    color: #4fc3f7;
    font-size: 0.7rem;
  }

  /* Chart */
  .chart-section {
    flex: 1;
    min-height: 200px;
  }
  .rge-canvas {
    width: 100%;
    height: 100%;
    min-height: 180px;
  }

  /* Misc */
  .error {
    color: #f44336;
    font-size: 0.75rem;
    margin: 0.3rem 0 0;
  }
  .hint {
    color: var(--color-text-muted);
    font-size: 0.72rem;
    margin: 0.2rem 0;
  }
</style>
