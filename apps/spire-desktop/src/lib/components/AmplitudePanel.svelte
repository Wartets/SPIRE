<!--
  SPIRE — Amplitude Panel Component

  Displays symbolic amplitude expressions computed from Feynman diagrams.
  Shows the full list of computed amplitudes with their coupling constants
  and momentum labels. The currently selected amplitude expression is
  displayed in a prominent monospace block.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    amplitudeResults,
    activeAmplitude,
    generatedDiagrams,
  } from "$lib/stores/physicsStore";
  import { exportAmplitudeLatex, deriveAmplitudeSteps } from "$lib/api";
  import { registerCommand, unregisterCommand } from "$lib/core/services/CommandRegistry";
  import type { AmplitudeResult, DerivationStep } from "$lib/types/spire";

  /** Select an amplitude to view in detail. */
  function selectAmplitude(amp: AmplitudeResult): void {
    activeAmplitude.set(amp.expression);
    selectedDiagramId = amp.diagram_id;
    derivationSteps = [];
    derivationOpen = false;
  }

  /** Copy the LaTeX representation of the selected amplitude to the clipboard. */
  async function copyLatex(): Promise<void> {
    if (!$generatedDiagrams || selectedDiagramId === null) return;
    const diagram = $generatedDiagrams.diagrams.find(
      (d) => d.id === selectedDiagramId
    );
    if (!diagram) return;
    latexStatus = "…";
    try {
      const latex = await exportAmplitudeLatex(diagram);
      await navigator.clipboard.writeText(latex);
      latexStatus = "Copied";
      setTimeout(() => { latexStatus = ""; }, 2000);
    } catch (e: unknown) {
      latexStatus = "Error";
      setTimeout(() => { latexStatus = ""; }, 3000);
    }
  }

  /** Fetch the step-by-step CAS derivation for the currently selected diagram. */
  async function showDerivation(): Promise<void> {
    if (!$generatedDiagrams || selectedDiagramId === null) return;
    const diagram = $generatedDiagrams.diagrams.find(
      (d) => d.id === selectedDiagramId
    );
    if (!diagram) return;
    derivationLoading = true;
    derivationError = "";
    try {
      derivationSteps = await deriveAmplitudeSteps(diagram);
      derivationOpen = true;
    } catch (e: unknown) {
      derivationError = e instanceof Error ? e.message : String(e);
    } finally {
      derivationLoading = false;
    }
  }

  let selectedDiagramId: number | null = null;
  let latexStatus: string = "";
  let derivationSteps: DerivationStep[] = [];
  let derivationOpen: boolean = false;
  let derivationLoading: boolean = false;
  let derivationError: string = "";

  $: results = $amplitudeResults;
  $: selected = $activeAmplitude;

  // ---------------------------------------------------------------------------
  // Command Registration
  // ---------------------------------------------------------------------------
  const AMP_CMD_IDS = [
    "spire.amplitude.copy_latex",
    "spire.amplitude.show_derivation",
  ];

  onMount(() => {
    registerCommand({
      id: "spire.amplitude.copy_latex",
      title: "Copy Amplitude LaTeX",
      category: "Amplitude",
      execute: () => copyLatex(),
    });
    registerCommand({
      id: "spire.amplitude.show_derivation",
      title: "Show Derivation Steps",
      category: "Amplitude",
      execute: () => showDerivation(),
    });
  });

  onDestroy(() => {
    for (const id of AMP_CMD_IDS) unregisterCommand(id);
  });
</script>

<div class="amplitude-panel">
  <h3>Invariant Amplitudes</h3>

  {#if results.length === 0}
    <p class="hint">No amplitudes computed yet. Generate diagrams and derive amplitudes first.</p>
  {:else}
    <!-- Active Amplitude Display -->
    {#if selected}
      <div class="active-expression">
        <pre>{selected}</pre>
        <div class="latex-row">
          <button class="latex-btn" on:click={copyLatex} disabled={selectedDiagramId === null}>
            Copy LaTeX
          </button>
          <button
            class="latex-btn derivation-btn"
            on:click={showDerivation}
            disabled={selectedDiagramId === null || derivationLoading}
          >
            {derivationLoading ? "Deriving…" : "Show Derivation"}
          </button>
          {#if latexStatus}
            <span class="latex-status">{latexStatus}</span>
          {/if}
        </div>

        <!-- CAS Derivation Steps -->
        {#if derivationError}
          <div class="derivation-error">{derivationError}</div>
        {/if}
        {#if derivationOpen && derivationSteps.length > 0}
          <div class="derivation-panel">
            <div class="derivation-header">
              <span class="derivation-title">Step-by-Step Derivation</span>
              <button class="derivation-close" on:click={() => { derivationOpen = false; }}>&times;</button>
            </div>
            <div class="derivation-steps">
              {#each derivationSteps as step, idx}
                <div class="derivation-step">
                  <div class="step-badge">{idx + 1}</div>
                  <div class="step-content">
                    <div class="step-label">{step.label}</div>
                    <div class="step-desc">{step.description}</div>
                    <pre class="step-latex">{step.latex}</pre>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Amplitude List -->
    <div class="amplitude-list">
      {#each results as amp, idx}
        <button
          class="amp-card"
          class:active={amp.expression === selected}
          on:click={() => selectAmplitude(amp)}
        >
          <div class="amp-header">
            <span class="amp-id">Diagram #{amp.diagram_id}</span>
            <span class="amp-couplings">{amp.couplings.join(", ")}</span>
          </div>
          <div class="amp-expr">{amp.expression}</div>
          <div class="amp-momenta">
            Momenta: {amp.momenta_labels.join(", ")}
          </div>
        </button>
      {/each}
    </div>

    <!-- Summary -->
    <div class="summary">
      {results.length} amplitude{results.length !== 1 ? "s" : ""} computed
      {#if $generatedDiagrams}
        from {$generatedDiagrams.diagrams.length} diagram{$generatedDiagrams.diagrams.length !== 1 ? "s" : ""}
      {/if}
    </div>
  {/if}
</div>

<style>
  .amplitude-panel {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  h3 {
    margin: 0 0 0.25rem;
    font-size: 0.95rem;
    color: var(--fg-accent);
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.3rem;
  }
  .hint {
    font-size: 0.8rem;
    color: var(--fg-secondary);
    font-style: italic;
    margin: 0;
  }
  .active-expression {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    padding: 0.6rem;
    overflow-x: auto;
  }
  .active-expression pre {
    margin: 0;
    font-size: 0.82rem;
    color: var(--hl-value);
    white-space: pre-wrap;
    word-break: break-all;
    line-height: 1.5;
  }
  .latex-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-top: 0.35rem;
  }
  .latex-btn {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 0.25rem 0.6rem;
    font-size: 0.72rem;
    cursor: pointer;
    font-family: var(--font-mono);
    transition: border-color 0.15s;
  }
  .latex-btn:hover:not(:disabled) {
    border-color: var(--border-focus);
  }
  .latex-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .latex-status {
    font-size: 0.72rem;
    color: var(--hl-success);
  }
  .amplitude-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    max-height: 15rem;
    overflow-y: auto;
  }
  .amp-card {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    padding: 0.4rem 0.5rem;
    text-align: left;
    cursor: pointer;
    color: var(--fg-primary);
    transition: border-color 0.15s;
    display: block;
    width: 100%;
    font-family: var(--font-mono);
    font-size: inherit;
  }
  .amp-card:hover {
    border-color: var(--border-focus);
  }
  .amp-card.active {
    border-color: var(--hl-value);
    background: var(--bg-surface);
  }
  .amp-header {
    display: flex;
    justify-content: space-between;
    font-size: 0.72rem;
    margin-bottom: 0.2rem;
  }
  .amp-id {
    color: var(--hl-symbol);
    font-weight: 600;
  }
  .amp-couplings {
    color: var(--fg-secondary);
    font-size: 0.7rem;
  }
  .amp-expr {
    font-size: 0.75rem;
    color: var(--fg-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .amp-momenta {
    font-size: 0.65rem;
    color: var(--fg-secondary);
    margin-top: 0.15rem;
  }
  .summary {
    font-size: 0.72rem;
    color: var(--fg-secondary);
    text-align: right;
  }

  /* ---- CAS Derivation Steps ---- */
  .derivation-btn {
    border-color: var(--hl-success);
    color: var(--hl-success);
  }
  .derivation-btn:hover:not(:disabled) {
    background: var(--bg-inset);
  }
  .derivation-error {
    font-size: 0.75rem;
    color: var(--hl-error);
    margin-top: 0.3rem;
  }
  .derivation-panel {
    margin-top: 0.4rem;
    border: 1px solid var(--border);
    background: var(--bg-inset);
    max-height: 20rem;
    overflow-y: auto;
  }
  .derivation-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.35rem 0.5rem;
    border-bottom: 1px solid var(--border);
    background: var(--bg-surface);
  }
  .derivation-title {
    font-size: 0.78rem;
    color: var(--hl-success);
    font-weight: 600;
  }
  .derivation-close {
    background: none;
    border: none;
    color: var(--fg-secondary);
    cursor: pointer;
    font-size: 0.8rem;
    padding: 0 0.25rem;
  }
  .derivation-close:hover {
    color: var(--hl-error);
  }
  .derivation-steps {
    display: flex;
    flex-direction: column;
    gap: 0;
  }
  .derivation-step {
    display: flex;
    gap: 0.5rem;
    padding: 0.45rem 0.5rem;
    border-bottom: 1px solid var(--border);
  }
  .derivation-step:last-child {
    border-bottom: none;
  }
  .step-badge {
    flex-shrink: 0;
    width: 1.4rem;
    height: 1.4rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--hl-symbol);
    font-size: 0.68rem;
    font-weight: 700;
  }
  .step-content {
    flex: 1;
    min-width: 0;
  }
  .step-label {
    font-size: 0.76rem;
    color: var(--hl-value);
    font-weight: 600;
    margin-bottom: 0.1rem;
  }
  .step-desc {
    font-size: 0.68rem;
    color: var(--fg-secondary);
    margin-bottom: 0.25rem;
  }
  .step-latex {
    margin: 0;
    font-size: 0.72rem;
    color: var(--fg-primary);
    white-space: pre-wrap;
    word-break: break-all;
    line-height: 1.4;
    background: var(--bg-primary);
    padding: 0.3rem 0.4rem;
    border: 1px solid var(--border);
  }
</style>
