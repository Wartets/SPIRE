<!--
  SPIRE - Reaction Workspace Component

  Provides controls to specify initial / final particle states, CMS energy,
  and trigger reaction construction, diagram generation, and amplitude
  derivation through the Tauri IPC bridge.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    theoreticalModel,
    activeReaction,
    reconstructedStates,
    generatedDiagrams,
    amplitudeResults,
    activeAmplitude,
    kinematics,
    appendLog,
    isModelLoaded,
    observableScripts,
    cutScripts,
  } from "$lib/stores/physicsStore";
  import {
    constructReaction,
    reconstructReaction,
    generateDiagrams,
    deriveAmplitude,
    computeKinematics,
    validateScript,
    testObservableScript,
    testCutScript,
  } from "$lib/api";
  import {
    SM_PARTICLE_IDS,
    PARTICLE_LABELS,
    DEFAULT_INITIAL_IDS,
    DEFAULT_FINAL_IDS,
    DEFAULT_CMS_ENERGY,
  } from "$lib/data/defaults";
  import {
    initialIdsInput,
    finalIdsInput,
    cmsEnergyInput,
    maxLoopOrderInput,
  } from "$lib/stores/workspaceInputsStore";

  import { registerCommand, unregisterCommand } from "$lib/core/services/CommandRegistry";
  import { addCitations } from "$lib/core/services/CitationRegistry";
  import { extractAndPushProfile } from "$lib/core/services/TelemetryService";
  import HoverDef from "$lib/components/ui/HoverDef.svelte";

  // --- Command Registration ---
  const REACTION_CMD_IDS = [
    "spire.reaction.run_pipeline",
    "spire.reaction.construct",
    "spire.reaction.reconstruct",
    "spire.reaction.generate_diagrams",
    "spire.reaction.compute_kinematics",
  ];

  onMount(() => {
    registerCommand({
      id: "spire.reaction.run_pipeline",
      title: "Run Full Pipeline",
      category: "Reaction",
      shortcut: "Mod+Enter",
      execute: () => handleRunAll(),
      pinned: true,
      icon: "P",
    });
    registerCommand({
      id: "spire.reaction.construct",
      title: "Construct Reaction",
      category: "Reaction",
      execute: () => handleConstructReaction(),
    });
    registerCommand({
      id: "spire.reaction.reconstruct",
      title: "Reconstruct Final States",
      category: "Reaction",
      execute: () => handleReconstruct(),
    });
    registerCommand({
      id: "spire.reaction.generate_diagrams",
      title: "Generate Feynman Diagrams",
      category: "Reaction",
      execute: () => handleGenerateDiagrams(),
    });
    registerCommand({
      id: "spire.reaction.compute_kinematics",
      title: "Compute Kinematics",
      category: "Reaction",
      execute: () => handleComputeKinematics(),
    });
  });

  onDestroy(() => {
    for (const id of REACTION_CMD_IDS) unregisterCommand(id);
  });

  // --- UI state ---
  let busy: boolean = false;
  let errorMsg: string = "";

  // --- Helpers for multi-select inputs ---
  let newInitialId: string = SM_PARTICLE_IDS[0];
  let newFinalId: string = SM_PARTICLE_IDS[0];

  function addInitial(): void {
    $initialIdsInput = [...$initialIdsInput, newInitialId];
  }
  function removeInitial(idx: number): void {
    $initialIdsInput = $initialIdsInput.filter((_, i) => i !== idx);
  }
  function addFinal(): void {
    $finalIdsInput = [...$finalIdsInput, newFinalId];
  }
  function removeFinal(idx: number): void {
    $finalIdsInput = $finalIdsInput.filter((_, i) => i !== idx);
  }

  /** Label for a particle ID. */
  function label(id: string): string {
    return PARTICLE_LABELS[id] ?? id;
  }

  // --- Pipeline actions ---

  async function handleConstructReaction(): Promise<void> {
    if (!$theoreticalModel) return;
    busy = true;
    errorMsg = "";
    try {
      const reaction = await constructReaction(
        $initialIdsInput,
        $finalIdsInput,
        $cmsEnergyInput,
        $theoreticalModel,
      );
      activeReaction.set(reaction);
      addCitations(["peskin1995", "pdg2024"]);
      appendLog(
        `Reaction constructed: ${$initialIdsInput.join(" ")} → ${$finalIdsInput.join(" ")} @ √s = ${$cmsEnergyInput} GeV` +
          (reaction.is_valid ? " [valid]" : ` [violated: ${reaction.violation_diagnostics.join("; ")}]`),
      );
    } catch (e: unknown) {
      errorMsg = e instanceof Error ? e.message : String(e);
      appendLog(`ERROR constructing reaction: ${errorMsg}`);
    } finally {
      busy = false;
    }
  }

  async function handleReconstruct(): Promise<void> {
    if (!$theoreticalModel) return;
    busy = true;
    errorMsg = "";
    try {
      const states = await reconstructReaction(
        $initialIdsInput,
        $cmsEnergyInput,
        $theoreticalModel,
      );
      reconstructedStates.set(states);
      appendLog(`Reconstructed ${states.length} possible final states`);
    } catch (e: unknown) {
      errorMsg = e instanceof Error ? e.message : String(e);
      appendLog(`ERROR reconstructing: ${errorMsg}`);
    } finally {
      busy = false;
    }
  }

  async function handleGenerateDiagrams(): Promise<void> {
    if (!$theoreticalModel || !$activeReaction) return;
    busy = true;
    errorMsg = "";
    try {
      const topoSet = await generateDiagrams(
        $activeReaction,
        $theoreticalModel,
        $maxLoopOrderInput,
      );
      generatedDiagrams.set(topoSet);
      extractAndPushProfile(topoSet, `Diagrams: ${topoSet.diagrams.length} topology(ies)`);
      addCitations(["hahn2001", "peskin1995"]);
      appendLog(`Generated ${topoSet.diagrams.length} Feynman diagram(s)`);
    } catch (e: unknown) {
      errorMsg = e instanceof Error ? e.message : String(e);
      appendLog(`ERROR generating diagrams: ${errorMsg}`);
    } finally {
      busy = false;
    }
  }

  async function handleDeriveAmplitudes(): Promise<void> {
    if (!$generatedDiagrams) return;
    busy = true;
    errorMsg = "";
    try {
      const results = [];
      for (const diag of $generatedDiagrams.diagrams) {
        const amp = await deriveAmplitude(diag);
        results.push(amp);
      }
      amplitudeResults.set(results);
      if (results.length > 0) {
        activeAmplitude.set(results[0].expression);
        extractAndPushProfile(results[0], `Amplitude: ${results.length} expression(s)`);
      }
      addCitations(["peskin1995", "kennedy1982"]);
      appendLog(`Derived ${results.length} amplitude expression(s)`);
    } catch (e: unknown) {
      errorMsg = e instanceof Error ? e.message : String(e);
      appendLog(`ERROR deriving amplitudes: ${errorMsg}`);
    } finally {
      busy = false;
    }
  }

  async function handleComputeKinematics(): Promise<void> {
    if (!$activeReaction) return;
    busy = true;
    errorMsg = "";
    try {
      const masses = $activeReaction.final_state.states.map(
        (s) => s.particle.field.mass,
      );
      const report = await computeKinematics(masses, $cmsEnergyInput);
      kinematics.set(report);
      addCitations(["peskin1995"]);
      appendLog(
        `Kinematics: threshold = ${report.threshold.threshold_energy.toFixed(4)} GeV, ` +
          `allowed = ${report.is_allowed}`,
      );
    } catch (e: unknown) {
      errorMsg = e instanceof Error ? e.message : String(e);
      appendLog(`ERROR computing kinematics: ${errorMsg}`);
    } finally {
      busy = false;
    }
  }

  /** Run the full pipeline: construct → diagrams → amplitudes → kinematics. */
  async function handleRunAll(): Promise<void> {
    await handleConstructReaction();
    if (!$activeReaction || !$activeReaction.is_valid) return;
    await handleGenerateDiagrams();
    if (!$generatedDiagrams) return;
    await handleDeriveAmplitudes();
    await handleComputeKinematics();
  }

  // --- Observables & Cuts ---

  let newObsName: string = "";
  let newObsScript: string = "";
  let newCutName: string = "";
  let newCutScript: string = "";
  let scriptError: string = "";

  async function handleAddObservable(): Promise<void> {
    if (!newObsScript.trim()) return;
    scriptError = "";
    try {
      await validateScript(newObsScript);
      const testResult = await testObservableScript(newObsScript);
      observableScripts.update((prev) => [
        ...prev,
        {
          name: newObsName || `Observable ${prev.length + 1}`,
          script: newObsScript,
          isValid: true,
          testResult,
        },
      ]);
      appendLog(`Added observable "${newObsName || "Observable"}": test = ${testResult.toFixed(6)}`);
      newObsName = "";
      newObsScript = "";
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      scriptError = msg;
      appendLog(`Script error: ${msg}`);
    }
  }

  async function handleAddCut(): Promise<void> {
    if (!newCutScript.trim()) return;
    scriptError = "";
    try {
      await validateScript(newCutScript);
      const testResult = await testCutScript(newCutScript);
      cutScripts.update((prev) => [
        ...prev,
        {
          name: newCutName || `Cut ${prev.length + 1}`,
          script: newCutScript,
          isValid: true,
          testResult,
        },
      ]);
      appendLog(`Added cut "${newCutName || "Cut"}": test event ${testResult ? "passed" : "failed"}`);
      newCutName = "";
      newCutScript = "";
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      scriptError = msg;
      appendLog(`Script error: ${msg}`);
    }
  }

  function removeObservable(idx: number): void {
    observableScripts.update((prev) => prev.filter((_, i) => i !== idx));
  }

  function removeCut(idx: number): void {
    cutScripts.update((prev) => prev.filter((_, i) => i !== idx));
  }
</script>

<div class="reaction-workspace" data-tour-id="reaction-input">
  <h3>Reaction Workspace</h3>

  {#if !$isModelLoaded}
    <p class="hint">Load a model first to define reactions.</p>
  {:else}
    <!-- Initial State -->
    <fieldset class="state-group">
      <legend>Initial State</legend>
      <div class="particle-tags">
        {#each $initialIdsInput as id, idx}
          <span class="tag">
            {label(id)}
            <button class="tag-remove" on:click={() => removeInitial(idx)}>&times;</button>
          </span>
        {/each}
      </div>
      <div class="add-row">
        <select bind:value={newInitialId}>
          {#each SM_PARTICLE_IDS as pid}
            <option value={pid}>{label(pid)}</option>
          {/each}
        </select>
        <button class="small-btn" on:click={addInitial}>+ Add</button>
      </div>
    </fieldset>

    <!-- Final State -->
    <fieldset class="state-group">
      <legend>Final State</legend>
      <div class="particle-tags">
        {#each $finalIdsInput as id, idx}
          <span class="tag">
            {label(id)}
            <button class="tag-remove" on:click={() => removeFinal(idx)}>&times;</button>
          </span>
        {/each}
      </div>
      <div class="add-row">
        <select bind:value={newFinalId}>
          {#each SM_PARTICLE_IDS as pid}
            <option value={pid}>{label(pid)}</option>
          {/each}
        </select>
        <button class="small-btn" on:click={addFinal}>+ Add</button>
      </div>
    </fieldset>

    <!-- Energy & Loop Order -->
    <div class="params-row">
      <label class="field-label">
        <HoverDef term="mandelstam_s">Centre-of-Mass Energy</HoverDef> (GeV)
        <input type="number" bind:value={$cmsEnergyInput} min="0" step="0.1" />
      </label>
      <label class="field-label">
        Max Loop Order
        <input type="number" bind:value={$maxLoopOrderInput} min="0" max="3" step="1" />
      </label>
    </div>

    <!-- Action Buttons -->
    <div class="actions">
      <button class="action-btn" on:click={handleRunAll} disabled={busy} data-tour-id="reaction-run">
        {busy ? "Running…" : "Run Full Pipeline"}
      </button>
      <div class="step-actions">
        <button class="step-btn" on:click={handleConstructReaction} disabled={busy}>
          Construct
        </button>
        <button class="step-btn" on:click={handleReconstruct} disabled={busy}>
          Reconstruct
        </button>
        <button class="step-btn" on:click={handleGenerateDiagrams} disabled={busy || !$activeReaction}>
          <HoverDef term="feynman_diagram">Diagrams</HoverDef>
        </button>
        <button class="step-btn" on:click={handleDeriveAmplitudes} disabled={busy || !$generatedDiagrams}>
          Amplitudes
        </button>
        <button class="step-btn" on:click={handleComputeKinematics} disabled={busy || !$activeReaction}>
          <HoverDef term="phase_space">Kinematics</HoverDef>
        </button>
      </div>
    </div>

    <!-- Error Display -->
    {#if errorMsg}
      <p class="error-msg">{errorMsg}</p>
    {/if}

    <!-- Reaction Status -->
    {#if $activeReaction}
      <div class="status-badge" class:valid={$activeReaction.is_valid} class:invalid={!$activeReaction.is_valid}>
        {#if $activeReaction.is_valid}
          Valid - order {$activeReaction.perturbative_order}, via {$activeReaction.interaction_types.join(", ")}
        {:else}
          Violation: {$activeReaction.violation_diagnostics.join("; ")}
        {/if}
      </div>
    {/if}

    <!-- Reconstructed States -->
    {#if $reconstructedStates.length > 0}
      <div class="recon-list">
        <h4>Reconstructed Final States ({$reconstructedStates.length})</h4>
        {#each $reconstructedStates as state, i}
          <div class="recon-item">
            <span class="recon-idx">#{i + 1}</span>
            <span class="recon-particles">
              {state.particles.map((p) => p.field.symbol).join(" ")}
            </span>
            <span class="recon-weight">w = {state.weight.toFixed(4)}</span>
          </div>
        {/each}
      </div>
    {/if}

    <!-- Observables & Cuts -->
    <fieldset class="state-group scripting-panel">
      <legend>Observables & Cuts</legend>

      <!-- Observable Editor -->
      <div class="script-section">
        <h4>Custom Observable</h4>
        <input class="script-name" type="text" bind:value={newObsName} placeholder="Name (e.g. Invariant mass)" />
        <textarea class="script-editor" bind:value={newObsScript} placeholder="event.momenta[2].pt()" rows="3"></textarea>
        <button class="small-btn" on:click={handleAddObservable}>+ Add Observable</button>
      </div>

      <!-- Cut Editor -->
      <div class="script-section">
        <h4>Kinematic Cut</h4>
        <input class="script-name" type="text" bind:value={newCutName} placeholder="Name (e.g. pT > 50 GeV)" />
        <textarea class="script-editor" bind:value={newCutScript} placeholder="event.momenta[2].pt() > 50.0" rows="3"></textarea>
        <button class="small-btn" on:click={handleAddCut}>+ Add Cut</button>
      </div>

      {#if scriptError}
        <p class="error-msg">{scriptError}</p>
      {/if}

      <!-- Active Observables -->
      {#if $observableScripts.length > 0}
        <div class="active-scripts">
          <h4>Active Observables ({$observableScripts.length})</h4>
          {#each $observableScripts as obs, i}
            <div class="script-item">
              <span class="script-item-name">{obs.name}</span>
              {#if obs.testResult !== undefined}
                <span class="script-item-result">= {obs.testResult.toFixed(4)}</span>
              {/if}
              <button class="tag-remove" on:click={() => removeObservable(i)}>&times;</button>
            </div>
          {/each}
        </div>
      {/if}

      <!-- Active Cuts -->
      {#if $cutScripts.length > 0}
        <div class="active-scripts">
          <h4>Active Cuts ({$cutScripts.length})</h4>
          {#each $cutScripts as cut, i}
            <div class="script-item">
              <span class="script-item-name">{cut.name}</span>
              {#if cut.testResult !== undefined}
                <span class="script-item-result"
                  class:pass={cut.testResult}
                  class:fail={!cut.testResult}>
                  {cut.testResult ? "✓ pass" : "✗ fail"}
                </span>
              {/if}
              <button class="tag-remove" on:click={() => removeCut(i)}>&times;</button>
            </div>
          {/each}
        </div>
      {/if}
    </fieldset>
  {/if}
</div>

<style>
  .reaction-workspace {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  h3 {
    margin: 0 0 0.25rem;
    font-size: 0.95rem;
    color: var(--fg-accent);
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.3rem;
  }
  h4 {
    margin: 0.3rem 0 0.2rem;
    font-size: 0.82rem;
    color: var(--fg-secondary);
  }
  .hint {
    font-size: 0.8rem;
    color: var(--fg-secondary);
    font-style: italic;
    margin: 0;
  }
  .state-group {
    border: 1px solid var(--border);
    padding: 0.4rem 0.5rem;
    margin: 0;
  }
  .state-group legend {
    font-size: 0.72rem;
    text-transform: uppercase;
    color: var(--fg-secondary);
    letter-spacing: 0.06em;
  }
  .particle-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-bottom: 0.3rem;
  }
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 0.2rem;
    background: var(--bg-inset);
    border: 1px solid var(--border);
    padding: 0.15rem 0.4rem;
    font-size: 0.75rem;
    color: var(--fg-primary);
  }
  .tag-remove {
    background: none;
    border: none;
    color: var(--hl-error);
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0;
    line-height: 1;
  }
  .add-row {
    display: flex;
    gap: 0.3rem;
    align-items: center;
  }
  .add-row select {
    flex: 1;
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 0.25rem;
    font-size: 0.78rem;
    font-family: var(--font-mono);
  }
  .small-btn {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
    cursor: pointer;
    font-family: var(--font-mono);
  }
  .small-btn:hover {
    border-color: var(--border-focus);
  }
  .params-row {
    display: flex;
    gap: 0.5rem;
  }
  .field-label {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    font-size: 0.72rem;
    color: var(--fg-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    flex: 1;
  }
  .field-label input {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 0.3rem 0.5rem;
    font-size: 0.85rem;
    font-family: var(--font-mono);
  }
  .actions {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .action-btn {
    background: var(--bg-surface);
    border: 1px solid var(--fg-accent);
    color: var(--fg-accent);
    padding: 0.5rem;
    font-size: 0.88rem;
    cursor: pointer;
    font-weight: 600;
    font-family: var(--font-mono);
    transition: border-color 0.15s;
  }
  .action-btn:hover:not(:disabled) {
    background: var(--bg-inset);
  }
  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .step-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }
  .step-btn {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 0.25rem 0.45rem;
    font-size: 0.72rem;
    cursor: pointer;
    font-family: var(--font-mono);
  }
  .step-btn:hover:not(:disabled) {
    border-color: var(--border-focus);
  }
  .step-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .error-msg {
    color: var(--hl-error);
    font-size: 0.78rem;
    margin: 0;
  }
  .status-badge {
    font-size: 0.78rem;
    padding: 0.3rem 0.5rem;
  }
  .status-badge.valid {
    background: var(--bg-inset);
    color: var(--hl-success);
    border: 1px solid var(--hl-success);
  }
  .status-badge.invalid {
    background: var(--bg-inset);
    color: var(--hl-error);
    border: 1px solid var(--hl-error);
  }
  .recon-list {
    max-height: 10rem;
    overflow-y: auto;
  }
  .recon-item {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.2rem 0;
    font-size: 0.78rem;
    border-bottom: 1px solid var(--border);
  }
  .recon-idx {
    color: var(--hl-symbol);
    font-weight: 600;
    min-width: 1.5rem;
  }
  .recon-particles {
    flex: 1;
    color: var(--fg-primary);
  }
  .recon-weight {
    color: var(--fg-secondary);
    font-size: 0.72rem;
  }

  /* --- Scripting Panel --- */
  .scripting-panel {
    margin-top: 0.5rem;
  }
  .script-section {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    margin-bottom: 0.5rem;
  }
  .script-name {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 0.25rem 0.4rem;
    font-size: 0.78rem;
    font-family: var(--font-mono);
  }
  .script-editor {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 0.3rem 0.4rem;
    font-size: 0.78rem;
    font-family: var(--font-mono);
    resize: vertical;
    min-height: 2.5rem;
  }
  .script-editor:focus, .script-name:focus {
    border-color: var(--border-focus);
    outline: none;
  }
  .active-scripts {
    margin-top: 0.3rem;
  }
  .script-item {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.15rem 0;
    font-size: 0.75rem;
    border-bottom: 1px solid var(--border);
  }
  .script-item-name {
    flex: 1;
    color: var(--fg-primary);
  }
  .script-item-result {
    color: var(--fg-secondary);
    font-family: var(--font-mono);
    font-size: 0.72rem;
  }
  .script-item-result.pass {
    color: var(--hl-success);
  }
  .script-item-result.fail {
    color: var(--hl-error);
  }
</style>
