<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import type { TheoreticalFramework } from "$lib/types/spire";

  export let framework: TheoreticalFramework;
  export let isModelLoaded = false;
  export let modelName: string | null = null;
  export let backendKind: "tauri" | "wasm" | "mock" = "mock";
  export let backendLabel = "Simulation";

  const dispatch = createEventDispatcher<{
    frameworkChange: TheoreticalFramework;
    togglePalette: void;
  }>();

  function handleFrameworkChange(event: Event): void {
    const value = (event.currentTarget as HTMLSelectElement).value as TheoreticalFramework;
    dispatch("frameworkChange", value);
  }
</script>

<nav class="navbar">
  <div class="navbar-brand">
    <a
      class="logo-link"
      href="https://github.com/Wartets/SPIRE"
      target="_blank"
      rel="noopener noreferrer"
      use:tooltip={{ text: "Open SPIRE repository" }}
    >
      <span class="logo">SPIRE</span>
    </a>
    <span class="tagline">Structured Particle Interaction &amp; Reaction Engine</span>
  </div>

  <div class="navbar-controls">
    <label class="nav-label">
      Framework
      <select class="nav-select" value={framework} on:change={handleFrameworkChange}>
        <option value="StandardModel">Standard Model</option>
        <option value="QED">QED</option>
        <option value="QCD">QCD</option>
        <option value="ElectroWeak">Electroweak</option>
        <option value="BSM">BSM</option>
      </select>
    </label>

    <span class="model-status" class:loaded={isModelLoaded}>
      {#if isModelLoaded}
        <span class="status-dot active"></span> {modelName ?? "Model loaded"}
      {:else}
        <span class="status-dot"></span> No model loaded
      {/if}
    </span>

    <span
      class="backend-indicator"
      class:backend-native={backendKind === "tauri"}
      class:backend-wasm={backendKind === "wasm"}
      class:backend-mock={backendKind === "mock"}
      use:tooltip={{ text: `Execution environment: ${backendLabel}` }}
    >
      <span class="backend-dot"></span>
      {backendLabel}
    </span>

    <button
      class="palette-hint"
      on:click={() => dispatch("togglePalette")}
      use:tooltip={{ text: "Open Command Palette (Ctrl+K)" }}
    >
      Ctrl+K
    </button>
  </div>
</nav>

<style>
  .navbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--spacing-sm) var(--spacing-lg);
    background: var(--color-bg-surface);
    color: var(--color-text-primary);
    border-bottom: 1px solid var(--color-border);
    flex-wrap: wrap;
    gap: var(--spacing-xs);
    min-height: 0;
  }

  .navbar-brand {
    display: flex;
    align-items: baseline;
    gap: var(--spacing-md);
  }

  .logo-link {
    text-decoration: none;
    color: inherit;
  }

  .logo {
    font-size: 1.3rem;
    font-weight: 700;
    letter-spacing: 0.12em;
    color: var(--color-text-primary);
    transition: color var(--transition-fast);
  }

  .logo-link:hover .logo {
    color: var(--color-accent);
  }

  .tagline {
    font-size: var(--text-xs);
    color: var(--color-text-muted);
  }

  .navbar-controls {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    flex-wrap: wrap;
    min-width: 0;
    overflow: hidden;
  }

  .nav-label {
    font-size: var(--text-xs);
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
  }

  .nav-select {
    background: var(--color-bg-inset);
    border: 1px solid var(--color-border);
    color: var(--color-text-primary);
    padding: var(--spacing-xs) var(--spacing-sm);
    font-size: var(--text-sm);
    font-family: var(--font-mono);
  }

  .model-status {
    font-size: var(--text-sm);
    color: var(--color-text-muted);
    padding: var(--spacing-xs) var(--spacing-sm);
    border: 1px solid var(--color-border);
    background: var(--color-bg-inset);
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
  }

  .status-dot {
    display: inline-block;
    width: 0.5rem;
    height: 0.5rem;
    background: var(--color-text-muted);
    flex-shrink: 0;
  }

  .status-dot.active {
    background: var(--color-success);
  }

  .model-status.loaded {
    color: var(--color-success);
    border-color: var(--color-success);
  }

  .backend-indicator {
    font-size: var(--text-xs);
    padding: 0.15rem 0.45rem;
    border: 1px solid var(--color-border);
    background: var(--color-bg-inset);
    display: flex;
    align-items: center;
    gap: 0.3rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-text-muted);
    white-space: nowrap;
  }

  .backend-dot {
    display: inline-block;
    width: 0.45rem;
    height: 0.45rem;
    flex-shrink: 0;
    background: var(--color-text-muted);
  }

  .backend-indicator.backend-native {
    color: var(--color-success);
    border-color: var(--color-success);
  }

  .backend-indicator.backend-native .backend-dot {
    background: var(--color-success);
  }

  .backend-indicator.backend-wasm {
    color: var(--color-accent);
    border-color: var(--color-accent);
  }

  .backend-indicator.backend-wasm .backend-dot {
    background: var(--color-accent);
  }

  .backend-indicator.backend-mock {
    color: var(--hl-value);
    border-color: var(--hl-value);
  }

  .backend-indicator.backend-mock .backend-dot {
    background: var(--hl-value);
  }

  .palette-hint {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    padding: 0.15rem 0.4rem;
    border: 1px solid var(--color-border);
    background: var(--color-bg-inset);
    color: var(--color-text-muted);
    letter-spacing: 0.04em;
    white-space: nowrap;
  }

  .palette-hint:hover {
    border-color: var(--color-accent);
    color: var(--color-text-primary);
  }
</style>
