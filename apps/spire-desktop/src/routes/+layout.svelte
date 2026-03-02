<!--
  SPIRE Desktop — Root Layout

  Global application shell: navigation bar, framework selector,
  model-status indicator.  Implements the "Typewriter" design
  system — monospace typography, sharp corners, high-contrast
  dark palette via CSS custom properties.
-->
<script lang="ts">
  import {
    activeFramework,
    isModelLoaded,
    theoreticalModel,
  } from "$lib/stores/physicsStore";
  import type { TheoreticalFramework } from "$lib/types/spire";

  function onFrameworkChange(e: Event): void {
    const val = (e.target as HTMLSelectElement).value as TheoreticalFramework;
    activeFramework.set(val);
  }
</script>

<div class="app-shell">
  <!-- Navigation Bar -->
  <nav class="navbar">
    <div class="navbar-brand">
      <span class="logo">SPIRE</span>
      <span class="tagline">Structured Particle Interaction &amp; Reaction Engine</span>
    </div>
    <div class="navbar-controls">
      <!-- Framework Selector -->
      <label class="nav-label">
        Framework
        <select class="nav-select" value={$activeFramework} on:change={onFrameworkChange}>
          <option value="StandardModel">Standard Model</option>
          <option value="QED">QED</option>
          <option value="QCD">QCD</option>
          <option value="ElectroWeak">Electroweak</option>
          <option value="BSM">BSM</option>
        </select>
      </label>

      <!-- Model Status Indicator -->
      <span class="model-status" class:loaded={$isModelLoaded}>
        {#if $isModelLoaded && $theoreticalModel}
          <span class="status-dot active"></span> {$theoreticalModel.name}
        {:else}
          <span class="status-dot"></span> No model loaded
        {/if}
      </span>
    </div>
  </nav>

  <!-- Main Content Area -->
  <main class="main-content">
    <slot />
  </main>
</div>

<style>
  /* ── Typewriter Design Tokens ─────────────────────────────── */
  :global(:root) {
    --font-mono: "Fira Code", "Cascadia Code", "Courier New", monospace;
    --bg-primary:   #121212;
    --bg-surface:   #1a1a1a;
    --bg-inset:     #0e0e0e;
    --fg-primary:   #e8e8e8;
    --fg-secondary: #888888;
    --fg-accent:    #ffffff;
    --border:       #333333;
    --border-focus: #666666;
    --hl-value:     #d4a017;
    --hl-symbol:    #5eb8ff;
    --hl-error:     #e74c3c;
    --hl-success:   #2ecc71;
  }

  /* ── Global Reset ─────────────────────────────────────────── */
  :global(body) {
    margin: 0;
    padding: 0;
    background: var(--bg-primary);
    color: var(--fg-primary);
    overflow: hidden;
    font-family: var(--font-mono);
  }
  :global(*) {
    box-sizing: border-box;
    border-radius: 0 !important;
  }

  /* ── App Shell ────────────────────────────────────────────── */
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    font-family: var(--font-mono);
  }

  /* ── Navbar ───────────────────────────────────────────────── */
  .navbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.4rem 0.8rem;
    background: var(--bg-surface);
    color: var(--fg-primary);
    flex-shrink: 0;
    border-bottom: 1px solid var(--border);
  }
  .navbar-brand {
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
  }
  .logo {
    font-size: 1.3rem;
    font-weight: 700;
    letter-spacing: 0.12em;
    color: var(--fg-accent);
  }
  .tagline {
    font-size: 0.7rem;
    color: var(--fg-secondary);
  }

  /* ── Navbar Controls ──────────────────────────────────────── */
  .navbar-controls {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .nav-label {
    font-size: 0.68rem;
    color: var(--fg-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }
  .nav-select {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 0.2rem 0.4rem;
    font-size: 0.78rem;
    font-family: var(--font-mono);
  }
  .model-status {
    font-size: 0.75rem;
    color: var(--fg-secondary);
    padding: 0.2rem 0.5rem;
    border: 1px solid var(--border);
    background: var(--bg-inset);
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }
  .status-dot {
    display: inline-block;
    width: 0.5rem;
    height: 0.5rem;
    background: var(--fg-secondary);
    flex-shrink: 0;
  }
  .status-dot.active {
    background: var(--hl-success);
  }
  .model-status.loaded {
    color: var(--hl-success);
    border-color: var(--hl-success);
  }

  /* ── Main Content ─────────────────────────────────────────── */
  .main-content {
    flex: 1;
    overflow: hidden;
    padding: 0.5rem;
    min-height: 0;
  }
</style>
