<!--
  SPIRE - External Models (Phase 33)

  Import panel for external theory model files:
    • SLHA spectrum files (.slha) - mass spectra, decay tables, mixing matrices
    • UFO model archives - FeynRules/SARAH output (particles.py, vertices.py, etc.)
    • NLO counterterm generator - automatic δZ/δm/δg from tree-level terms

  Features:
    - Drag-and-drop / file-picker upload zones for SLHA and UFO
    - Parsed SLHA block browser with mass table and decay table views
    - UFO particle/vertex summary with merge into TheoreticalModel
    - NLO counterterm derivation with LaTeX display
-->
<script lang="ts">
  import { appendLog, theoreticalModel } from "$lib/stores/physicsStore";
  import { importSlhaString, importUfoModel, deriveCounterterms } from "$lib/api";
  import { addCitations } from "$lib/core/services/CitationRegistry";
  import MathRenderer from "$lib/components/math/MathRenderer.svelte";
  import type {
    SlhaDocument,
    SlhaBlock,
    SlhaDecay,
    UfoModel,
    UfoFileContents,
    TheoreticalModel,
    CountertermResult,
    CountertermEntry,
    FieldSpin,
    ExternalField,
  } from "$lib/types/spire";

  // ---------------------------------------------------------------------------
  // Tab State
  // ---------------------------------------------------------------------------
  type TabId = "slha" | "ufo" | "nlo";
  let activeTab: TabId = "slha";
  /** Whether to show rendered math or raw LaTeX source (NLO results). */
  let latexViewMode: "rendered" | "raw" = "rendered";

  // ---------------------------------------------------------------------------
  // SLHA State
  // ---------------------------------------------------------------------------
  let slhaText: string = "";
  let slhaDoc: SlhaDocument | null = null;
  let slhaError: string = "";
  let slhaErrorHints: string[] = [];
  let slhaLoading: boolean = false;
  let slhaBlockFilter: string = "";

  function buildImportErrorHints(message: string, kind: "slha" | "ufo" | "nlo"): string[] {
    const lower = message.toLowerCase();
    const hints: string[] = [];
    const lineCol = message.match(/line\s*(\d+)(?:\s*[,;:]?\s*col(?:umn)?\s*(\d+))?/i);
    if (lineCol) {
      hints.push(`Parser location: line ${lineCol[1]}, column ${lineCol[2] ?? "?"}.`);
    }

    if (kind === "slha") {
      if (lower.includes("block")) hints.push("Ensure each SLHA block starts with 'BLOCK <NAME>' and uses numeric entries.");
      if (lower.includes("decay")) hints.push("Check DECAY headers and daughter multiplicity columns in each channel row.");
      hints.push("Try loading only MASS and DECAY blocks first to isolate malformed sections.");
    }

    if (kind === "ufo") {
      if (lower.includes("particles")) hints.push("particles.py is mandatory. Re-export UFO package and upload particles.py first.");
      if (lower.includes("syntax") || lower.includes("parse")) hints.push("Verify Python syntax in uploaded files (trailing commas, unmatched brackets, indentation)." );
      if (lower.includes("name") || lower.includes("vertex")) hints.push("Check that particle names referenced in vertices.py exist in particles.py.");
      hints.push("Upload the core set: particles.py, vertices.py, couplings.py, parameters.py, lorentz.py.");
    }

    if (kind === "nlo") {
      hints.push("Verify field IDs in the external leg list exist in the known fields table.");
      hints.push("Use explicit multiplication symbols in the term (e.g. e * psi_bar * gamma_mu * psi * A^mu).");
    }

    return hints;
  }

  async function handleSlhaImport(): Promise<void> {
    if (!slhaText.trim()) {
      slhaError = "Paste or upload an SLHA file first.";
      return;
    }
    slhaError = "";
    slhaErrorHints = [];
    slhaLoading = true;
    try {
      slhaDoc = await importSlhaString(slhaText);
      const nBlocks = Object.keys(slhaDoc.blocks).length;
      const nDecays = Object.keys(slhaDoc.decays).length;
      appendLog(`SLHA imported: ${nBlocks} blocks, ${nDecays} decay tables`);
    } catch (e: unknown) {
      slhaError = e instanceof Error ? e.message : String(e);
      slhaErrorHints = buildImportErrorHints(slhaError, "slha");
      appendLog(`SLHA import error: ${slhaError}`);
    } finally {
      slhaLoading = false;
    }
  }

  async function handleSlhaFile(event: Event): Promise<void> {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    slhaText = await file.text();
    await handleSlhaImport();
  }

  function handleSlhaDrop(event: DragEvent): void {
    event.preventDefault();
    const file = event.dataTransfer?.files?.[0];
    if (!file) return;
    file.text().then((text) => {
      slhaText = text;
      handleSlhaImport();
    });
  }

  $: filteredBlocks = slhaDoc
    ? Object.values(slhaDoc.blocks).filter(
        (b: SlhaBlock) =>
          !slhaBlockFilter || b.name.toLowerCase().includes(slhaBlockFilter.toLowerCase()),
      )
    : [];

  $: slhaDecays = slhaDoc
    ? Object.values(slhaDoc.decays).sort((a: SlhaDecay, b: SlhaDecay) => a.pdg_id - b.pdg_id)
    : [];

  // ---------------------------------------------------------------------------
  // UFO State
  // ---------------------------------------------------------------------------
  let ufoFiles: UfoFileContents = {
    particles_py: null,
    vertices_py: null,
    couplings_py: null,
    parameters_py: null,
    lorentz_py: null,
  };
  let ufoModelName: string = "Imported UFO Model";
  let ufoModel: UfoModel | null = null;
  let ufoTheoreticalModel: TheoreticalModel | null = null;
  let ufoError: string = "";
  let ufoErrorHints: string[] = [];
  let ufoLoading: boolean = false;

  const UFO_FILE_KEYS: { key: keyof UfoFileContents; label: string }[] = [
    { key: "particles_py", label: "particles.py" },
    { key: "vertices_py", label: "vertices.py" },
    { key: "couplings_py", label: "couplings.py" },
    { key: "parameters_py", label: "parameters.py" },
    { key: "lorentz_py", label: "lorentz.py" },
  ];

  async function handleUfoFileUpload(
    key: keyof UfoFileContents,
    event: Event,
  ): Promise<void> {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    ufoFiles[key] = await file.text();
    ufoFiles = ufoFiles; // trigger reactivity
  }

  async function handleUfoImport(): Promise<void> {
    const hasAny = Object.values(ufoFiles).some((v) => v !== null);
    if (!hasAny) {
      ufoError = "Upload at least one UFO .py file.";
      return;
    }
    ufoError = "";
    ufoErrorHints = [];
    ufoLoading = true;
    try {
      const [model, theoretical] = await importUfoModel(ufoFiles, ufoModelName);
      ufoModel = model;
      ufoTheoreticalModel = theoretical;
      addCitations(["degrande2012"]);
      appendLog(
        `UFO imported: ${model.particles.length} particles, ${model.vertices.length} vertices`,
      );
    } catch (e: unknown) {
      ufoError = e instanceof Error ? e.message : String(e);
      ufoErrorHints = buildImportErrorHints(ufoError, "ufo");
      appendLog(`UFO import error: ${ufoError}`);
    } finally {
      ufoLoading = false;
    }
  }

  function applyUfoModel(): void {
    if (ufoTheoreticalModel) {
      theoreticalModel.set(ufoTheoreticalModel);
      appendLog(`Applied UFO model "${ufoModelName}" as active theoretical model`);
    }
  }

  // ---------------------------------------------------------------------------
  // NLO Counterterm State
  // ---------------------------------------------------------------------------
  let nloInput: string = "";
  let nloFields: { id: string; spin: FieldSpin }[] = [
    { id: "psi", spin: "Fermion" },
    { id: "A", spin: "Vector" },
  ];
  let nloExternalFields: ExternalField[] = [
    { field_id: "psi", is_adjoint: true },
    { field_id: "psi", is_adjoint: false },
    { field_id: "A", is_adjoint: false },
  ];
  let nloResult: CountertermResult | null = null;
  let nloError: string = "";
  let nloLoading: boolean = false;

  let newFieldId: string = "";
  let newFieldSpin: FieldSpin = "Scalar";

  function addNloField(): void {
    if (newFieldId.trim()) {
      nloFields = [...nloFields, { id: newFieldId.trim(), spin: newFieldSpin }];
      newFieldId = "";
    }
  }

  function removeNloField(idx: number): void {
    nloFields = nloFields.filter((_, i) => i !== idx);
  }

  let newExtFieldId: string = "";
  let newExtAdjoint: boolean = false;

  function addExtField(): void {
    if (newExtFieldId.trim()) {
      nloExternalFields = [
        ...nloExternalFields,
        { field_id: newExtFieldId.trim(), is_adjoint: newExtAdjoint },
      ];
      newExtFieldId = "";
      newExtAdjoint = false;
    }
  }

  function removeExtField(idx: number): void {
    nloExternalFields = nloExternalFields.filter((_, i) => i !== idx);
  }

  async function handleNloDerive(): Promise<void> {
    if (!nloInput.trim()) {
      nloError = "Enter a Lagrangian term first.";
      return;
    }
    nloError = "";
    nloLoading = true;
    try {
      const knownFields: Record<string, FieldSpin> = {};
      for (const f of nloFields) {
        knownFields[f.id] = f.spin;
      }
      nloResult = await deriveCounterterms(nloInput, knownFields, nloExternalFields);
      addCitations(["kennedy1982", "bardin1999"]);
      appendLog(
        `NLO counterterms: ${nloResult.counterterms.length} terms, ` +
          `${nloResult.renorm_constants.length} constants`,
      );
    } catch (e: unknown) {
      nloError = e instanceof Error ? e.message : String(e);
      appendLog(`NLO error: ${nloError}`);
    } finally {
      nloLoading = false;
    }
  }
</script>

<!-- ========================================================================= -->
<!-- Template                                                                  -->
<!-- ========================================================================= -->

<div class="external-models">
  <!-- Tab Bar -->
  <nav class="tab-bar">
    <button
      class="tab-btn"
      class:active={activeTab === "slha"}
      on:click={() => (activeTab = "slha")}
    >
      SLHA Spectrum
    </button>
    <button
      class="tab-btn"
      class:active={activeTab === "ufo"}
      on:click={() => (activeTab = "ufo")}
    >
      UFO Model
    </button>
    <button
      class="tab-btn"
      class:active={activeTab === "nlo"}
      on:click={() => (activeTab = "nlo")}
    >
      NLO Counterterms
    </button>
  </nav>

  <!-- ─── SLHA Tab ─── -->
  {#if activeTab === "slha"}
    <section class="tab-panel">
      <h3>SLHA Spectrum Import</h3>
      <p class="hint">
        Upload or paste a SUSY Les Houches Accord (.slha) file to inspect mass spectra,
        decay widths, and mixing matrices.
      </p>

      <!-- Upload zone -->
      <div
        class="drop-zone"
        on:dragover|preventDefault
        on:drop={handleSlhaDrop}
        role="button"
        tabindex="0"
      >
        <span class="drop-icon">SLHA</span>
        <span>Drop .slha file here or</span>
        <label class="file-label">
          Browse…
          <input type="file" accept=".slha,.txt,.dat" on:change={handleSlhaFile} hidden />
        </label>
      </div>

      <!-- Text area -->
      <textarea
        class="slha-input"
        bind:value={slhaText}
        placeholder="Or paste SLHA content here…"
        rows="8"
      ></textarea>

      <button class="btn-primary" on:click={handleSlhaImport} disabled={slhaLoading}>
        {slhaLoading ? "Parsing…" : "Parse SLHA"}
      </button>

      {#if slhaError}
        <div class="error-msg">{slhaError}</div>
        {#if slhaErrorHints.length > 0}
          <ul class="error-hints">
            {#each slhaErrorHints as hint}
              <li>{hint}</li>
            {/each}
          </ul>
        {/if}
      {/if}

      <!-- Results -->
      {#if slhaDoc}
        <div class="slha-results">
          <!-- Block browser -->
          <div class="section">
            <h4>Blocks ({filteredBlocks.length})</h4>
            <input
              type="text"
              class="filter-input"
              bind:value={slhaBlockFilter}
              placeholder="Filter blocks…"
            />
            <div class="block-list">
              {#each filteredBlocks as block (block.name)}
                <details class="block-details">
                  <summary>
                    <strong>{block.name}</strong>
                    {#if block.scale !== null}
                      <span class="scale">Q = {block.scale.toExponential(3)}</span>
                    {/if}
                  </summary>
                  <table class="data-table">
                    <thead>
                      <tr><th>Index</th><th>Value</th></tr>
                    </thead>
                    <tbody>
                      {#each Object.entries(block.entries) as [key, value]}
                        <tr><td>{key}</td><td>{Number(value).toPrecision(8)}</td></tr>
                      {/each}
                    </tbody>
                  </table>
                </details>
              {/each}
            </div>
          </div>

          <!-- Decay tables -->
          {#if slhaDecays.length > 0}
            <div class="section">
              <h4>Decay Tables ({slhaDecays.length})</h4>
              <div class="block-list">
                {#each slhaDecays as decay (decay.pdg_id)}
                  <details class="block-details">
                    <summary>
                      <strong>PDG {decay.pdg_id}</strong>
                      - Γ = {decay.total_width.toExponential(4)} GeV
                    </summary>
                    <table class="data-table">
                      <thead>
                        <tr><th>BR</th><th>Daughters</th></tr>
                      </thead>
                      <tbody>
                        {#each decay.channels as ch}
                          <tr>
                            <td>{ch.branching_ratio.toFixed(6)}</td>
                            <td>{ch.daughter_pdg_ids.join(", ")}</td>
                          </tr>
                        {/each}
                      </tbody>
                    </table>
                  </details>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      {/if}
    </section>
  {/if}

  <!-- ─── UFO Tab ─── -->
  {#if activeTab === "ufo"}
    <section class="tab-panel">
      <h3>UFO Model Import</h3>
      <p class="hint">
        Upload FeynRules / SARAH UFO output files. Each .py file is optional.
        At minimum, upload particles.py.
      </p>

      <div class="ufo-fields">
        <label class="ufo-name-label">
          Model name:
          <input type="text" bind:value={ufoModelName} class="ufo-name-input" />
        </label>
      </div>

      <div class="ufo-file-grid">
        {#each UFO_FILE_KEYS as { key, label } (key)}
          <div class="ufo-file-slot">
            <span class="ufo-file-label">{label}</span>
            <label class="file-label small">
              {ufoFiles[key] ? "✓ Loaded" : "Upload"}
              <input type="file" accept=".py" on:change={(e) => handleUfoFileUpload(key, e)} hidden />
            </label>
          </div>
        {/each}
      </div>

      <button class="btn-primary" on:click={handleUfoImport} disabled={ufoLoading}>
        {ufoLoading ? "Importing…" : "Import UFO Model"}
      </button>

      {#if ufoError}
        <div class="error-msg">{ufoError}</div>
        {#if ufoErrorHints.length > 0}
          <ul class="error-hints">
            {#each ufoErrorHints as hint}
              <li>{hint}</li>
            {/each}
          </ul>
        {/if}
      {/if}

      {#if ufoModel}
        <div class="ufo-results">
          <div class="section">
            <h4>Particles ({ufoModel.particles.length})</h4>
            <table class="data-table">
              <thead>
                <tr><th>PDG</th><th>Name</th><th>Spin</th><th>Color</th><th>Charge</th></tr>
              </thead>
              <tbody>
                {#each ufoModel.particles as p}
                  <tr>
                    <td>{p.pdg_code}</td>
                    <td>{p.name}</td>
                    <td>{p.spin}</td>
                    <td>{p.color}</td>
                    <td>{p.charge}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>

          <div class="section">
            <h4>Vertices ({ufoModel.vertices.length})</h4>
            <div class="vertex-list">
              {#each ufoModel.vertices.slice(0, 50) as vtx}
                <div class="vertex-chip">{vtx.name}: {vtx.particles.join("-")}</div>
              {/each}
              {#if ufoModel.vertices.length > 50}
                <div class="vertex-chip muted">
                  … and {ufoModel.vertices.length - 50} more
                </div>
              {/if}
            </div>
          </div>

          <button class="btn-secondary" on:click={applyUfoModel}>
            Apply as Active Model
          </button>
        </div>
      {/if}
    </section>
  {/if}

  <!-- ─── NLO Tab ─── -->
  {#if activeTab === "nlo"}
    <section class="tab-panel">
      <h3>NLO Counterterm Generator</h3>
      <p class="hint">
        Enter a tree-level Lagrangian term and define the field content.
        SPIRE will perform multiplicative renormalization
        (φ → √Z·φ, g → g + δg) and extract all 𝒪(δ) counterterm vertices.
      </p>

      <!-- Known fields -->
      <div class="section">
        <h4>Known Fields</h4>
        <div class="field-chips">
          {#each nloFields as f, i}
            <span class="field-chip">
              {f.id} ({f.spin})
              <button class="chip-remove" on:click={() => removeNloField(i)}>×</button>
            </span>
          {/each}
        </div>
        <div class="inline-form">
          <input type="text" bind:value={newFieldId} placeholder="field_id" class="small-input" />
          <select bind:value={newFieldSpin} class="small-select">
            <option value="Scalar">Scalar</option>
            <option value="Fermion">Fermion</option>
            <option value="Vector">Vector</option>
            <option value="ThreeHalf">Spin-3/2</option>
            <option value="Tensor2">Tensor</option>
          </select>
          <button class="btn-small" on:click={addNloField}>Add</button>
        </div>
      </div>

      <!-- External fields -->
      <div class="section">
        <h4>External Fields (legs)</h4>
        <div class="field-chips">
          {#each nloExternalFields as ef, i}
            <span class="field-chip">
              {ef.is_adjoint ? "†" : ""}{ef.field_id}
              <button class="chip-remove" on:click={() => removeExtField(i)}>×</button>
            </span>
          {/each}
        </div>
        <div class="inline-form">
          <input
            type="text"
            bind:value={newExtFieldId}
            placeholder="field_id"
            class="small-input"
          />
          <label class="checkbox-label">
            <input type="checkbox" bind:checked={newExtAdjoint} />
            Adjoint
          </label>
          <button class="btn-small" on:click={addExtField}>Add</button>
        </div>
      </div>

      <!-- Term input -->
      <div class="section">
        <h4>Tree-Level Term</h4>
        <input
          type="text"
          class="term-input"
          bind:value={nloInput}
          placeholder="e.g. e * psi_bar * gamma_mu * psi * A^mu"
        />
      </div>

      <button class="btn-primary" on:click={handleNloDerive} disabled={nloLoading}>
        {nloLoading ? "Deriving…" : "Derive Counterterms"}
      </button>

      {#if nloError}
        <div class="error-msg">{nloError}</div>
      {/if}

      {#if nloResult}
        <div class="nlo-results">
          <div class="nlo-results-header">
            <button class="btn-small" on:click={() => (latexViewMode = latexViewMode === "rendered" ? "raw" : "rendered")}>
              {latexViewMode === "rendered" ? "Raw LaTeX" : "Rendered Math"}
            </button>
          </div>
          <div class="section">
            <h4>Renormalization Constants ({nloResult.renorm_constants.length})</h4>
            <div class="const-list">
              {#each nloResult.renorm_constants as rc}
                <div class="const-chip">
                  <strong>{rc.name}</strong> ({rc.kind}) - δ({rc.original_parameter})
                </div>
              {/each}
            </div>
          </div>

          <div class="section">
            <h4>Counterterms ({nloResult.counterterms.length})</h4>
            {#each nloResult.counterterms as ct, i}
              <div class="counterterm-card">
                <div class="ct-header">Counterterm {i + 1}</div>
                <div class="ct-desc">{ct.description}</div>
                {#if ct.latex}
                  <div class="ct-latex"><MathRenderer latex={ct.latex} mode={latexViewMode} block={true} /></div>
                {/if}
                <div class="ct-params">
                  δ-parameters: {ct.delta_parameters.join(", ")}
                </div>
              </div>
            {/each}
          </div>

          {#if nloResult.counterterm_rules.length > 0}
            <div class="section">
              <h4>Counterterm Feynman Rules ({nloResult.counterterm_rules.length})</h4>
              {#each nloResult.counterterm_rules as rule, i}
                <div class="counterterm-card">
                  <div class="ct-header">{rule.n_legs}-point vertex (SF = {rule.symmetry_factor})</div>
                  <div class="ct-latex"><MathRenderer latex={rule.latex} mode={latexViewMode} block={true} /></div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </section>
  {/if}
</div>

<!-- ========================================================================= -->
<!-- Styles                                                                    -->
<!-- ========================================================================= -->
<style>
  .external-models {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    height: 100%;
    overflow: hidden;
    font-size: 0.85rem;
  }

  /* ── Tab Bar ── */
  .tab-bar {
    display: flex;
    gap: 2px;
    border-bottom: 1px solid var(--border, #444);
    flex-shrink: 0;
  }

  .tab-btn {
    padding: 0.4rem 0.8rem;
    border: none;
    background: transparent;
    color: var(--text-muted, var(--color-text-muted));
    cursor: pointer;
    border-bottom: 2px solid transparent;
    font-size: 0.8rem;
    transition: color 0.15s, border-color 0.15s;
  }
  .tab-btn:hover {
    color: var(--text, #eee);
  }
  .tab-btn.active {
    color: var(--hl-accent, #58a6ff);
    border-bottom-color: var(--hl-accent, #58a6ff);
  }

  /* ── Tab Panel ── */
  .tab-panel {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
  }

  .tab-panel h3 {
    margin: 0 0 0.25rem;
    font-size: 0.95rem;
  }

  .hint {
    color: var(--text-muted, var(--color-text-muted));
    margin: 0 0 0.5rem;
    font-size: 0.78rem;
    line-height: 1.4;
  }

  /* ── Drop Zone ── */
  .drop-zone {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border: 2px dashed var(--border, var(--color-text-muted));
    border-radius: 6px;
    margin-bottom: 0.5rem;
    color: var(--text-muted, var(--color-text-muted));
    transition: border-color 0.2s;
  }
  .drop-zone:hover,
  .drop-zone:focus-within {
    border-color: var(--hl-accent, #58a6ff);
  }
  .drop-icon {
    font-size: 1.3rem;
  }

  /* ── Buttons ── */
  .btn-primary {
    padding: 0.4rem 1rem;
    background: var(--hl-accent, #238636);
    color: var(--color-text-primary);
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8rem;
  }
  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .btn-secondary {
    padding: 0.35rem 0.8rem;
    background: transparent;
    color: var(--hl-accent, #58a6ff);
    border: 1px solid var(--hl-accent, #58a6ff);
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8rem;
    margin-top: 0.5rem;
  }
  .btn-small {
    padding: 0.2rem 0.5rem;
    background: var(--bg-secondary, #2d2d2d);
    color: var(--text, #eee);
    border: 1px solid var(--border, var(--color-text-muted));
    border-radius: 3px;
    cursor: pointer;
    font-size: 0.78rem;
  }

  .file-label {
    color: var(--hl-accent, #58a6ff);
    cursor: pointer;
    text-decoration: underline;
    font-size: 0.82rem;
  }
  .file-label.small {
    font-size: 0.78rem;
  }

  /* ── Inputs ── */
  .slha-input,
  .term-input,
  .filter-input,
  .ufo-name-input {
    width: 100%;
    font-family: "Fira Code", "Consolas", monospace;
    font-size: 0.78rem;
    padding: 0.35rem 0.5rem;
    border: 1px solid var(--border, var(--color-text-muted));
    border-radius: 4px;
    background: var(--bg-secondary, #1e1e1e);
    color: var(--text, #eee);
    box-sizing: border-box;
    margin-bottom: 0.5rem;
  }
  .slha-input {
    resize: vertical;
  }

  .small-input {
    width: 100px;
    padding: 0.2rem 0.4rem;
    border: 1px solid var(--border, var(--color-text-muted));
    border-radius: 3px;
    background: var(--bg-secondary, #1e1e1e);
    color: var(--text, #eee);
    font-size: 0.78rem;
    font-family: "Fira Code", "Consolas", monospace;
  }
  .small-select {
    padding: 0.2rem 0.3rem;
    border: 1px solid var(--border, var(--color-text-muted));
    border-radius: 3px;
    background: var(--bg-secondary, #1e1e1e);
    color: var(--text, #eee);
    font-size: 0.78rem;
  }

  /* ── Error ── */
  .error-msg {
    color: var(--hl-error, #f85149);
    background: rgba(248, 81, 73, 0.1);
    padding: 0.3rem 0.5rem;
    border-radius: 4px;
    margin: 0.5rem 0 0.25rem;
    font-size: 0.78rem;
  }

  .error-hints {
    margin: 0 0 0.5rem;
    padding-left: 1rem;
    color: var(--text-muted, var(--color-text-muted));
    font-size: 0.74rem;
    line-height: 1.35;
  }

  /* ── Sections & Data ── */
  .section {
    margin: 0.75rem 0;
  }
  .section h4 {
    margin: 0 0 0.3rem;
    font-size: 0.85rem;
    color: var(--text, #eee);
  }

  .block-list {
    max-height: 250px;
    overflow-y: auto;
  }
  .block-details {
    margin-bottom: 0.3rem;
  }
  .block-details summary {
    cursor: pointer;
    padding: 0.2rem 0.4rem;
    background: var(--bg-secondary, #252525);
    border-radius: 3px;
    font-size: 0.8rem;
  }
  .scale {
    color: var(--text-muted, var(--color-text-muted));
    font-size: 0.75rem;
    margin-left: 0.5rem;
  }

  .data-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.78rem;
    margin-top: 0.3rem;
  }
  .data-table th,
  .data-table td {
    padding: 0.15rem 0.4rem;
    border-bottom: 1px solid var(--border, #333);
    text-align: left;
  }
  .data-table th {
    color: var(--text-muted, var(--color-text-muted));
    font-weight: 600;
  }

  /* ── UFO ── */
  .ufo-fields {
    margin-bottom: 0.5rem;
  }
  .ufo-name-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.8rem;
    color: var(--text-muted, var(--color-text-muted));
  }

  .ufo-file-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }
  .ufo-file-slot {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.3rem;
    padding: 0.5rem;
    border: 1px solid var(--border, #444);
    border-radius: 4px;
    background: var(--bg-secondary, #1e1e1e);
  }
  .ufo-file-label {
    font-family: "Fira Code", "Consolas", monospace;
    font-size: 0.75rem;
    color: var(--text-muted, var(--color-text-muted));
  }

  .vertex-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
  }
  .vertex-chip {
    padding: 0.15rem 0.4rem;
    background: var(--bg-secondary, #252525);
    border: 1px solid var(--border, #444);
    border-radius: 3px;
    font-size: 0.75rem;
    font-family: "Fira Code", "Consolas", monospace;
  }
  .vertex-chip.muted {
    color: var(--text-muted, var(--color-text-muted));
    font-style: italic;
  }

  /* ── NLO ── */
  .field-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    margin-bottom: 0.3rem;
  }
  .field-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.15rem 0.45rem;
    background: var(--bg-secondary, #252525);
    border: 1px solid var(--border, #444);
    border-radius: 3px;
    font-size: 0.78rem;
    font-family: "Fira Code", "Consolas", monospace;
  }
  .chip-remove {
    background: none;
    border: none;
    color: var(--hl-error, #f85149);
    cursor: pointer;
    padding: 0;
    font-size: 0.85rem;
    line-height: 1;
  }

  .inline-form {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    margin-bottom: 0.3rem;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.2rem;
    font-size: 0.78rem;
    color: var(--text-muted, var(--color-text-muted));
  }

  .const-list {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .const-chip {
    padding: 0.2rem 0.5rem;
    background: var(--bg-secondary, #252525);
    border: 1px solid var(--border, #444);
    border-radius: 3px;
    font-size: 0.78rem;
  }

  .counterterm-card {
    padding: 0.5rem;
    background: var(--bg-secondary, #1e1e1e);
    border: 1px solid var(--border, #444);
    border-radius: 4px;
    margin-bottom: 0.4rem;
  }
  .ct-header {
    font-weight: 600;
    font-size: 0.82rem;
    margin-bottom: 0.2rem;
    color: var(--hl-accent, #58a6ff);
  }
  .ct-desc {
    font-size: 0.78rem;
    color: var(--text-muted, var(--color-text-muted));
    margin-bottom: 0.15rem;
  }
  .ct-latex {
    padding: 0.2rem 0.4rem;
    background: var(--bg, #161616);
    border-radius: 3px;
    margin: 0.2rem 0;
    overflow-x: auto;
  }
  .nlo-results-header {
    display: flex;
    justify-content: flex-end;
    margin-bottom: 0.5rem;
  }
  .ct-params {
    font-size: 0.75rem;
    color: var(--text-muted, var(--color-text-muted));
  }
</style>
