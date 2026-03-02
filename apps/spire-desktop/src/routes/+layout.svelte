<!--
  SPIRE Desktop — Root Layout

  Global application shell providing the navigation bar, framework
  selector, and model-status indicator. Child routes are rendered
  via the <slot /> element.
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
  :global(body) {
    margin: 0;
    padding: 0;
    background: #0e1628;
    color: #e0e0e0;
    overflow: hidden;
  }
  :global(*) {
    box-sizing: border-box;
  }
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
  }
  .navbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.4rem 0.8rem;
    background: #1a1a2e;
    color: #e0e0e0;
    flex-shrink: 0;
    border-bottom: 1px solid #0f3460;
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
    color: #7ec8e3;
  }
  .tagline {
    font-size: 0.7rem;
    opacity: 0.5;
  }
  .navbar-controls {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .nav-label {
    font-size: 0.68rem;
    color: #a0b4c8;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }
  .nav-select {
    background: #0d1b2a;
    border: 1px solid #1b2838;
    border-radius: 3px;
    color: #e0e0e0;
    padding: 0.2rem 0.4rem;
    font-size: 0.78rem;
  }
  .model-status {
    font-size: 0.75rem;
    color: #7f8c8d;
    padding: 0.2rem 0.5rem;
    border: 1px solid #1b2838;
    border-radius: 3px;
    background: #0d1b2a;
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }
  .status-dot {
    display: inline-block;
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    background: #7f8c8d;
    flex-shrink: 0;
  }
  .status-dot.active {
    background: #2ecc71;
  }
  .model-status.loaded {
    color: #2ecc71;
    border-color: #1e8449;
  }
  .main-content {
    flex: 1;
    overflow: hidden;
    padding: 0.6rem;
    min-height: 0;
  }
</style>
