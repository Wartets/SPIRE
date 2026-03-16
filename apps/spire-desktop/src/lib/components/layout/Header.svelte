<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { tooltip } from "$lib/actions/tooltip";

  const dispatch = createEventDispatcher<{
    togglePalette: void;
    toggleKernelManager: void;
  }>();
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
    <button
      class="kernel-hint"
      on:click={() => dispatch("toggleKernelManager")}
      use:tooltip={{ text: "Open Kernel / Session Manager" }}
    >
      Sessions
    </button>

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

  .kernel-hint,
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

  .kernel-hint:hover,
  .palette-hint:hover {
    border-color: var(--color-accent);
    color: var(--color-text-primary);
  }
</style>
