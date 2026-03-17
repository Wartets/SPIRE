<!--
  SPIRE - Model Loader Component

  Loads a theoretical model from TOML definitions.
  Pre-populated with the Standard Model (particles + vertices).
  Supports "Custom" mode: editable TOML areas with LocalStorage persistence.
  Calls loadModel() via the Tauri IPC bridge and writes the
  result to the theoreticalModel store.
-->
<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { theoreticalModel, appendLog, activeFramework } from "$lib/stores/physicsStore";
  import { loadModel, exportModelUfo } from "$lib/api";
  import { registerCommand, unregisterCommand } from "$lib/core/services/CommandRegistry";
  import { addCitations } from "$lib/core/services/CitationRegistry";
  import HoverDef from "$lib/components/ui/HoverDef.svelte";
  import {
    DEFAULT_PARTICLES_TOML,
    DEFAULT_VERTICES_TOML,
  } from "$lib/data/defaults";
  import {
    particlesTomlInput,
    verticesTomlInput,
    modelNameInput,
  } from "$lib/stores/workspaceInputsStore";
  import {
    getWidgetUiSnapshot,
    setWidgetUiSnapshot,
  } from "$lib/stores/workspaceStore";
  import { publishWidgetInterop } from "$lib/stores/widgetInteropStore";
  import type { TheoreticalFramework } from "$lib/types/spire";

  // ---------------------------------------------------------------------------
  // LocalStorage keys
  // ---------------------------------------------------------------------------
  const LS_KEY_PARTICLES = "spire:custom_particles_toml";
  const LS_KEY_VERTICES = "spire:custom_vertices_toml";
  const LS_KEY_MODEL_NAME = "spire:custom_model_name";

  let loading: boolean = false;
  let errorMsg: string = "";
  let errorDetails: string[] = [];
  let fieldCount: number = 0;
  let vertexCount: number = 0;
  let lastLoadedAt = "";
  let modelFingerprint = "";
  let unstableFieldCount = 0;
  let massWindow = "-";
  let isCustom: boolean = false;
  let savedIndicator: string = "";
  let rootScroller: HTMLDivElement | null = null;

  const MODEL_LOADER_UI_KEY = "model-loader";

  function persistModelLoaderUi(patch: Record<string, unknown>): void {
    setWidgetUiSnapshot(MODEL_LOADER_UI_KEY, patch);
  }

  $: if ($theoreticalModel) {
    publishWidgetInterop("model", {
      modelName: $theoreticalModel.name,
      fieldCount: $theoreticalModel.fields.length,
      vertexCount: $theoreticalModel.vertex_factors.length,
      framework: $activeFramework,
    });
  }

  // ---------------------------------------------------------------------------
  // Lifecycle: restore custom data from LocalStorage on mount
  // ---------------------------------------------------------------------------
  const MODEL_CMD_IDS = [
    "spire.model.load",
    "spire.model.export_ufo",
  ];

  onMount(() => {
    registerCommand({
      id: "spire.model.load",
      title: "Load Theoretical Model",
      category: "Model",
      shortcut: "Mod+Shift+L",
      execute: () => handleLoad(),
      pinned: true,
      icon: "M",
    });
    registerCommand({
      id: "spire.model.export_ufo",
      title: "Export Model (UFO)",
      category: "Model",
      execute: () => handleExportUfo(),
    });

    if ($activeFramework === "Custom") {
      activateCustomMode();
    }

    const restoreUi = async (): Promise<void> => {
      const snapshot = getWidgetUiSnapshot<{ showEditors?: boolean; scrollTop?: number }>(
        MODEL_LOADER_UI_KEY,
      );
      if (!snapshot) return;

      if (typeof snapshot.showEditors === "boolean") {
        showEditors = snapshot.showEditors;
      }

      await tick();
      if (rootScroller && typeof snapshot.scrollTop === "number") {
        rootScroller.scrollTop = snapshot.scrollTop;
      }
    };

    void restoreUi();
  });

  onDestroy(() => {
    if (scrollPersistRaf !== null) {
      cancelAnimationFrame(scrollPersistRaf);
      scrollPersistRaf = null;
    }
    for (const id of MODEL_CMD_IDS) unregisterCommand(id);
  });

  // ---------------------------------------------------------------------------
  // Framework switching
  // ---------------------------------------------------------------------------
  function handleFrameworkChange(event: Event): void {
    const value = (event.target as HTMLSelectElement).value as TheoreticalFramework;
    if (value === "Custom") {
      activeFramework.set("Custom");
      activateCustomMode();
      appendLog("Switched to Custom model mode");
    } else {
      activeFramework.set(value);
      deactivateCustomMode();
      appendLog(`Framework set to ${value}`);
    }
  }

  /** Enter custom editing mode - restore from localStorage or start blank. */
  function activateCustomMode(): void {
    isCustom = true;
    showEditors = true;
    const storedParticles = localStorage.getItem(LS_KEY_PARTICLES);
    const storedVertices = localStorage.getItem(LS_KEY_VERTICES);
    const storedName = localStorage.getItem(LS_KEY_MODEL_NAME);
    if (storedParticles) {
      $particlesTomlInput = storedParticles;
      $verticesTomlInput = storedVertices || "";
      $modelNameInput = storedName || "Custom Model";
      appendLog("Restored custom model from LocalStorage");
    } else {
      $particlesTomlInput = CUSTOM_TEMPLATE_PARTICLES;
      $verticesTomlInput = CUSTOM_TEMPLATE_VERTICES;
      $modelNameInput = "Custom Model";
    }
  }

  /** Exit custom mode - restore SM defaults. */
  function deactivateCustomMode(): void {
    isCustom = false;
    $particlesTomlInput = DEFAULT_PARTICLES_TOML;
    $verticesTomlInput = DEFAULT_VERTICES_TOML;
    $modelNameInput = "Standard Model";
  }

  // ---------------------------------------------------------------------------
  // LocalStorage persistence
  // ---------------------------------------------------------------------------
  function saveToLocalStorage(): void {
    localStorage.setItem(LS_KEY_PARTICLES, $particlesTomlInput);
    localStorage.setItem(LS_KEY_VERTICES, $verticesTomlInput);
    localStorage.setItem(LS_KEY_MODEL_NAME, $modelNameInput);
    savedIndicator = "Saved";
    appendLog("Custom model saved to LocalStorage");
    setTimeout(() => { savedIndicator = ""; }, 2000);
  }

  function clearLocalStorage(): void {
    localStorage.removeItem(LS_KEY_PARTICLES);
    localStorage.removeItem(LS_KEY_VERTICES);
    localStorage.removeItem(LS_KEY_MODEL_NAME);
    $particlesTomlInput = CUSTOM_TEMPLATE_PARTICLES;
    $verticesTomlInput = CUSTOM_TEMPLATE_VERTICES;
    $modelNameInput = "Custom Model";
    appendLog("Custom model data cleared from LocalStorage");
  }

  // ---------------------------------------------------------------------------
  // Model loading
  // ---------------------------------------------------------------------------
  function parseLocationHint(message: string): string {
    const lineCol = message.match(/line\s*(\d+)(?:\s*[,;:]?\s*col(?:umn)?\s*(\d+))?/i);
    if (lineCol) {
      const line = lineCol[1];
      const col = lineCol[2] ?? "?";
      return `Parser location: line ${line}, column ${col}.`;
    }
    const tuple = message.match(/\((\d+)\s*,\s*(\d+)\)/);
    if (tuple) {
      return `Parser location: line ${tuple[1]}, column ${tuple[2]}.`;
    }
    return "";
  }

  async function handleLoad(): Promise<void> {
    loading = true;
    errorMsg = "";
    errorDetails = [];
    try {
      const model = await loadModel($particlesTomlInput, $verticesTomlInput, $modelNameInput);
      theoreticalModel.set(model);
      fieldCount = model.fields.length;
      vertexCount = model.vertex_factors.length;
      unstableFieldCount = model.fields.filter((field) => Number.isFinite(field.width) && field.width > 0).length;
      const masses = model.fields
        .map((field) => field.mass)
        .filter((mass) => Number.isFinite(mass) && mass > 0)
        .sort((a, b) => a - b);
      massWindow = masses.length > 0
        ? `${masses[0].toExponential(2)} … ${masses[masses.length - 1].toExponential(2)} GeV`
        : "No massive fields";
      lastLoadedAt = new Date().toLocaleString();
      modelFingerprint = [
        model.name,
        `${model.fields.length}f`,
        `${model.vertex_factors.length}v`,
        `${model.terms.length}t`,
      ].join(" • ");
      addCitations(["pdg2024", "weinberg1967", "glashow1961"]);
      appendLog(`Model "${model.name}" loaded - ${fieldCount} fields, ${vertexCount} vertices`);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      errorMsg = msg;
      const msgLc = msg.toLowerCase();
      errorDetails = [
        parseLocationHint(msg),
        msgLc.includes("toml") ? "Verify TOML syntax (quotes, commas, and array brackets)." : "",
        msgLc.includes("particle") ? "Check that every vertex field_id exists in the particle table." : "",
        msgLc.includes("vertex") ? "Ensure each vertex has coupling_symbol, field_ids, and interaction_type." : "",
        msgLc.includes("unknown field") ? "Unknown field detected: cross-check particle IDs and case sensitivity." : "",
        "Use the Custom template scaffold to validate required fields incrementally.",
      ].filter((line) => line.length > 0);
      appendLog(`ERROR loading model: ${msg}`);
    } finally {
      loading = false;
    }
  }

  /** Toggle between showing/hiding TOML editors (always shown in custom mode). */
  let showEditors: boolean = false;

  $: interactionSummary = (() => {
    if (!$theoreticalModel) return "";
    const counts = new Map<string, number>();
    for (const field of $theoreticalModel.fields) {
      for (const interaction of field.interactions ?? []) {
        counts.set(interaction, (counts.get(interaction) ?? 0) + 1);
      }
    }
    return [...counts.entries()]
      .sort((a, b) => b[1] - a[1])
      .slice(0, 4)
      .map(([name, count]) => `${name} (${count})`)
      .join(" · ");
  })();

  $: spinSummary = (() => {
    if (!$theoreticalModel) return "";
    const spins = new Set($theoreticalModel.fields.map((f) => String(f.quantum_numbers?.spin ?? "?")));
    return [...spins].sort().join(", ");
  })();

  $: gaugeSummary = (() => {
    const gauge = $theoreticalModel?.gauge_symmetry;
    if (!gauge) return "Not specified";
    return gauge.label || `${gauge.groups.length} gauge groups`;
  })();

  $: persistModelLoaderUi({ showEditors });

  let scrollPersistRaf: number | null = null;

  function handleRootScroll(): void {
    if (!rootScroller) return;
    if (scrollPersistRaf !== null) cancelAnimationFrame(scrollPersistRaf);
    scrollPersistRaf = requestAnimationFrame(() => {
      scrollPersistRaf = null;
      persistModelLoaderUi({ scrollTop: rootScroller?.scrollTop ?? 0 });
    });
  }

  // ---------------------------------------------------------------------------
  // UFO Export
  // ---------------------------------------------------------------------------
  let ufoExportContent: string = "";
  let showUfoModal: boolean = false;
  let ufoExporting: boolean = false;

  /** Export the current model in simplified UFO format. */
  async function handleExportUfo(): Promise<void> {
    if (!$theoreticalModel) return;
    ufoExporting = true;
    try {
      const files = await exportModelUfo($theoreticalModel);
      // Combine all files into a single display string
      const sections = Object.entries(files)
        .map(([name, content]) => `# ═══ ${name} ═══\n${content}`)
        .join("\n\n");
      ufoExportContent = sections;
      showUfoModal = true;
      appendLog(`UFO export generated - ${Object.keys(files).length} files`);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      appendLog(`ERROR exporting UFO: ${msg}`);
    } finally {
      ufoExporting = false;
    }
  }

  /** Copy the UFO export content to clipboard. */
  async function copyUfoToClipboard(): Promise<void> {
    await navigator.clipboard.writeText(ufoExportContent);
    appendLog("UFO export copied to clipboard");
  }

  // ---------------------------------------------------------------------------
  // Template TOML for custom models (minimal scaffold)
  // ---------------------------------------------------------------------------
  const CUSTOM_TEMPLATE_PARTICLES = `# Custom Model - Particle Definitions
# Copy the Standard Model format or define your own particles.
#
# [[particle]]
# id             = "X"
# name           = "New Boson"
# symbol         = "X"
# mass           = 500.0
# width          = 5.0
# spin           = 2
# electric_charge = 0
# weak_isospin   = 0
# hypercharge    = 0
# baryon_number  = 0
# lepton_numbers = [0, 0, 0]
# parity         = "Even"
# charge_conjugation = "Undefined"
# color          = "Singlet"
# weak_multiplet = "Singlet"
# interactions   = ["Electromagnetic"]
`;

  const CUSTOM_TEMPLATE_VERTICES = `# Custom Model - Vertex Definitions
# Define interaction vertices referencing particles defined above.
#
# [[vertex]]
# id                = "custom_vertex"
# description       = "Custom interaction"
# field_ids         = ["X", "e-", "e-"]
# coupling_symbol   = "g_X"
# coupling_value    = 0.1
# lorentz_structure = "gamma_mu"
# interaction_type  = "Electromagnetic"
# term_kind         = "Interaction"
# expression        = "-i g_X gamma^mu"
# n_legs            = 3
#
# For dimension-6 EFT operators, use:
# term_kind         = "ContactInteraction"
# lorentz_structure = "contact_VmA"   (or contact_VpA, contact_SS, contact_4f)
# operator_dimension = 6
`;
</script>

<div
  class="model-loader"
  data-tour-id="model-loader"
  bind:this={rootScroller}
  on:scroll={handleRootScroll}
>
  <div class="header-row">
    <h3>Model Loader</h3>
    {#if $theoreticalModel}
      <span class="loaded-chip">Loaded</span>
    {/if}
  </div>

  <!-- Framework Selector -->
  <label class="field-label">
    Framework
    <select on:change={handleFrameworkChange} value={isCustom ? "Custom" : $activeFramework}>
      <option value="StandardModel">Standard Model</option>
      <option value="QED">QED</option>
      <option value="QCD">QCD</option>
      <option value="ElectroWeak">Electroweak</option>
      <option value="BSM">BSM Extension</option>
      <option value="Custom">Custom Model</option>
    </select>
  </label>

  <!-- Model name -->
  <label class="field-label">
    Model Name
    <input type="text" bind:value={$modelNameInput} placeholder="Standard Model" />
  </label>

  <!-- Custom mode: LocalStorage controls -->
  {#if isCustom}
    <div class="custom-controls">
      <button class="save-btn" on:click={saveToLocalStorage}>Save to Browser</button>
      <button class="clear-btn" on:click={clearLocalStorage}>Clear Saved</button>
      {#if savedIndicator}
        <span class="saved-indicator">{savedIndicator}</span>
      {/if}
    </div>
  {/if}

  <!-- Toggle TOML editors (always visible in custom mode) -->
  {#if !isCustom}
    <button class="toggle-btn" on:click={() => (showEditors = !showEditors)}>
      {showEditors ? "Hide TOML" : "Edit TOML Data"}
    </button>
  {/if}

  {#if showEditors || isCustom}
    <label class="field-label">
      Particles TOML
      <textarea bind:value={$particlesTomlInput} rows={isCustom ? 12 : 8} spellcheck="false"></textarea>
    </label>
    <label class="field-label">
      Vertices TOML
      <textarea bind:value={$verticesTomlInput} rows={isCustom ? 12 : 8} spellcheck="false"></textarea>
    </label>
  {/if}

  <!-- Load Button -->
  <button class="primary-btn" on:click={handleLoad} disabled={loading}>
    {#if loading}
      Loading…
    {:else}
      Load Model
    {/if}
  </button>

  <!-- Status -->
  {#if errorMsg}
    <p class="error-msg">{errorMsg}</p>
    {#if errorDetails.length > 0}
      <ul class="error-details">
        {#each errorDetails as detail}
          <li>{detail}</li>
        {/each}
      </ul>
    {/if}
  {/if}

  {#if $theoreticalModel}
    <div class="status-badge success">
      {$theoreticalModel.name} - {fieldCount} fields, {vertexCount} vertices
    </div>

    <div class="model-metadata">
      <div><span>Gauge</span> {gaugeSummary}</div>
      <div><span>Terms</span> {$theoreticalModel.terms.length} total</div>
      <div><span>Propagators</span> {$theoreticalModel.propagators.length} rules</div>
      <div><span>Spins</span> {spinSummary || "Unknown"}</div>
      <div><span>Mass window</span> {massWindow}</div>
      <div><span>Unstable fields</span> {unstableFieldCount}</div>
      <div><span>Top interactions</span> {interactionSummary || "Not available"}</div>
      <div><span>Loaded at</span> {lastLoadedAt || "-"}</div>
      <div><span>Fingerprint</span> {modelFingerprint || "-"}</div>
    </div>

    <!-- UFO Export -->
    <button class="ufo-btn" on:click={handleExportUfo} disabled={ufoExporting}>
      {#if ufoExporting}
        Exporting…
      {:else}
        Export <HoverDef term="feynman_diagram">UFO</HoverDef>
      {/if}
    </button>
  {/if}

  <!-- UFO Export Modal -->
  {#if showUfoModal}
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <div
      class="ufo-modal-overlay"
      on:click={() => (showUfoModal = false)}
      role="presentation"
    >
      <!-- svelte-ignore a11y-click-events-have-key-events -->
      <!-- svelte-ignore a11y-click-events-have-key-events -->
      <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
      <div
        class="ufo-modal"
        role="dialog"
        aria-modal="true"
        tabindex="-1"
        on:click|stopPropagation
      >
        <div class="ufo-modal-header">
          <h4>UFO Model Export</h4>
          <button
            class="close-btn"
            type="button"
            on:click={() => (showUfoModal = false)}
            aria-label="Close modal"
          >&times;</button>
        </div>
        <pre class="ufo-content">{ufoExportContent}</pre>
        <div class="ufo-modal-footer">
          <button
            class="copy-ufo-btn"
            type="button"
            on:click={copyUfoToClipboard}
          >Copy to Clipboard</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .model-loader {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    height: 100%;
    overflow-y: auto;
    min-height: 0;
  }
  h3 {
    margin: 0 0 0.25rem;
    font-size: 0.95rem;
    color: var(--fg-accent);
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.3rem;
  }
  .header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }
  .loaded-chip {
    border: 1px solid var(--hl-success);
    color: var(--hl-success);
    background: var(--bg-inset);
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 0.15rem 0.4rem;
    border-radius: 999px;
    font-weight: 600;
  }
  .field-label {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.75rem;
    color: var(--fg-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  select,
  input[type="text"] {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 0.35rem 0.5rem;
    font-size: 0.85rem;
    font-family: var(--font-mono);
  }
  textarea {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 0.4rem;
    font-size: 0.72rem;
    font-family: var(--font-mono);
    resize: vertical;
    line-height: 1.4;
  }
  .toggle-btn {
    background: none;
    border: none;
    color: var(--hl-symbol);
    font-size: 0.78rem;
    cursor: pointer;
    text-align: left;
    padding: 0.2rem 0;
    font-family: var(--font-mono);
  }
  .toggle-btn:hover {
    text-decoration: underline;
  }
  .primary-btn {
    background: var(--bg-surface);
    border: 1px solid var(--fg-accent);
    color: var(--fg-accent);
    padding: 0.45rem 0.8rem;
    font-size: 0.85rem;
    cursor: pointer;
    font-family: var(--font-mono);
    transition: border-color 0.15s;
  }
  .primary-btn:hover:not(:disabled) {
    background: var(--bg-inset);
  }
  .primary-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .error-msg {
    color: var(--hl-error);
    font-size: 0.78rem;
    margin: 0;
  }
  .error-details {
    margin: 0;
    padding-left: 1rem;
    color: var(--fg-secondary);
    font-size: 0.7rem;
    line-height: 1.4;
  }
  .status-badge {
    font-size: 0.78rem;
    padding: 0.3rem 0.5rem;
  }
  .status-badge.success {
    background: var(--bg-inset);
    color: var(--hl-success);
    border: 1px solid var(--hl-success);
  }
  .model-metadata {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.74rem;
    color: var(--fg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-inset);
    padding: 0.45rem 0.55rem;
  }
  .model-metadata span {
    color: var(--fg-accent);
    margin-right: 0.35rem;
    font-weight: 600;
  }
  .custom-controls {
    display: flex;
    gap: 0.4rem;
    align-items: center;
  }
  .save-btn {
    background: var(--bg-inset);
    border: 1px solid var(--hl-success);
    color: var(--hl-success);
    padding: 0.3rem 0.6rem;
    font-size: 0.75rem;
    cursor: pointer;
    font-family: var(--font-mono);
    transition: border-color 0.15s;
  }
  .save-btn:hover {
    background: var(--bg-surface);
  }
  .clear-btn {
    background: var(--bg-inset);
    border: 1px solid var(--hl-error);
    color: var(--hl-error);
    padding: 0.3rem 0.6rem;
    font-size: 0.75rem;
    cursor: pointer;
    font-family: var(--font-mono);
    transition: border-color 0.15s;
  }
  .clear-btn:hover {
    background: var(--bg-surface);
  }
  .saved-indicator {
    color: var(--hl-success);
    font-size: 0.75rem;
  }
  .ufo-btn {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 0.35rem 0.7rem;
    font-size: 0.8rem;
    cursor: pointer;
    font-family: var(--font-mono);
    transition: border-color 0.15s;
  }
  .ufo-btn:hover:not(:disabled) {
    border-color: var(--border-focus);
  }
  .ufo-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .ufo-modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.75);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }
  .ufo-modal {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    width: min(90vw, 640px);
    max-height: 80vh;
    display: flex;
    flex-direction: column;
  }
  .ufo-modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.6rem 0.8rem;
    border-bottom: 1px solid var(--border);
  }
  .ufo-modal-header h4 {
    margin: 0;
    font-size: 0.9rem;
    color: var(--fg-accent);
  }
  .close-btn {
    background: none;
    border: none;
    color: var(--fg-secondary);
    font-size: 1rem;
    cursor: pointer;
    font-family: var(--font-mono);
  }
  .close-btn:hover {
    color: var(--hl-error);
  }
  .ufo-content {
    flex: 1;
    overflow: auto;
    padding: 0.6rem 0.8rem;
    margin: 0;
    font-size: 0.72rem;
    color: var(--fg-primary);
    line-height: 1.45;
    white-space: pre;
  }
  .ufo-modal-footer {
    padding: 0.5rem 0.8rem;
    border-top: 1px solid var(--border);
    text-align: right;
  }
  .copy-ufo-btn {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 0.3rem 0.6rem;
    font-size: 0.75rem;
    cursor: pointer;
    font-family: var(--font-mono);
    transition: border-color 0.15s;
  }
  .copy-ufo-btn:hover {
    border-color: var(--border-focus);
  }
  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }
</style>
