<!--
  SPIRE — Reaction Workspace Component

  Provides controls to specify initial / final particle states, CMS energy,
  and trigger reaction construction, diagram generation, and amplitude
  derivation through the Tauri IPC bridge.
-->
<script lang="ts">
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
  } from "$lib/stores/physicsStore";
  import {
    constructReaction,
    reconstructReaction,
    generateDiagrams,
    deriveAmplitude,
    computeKinematics,
  } from "$lib/api";
  import {
    SM_PARTICLE_IDS,
    PARTICLE_LABELS,
    DEFAULT_INITIAL_IDS,
    DEFAULT_FINAL_IDS,
    DEFAULT_CMS_ENERGY,
  } from "$lib/data/defaults";

  // --- Input state ---
  let initialIds: string[] = [...DEFAULT_INITIAL_IDS];
  let finalIds: string[] = [...DEFAULT_FINAL_IDS];
  let cmsEnergy: number = DEFAULT_CMS_ENERGY;
  let maxLoopOrder: number = 0;

  // --- UI state ---
  let busy: boolean = false;
  let errorMsg: string = "";

  // --- Helpers for multi-select inputs ---
  let newInitialId: string = SM_PARTICLE_IDS[0];
  let newFinalId: string = SM_PARTICLE_IDS[0];

  function addInitial(): void {
    initialIds = [...initialIds, newInitialId];
  }
  function removeInitial(idx: number): void {
    initialIds = initialIds.filter((_, i) => i !== idx);
  }
  function addFinal(): void {
    finalIds = [...finalIds, newFinalId];
  }
  function removeFinal(idx: number): void {
    finalIds = finalIds.filter((_, i) => i !== idx);
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
        initialIds,
        finalIds,
        cmsEnergy,
        $theoreticalModel,
      );
      activeReaction.set(reaction);
      appendLog(
        `Reaction constructed: ${initialIds.join(" ")} → ${finalIds.join(" ")} @ √s = ${cmsEnergy} GeV` +
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
        initialIds,
        cmsEnergy,
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
        maxLoopOrder,
      );
      generatedDiagrams.set(topoSet);
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
      }
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
      const report = await computeKinematics(masses, cmsEnergy);
      kinematics.set(report);
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
    if ($activeReaction && $activeReaction.is_valid) {
      await handleGenerateDiagrams();
      await handleDeriveAmplitudes();
      await handleComputeKinematics();
    }
  }
</script>

<div class="reaction-workspace">
  <h3>Reaction Workspace</h3>

  {#if !$isModelLoaded}
    <p class="hint">Load a model first to define reactions.</p>
  {:else}
    <!-- Initial State -->
    <fieldset class="state-group">
      <legend>Initial State</legend>
      <div class="particle-tags">
        {#each initialIds as id, idx}
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
        {#each finalIds as id, idx}
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
        Centre-of-Mass Energy (GeV)
        <input type="number" bind:value={cmsEnergy} min="0" step="0.1" />
      </label>
      <label class="field-label">
        Max Loop Order
        <input type="number" bind:value={maxLoopOrder} min="0" max="3" step="1" />
      </label>
    </div>

    <!-- Action Buttons -->
    <div class="actions">
      <button class="action-btn" on:click={handleRunAll} disabled={busy}>
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
          Diagrams
        </button>
        <button class="step-btn" on:click={handleDeriveAmplitudes} disabled={busy || !$generatedDiagrams}>
          Amplitudes
        </button>
        <button class="step-btn" on:click={handleComputeKinematics} disabled={busy || !$activeReaction}>
          Kinematics
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
          Valid — order {$activeReaction.perturbative_order}, via {$activeReaction.interaction_types.join(", ")}
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
</style>
