<!--
  SPIRE — Amplitude Panel Component

  Displays symbolic amplitude expressions computed from Feynman diagrams.
  Shows the full list of computed amplitudes with their coupling constants
  and momentum labels. The currently selected amplitude expression is
  displayed in a prominent monospace block.
-->
<script lang="ts">
  import {
    amplitudeResults,
    activeAmplitude,
    generatedDiagrams,
  } from "$lib/stores/physicsStore";
  import { exportAmplitudeLatex } from "$lib/api";
  import type { AmplitudeResult } from "$lib/types/spire";

  /** Select an amplitude to view in detail. */
  function selectAmplitude(amp: AmplitudeResult): void {
    activeAmplitude.set(amp.expression);
    selectedDiagramId = amp.diagram_id;
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
      latexStatus = "✓ Copied!";
      setTimeout(() => { latexStatus = ""; }, 2000);
    } catch (e: unknown) {
      latexStatus = "✗ Error";
      setTimeout(() => { latexStatus = ""; }, 3000);
    }
  }

  let selectedDiagramId: number | null = null;
  let latexStatus: string = "";

  $: results = $amplitudeResults;
  $: selected = $activeAmplitude;
</script>

<div class="amplitude-panel">
  <h3>𝓜 Amplitudes</h3>

  {#if results.length === 0}
    <p class="hint">No amplitudes computed yet. Generate diagrams and derive amplitudes first.</p>
  {:else}
    <!-- Active Amplitude Display -->
    {#if selected}
      <div class="active-expression">
        <pre>{selected}</pre>
        <div class="latex-row">
          <button class="latex-btn" on:click={copyLatex} disabled={selectedDiagramId === null}>
            📋 Copy LaTeX
          </button>
          {#if latexStatus}
            <span class="latex-status">{latexStatus}</span>
          {/if}
        </div>
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
    color: #7ec8e3;
    border-bottom: 1px solid #0f3460;
    padding-bottom: 0.3rem;
  }
  .hint {
    font-size: 0.8rem;
    opacity: 0.5;
    font-style: italic;
    margin: 0;
  }
  .active-expression {
    background: #0a0f1a;
    border: 1px solid #1a5276;
    border-radius: 4px;
    padding: 0.6rem;
    overflow-x: auto;
  }
  .active-expression pre {
    margin: 0;
    font-family: "Fira Code", "Cascadia Code", monospace;
    font-size: 0.82rem;
    color: #f39c12;
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
    background: #1a3a5c;
    border: 1px solid #2980b9;
    border-radius: 4px;
    color: #e0e0e0;
    padding: 0.25rem 0.6rem;
    font-size: 0.72rem;
    cursor: pointer;
    transition: background 0.15s;
  }
  .latex-btn:hover:not(:disabled) {
    background: #2471a3;
  }
  .latex-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .latex-status {
    font-size: 0.72rem;
    color: #2ecc71;
    animation: fadeIn 0.2s ease-in;
  }
  .amplitude-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    max-height: 15rem;
    overflow-y: auto;
  }
  .amp-card {
    background: #0d1b2a;
    border: 1px solid #1b2838;
    border-radius: 4px;
    padding: 0.4rem 0.5rem;
    text-align: left;
    cursor: pointer;
    color: #e0e0e0;
    transition: border-color 0.15s;
    display: block;
    width: 100%;
    font-family: inherit;
    font-size: inherit;
  }
  .amp-card:hover {
    border-color: #2980b9;
  }
  .amp-card.active {
    border-color: #f39c12;
    background: #162447;
  }
  .amp-header {
    display: flex;
    justify-content: space-between;
    font-size: 0.72rem;
    margin-bottom: 0.2rem;
  }
  .amp-id {
    color: #5dade2;
    font-weight: 600;
  }
  .amp-couplings {
    color: #a0b4c8;
    font-family: "Fira Code", monospace;
    font-size: 0.7rem;
  }
  .amp-expr {
    font-family: "Fira Code", "Cascadia Code", monospace;
    font-size: 0.75rem;
    color: #c8d6e5;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .amp-momenta {
    font-size: 0.65rem;
    color: #7f8c8d;
    margin-top: 0.15rem;
  }
  .summary {
    font-size: 0.72rem;
    color: #7f8c8d;
    text-align: right;
  }
</style>
