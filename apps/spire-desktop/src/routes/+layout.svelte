<!--
  SPIRE Desktop — Root Layout

  Global application shell: navigation bar, framework selector,
  model-status indicator, global keybind manager, and Command Palette
  overlay.  Implements the "Typewriter" design system — monospace
  typography, sharp corners, high-contrast dark palette via CSS custom
  properties.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    activeFramework,
    isModelLoaded,
    theoreticalModel,
  } from "$lib/stores/physicsStore";
  import { backendKind, backendLabel } from "$lib/core/backend";
  import {
    commands,
    findCommandByShortcut,
    paletteOpen,
    togglePalette,
    parseShortcut,
    matchesShortcut,
  } from "$lib/core/services/CommandRegistry";
  import CommandPalette from "$lib/components/ui/CommandPalette.svelte";
  import ContextMenu from "$lib/components/ui/ContextMenu.svelte";
  import TutorialOverlay from "$lib/components/ui/TutorialOverlay.svelte";
  import { tutorialActive } from "$lib/core/services/TutorialService";
  import { showContextMenu } from "$lib/stores/contextMenuStore";
  import { initMainWindowSync } from "$lib/core/services/StoreSyncService";
  import type { TheoreticalFramework } from "$lib/types/spire";

  function onFrameworkChange(e: Event): void {
    const val = (e.target as HTMLSelectElement).value as TheoreticalFramework;
    activeFramework.set(val);
  }

  // ── Global Keybind Handler ─────────────────────────────────
  const paletteShortcut = parseShortcut("Mod+K");

  function handleGlobalKeydown(event: KeyboardEvent): void {
    // Ignore keystrokes when the user is typing in an input or textarea,
    // UNLESS it is the palette trigger (Mod+K) or Escape.
    const target = event.target as HTMLElement;
    const isInput =
      target.tagName === "INPUT" ||
      target.tagName === "TEXTAREA" ||
      target.tagName === "SELECT" ||
      target.isContentEditable;

    // Always handle palette toggle
    if (matchesShortcut(event, paletteShortcut)) {
      event.preventDefault();
      togglePalette();
      return;
    }

    // Always handle Escape to close palette
    if (event.key === "Escape" && $paletteOpen) {
      event.preventDefault();
      paletteOpen.set(false);
      return;
    }

    // Skip shortcut resolution when palette is open or user is in an input
    if ($paletteOpen || isInput) return;

    // Resolve registered command shortcuts
    const cmd = findCommandByShortcut(event, $commands);
    if (cmd) {
      event.preventDefault();
      cmd.execute();
    }
  }

  // ── Global Context Menu Interceptor ─────────────────────────────
  function handleGlobalContextMenu(event: MouseEvent): void {
    event.preventDefault();
    // Default items — widgets can override via their own contextmenu handlers
    showContextMenu(event.clientX, event.clientY, [
      { id: "ctx-palette", label: "Command Palette", icon: "⌘", shortcut: "Ctrl+K", action: () => togglePalette() },
    ]);
  }

  let stopMainSync: (() => void) | null = null;

  /** True when this layout is running inside a tear-off window. */
  const isTearOff =
    typeof window !== "undefined" &&
    window.location.pathname.startsWith("/window");

  onMount(async () => {
    window.addEventListener("keydown", handleGlobalKeydown, { capture: true });
    window.addEventListener("contextmenu", handleGlobalContextMenu);

    // Only the main window acts as the state authority.
    // Tear-off windows initialise their own sync in window/+page.svelte.
    if (!isTearOff) {
      stopMainSync = await initMainWindowSync();
    }
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleGlobalKeydown, { capture: true });
    window.removeEventListener("contextmenu", handleGlobalContextMenu);
    stopMainSync?.();
  });
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

      <!-- Backend Environment Indicator -->
      <span
        class="backend-indicator"
        class:backend-native={$backendKind === "tauri"}
        class:backend-wasm={$backendKind === "wasm"}
        class:backend-mock={$backendKind === "mock"}
        title="Execution environment: {$backendLabel}"
      >
        <span class="backend-dot"></span>
        {$backendLabel}
      </span>

      <!-- Command Palette Shortcut Hint -->
      <button
        class="palette-hint"
        on:click={() => togglePalette()}
        title="Open Command Palette (Ctrl+K)"
      >
        Ctrl+K
      </button>
    </div>
  </nav>

  <!-- Command Palette Overlay -->
  {#if $paletteOpen}
    <CommandPalette />
  {/if}

  <!-- Global Context Menu Overlay -->
  <ContextMenu />

  <!-- Tutorial Overlay -->
  {#if $tutorialActive}
    <TutorialOverlay />
  {/if}

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
    max-width: 100vw;
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
    width: 100vw;
    max-width: 100vw;
    overflow: hidden;
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
    flex-wrap: wrap;
    gap: 0.25rem;
    min-height: 0;
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
    gap: 0.6rem;
    flex-wrap: wrap;
    min-width: 0;
    overflow: hidden;
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

  /* ── Backend Indicator ────────────────────────────────────── */
  .backend-indicator {
    font-size: 0.68rem;
    padding: 0.15rem 0.45rem;
    border: 1px solid var(--border);
    background: var(--bg-inset);
    display: flex;
    align-items: center;
    gap: 0.3rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--fg-secondary);
    white-space: nowrap;
  }
  .backend-dot {
    display: inline-block;
    width: 0.45rem;
    height: 0.45rem;
    flex-shrink: 0;
    background: var(--fg-secondary);
  }
  .backend-indicator.backend-native {
    color: var(--hl-success);
    border-color: var(--hl-success);
  }
  .backend-indicator.backend-native .backend-dot {
    background: var(--hl-success);
  }
  .backend-indicator.backend-wasm {
    color: var(--hl-symbol);
    border-color: var(--hl-symbol);
  }
  .backend-indicator.backend-wasm .backend-dot {
    background: var(--hl-symbol);
  }
  .backend-indicator.backend-mock {
    color: var(--hl-value);
    border-color: var(--hl-value);
  }
  .backend-indicator.backend-mock .backend-dot {
    background: var(--hl-value);
  }

  /* ── Palette Hint ─────────────────────────────────────────── */
  .palette-hint {
    font-family: var(--font-mono);
    font-size: 0.62rem;
    padding: 0.15rem 0.4rem;
    border: 1px solid var(--border);
    background: var(--bg-inset);
    color: var(--fg-secondary);
    cursor: pointer;
    letter-spacing: 0.04em;
    white-space: nowrap;
  }
  .palette-hint:hover {
    border-color: var(--border-focus);
    color: var(--fg-primary);
  }

  /* ── Main Content ─────────────────────────────────────────── */
  .main-content {
    flex: 1;
    overflow: hidden;
    padding: 0.5rem;
    min-height: 0;
    min-width: 0;
    max-width: 100vw;
  }

  /* ── Responsive: small viewports ─────────────────────────── */
  @media (max-width: 768px) {
    .navbar { padding: 0.25rem 0.5rem; }
    .tagline { display: none; }
    .navbar-controls { gap: 0.35rem; }
    .model-status { font-size: 0.65rem; padding: 0.15rem 0.35rem; }
    .backend-indicator { font-size: 0.6rem; padding: 0.1rem 0.3rem; }
    .nav-label { font-size: 0.6rem; }
    .nav-select { font-size: 0.68rem; padding: 0.15rem 0.3rem; }
    .main-content { padding: 0.25rem; }
  }
</style>
