<!--
  SPIRE Desktop - Root Layout

  Global application shell: navigation bar, framework selector,
  model-status indicator, global keybind manager, and Command Palette
  overlay.  Implements the "Typewriter" design system - monospace
  typography, sharp corners, high-contrast dark palette via CSS custom
  properties.
-->
<script lang="ts">
  import "../app.pcss";
  import { onMount, onDestroy } from "svelte";
  import {
    activeFramework,
    isModelLoaded,
    theoreticalModel,
  } from "$lib/stores/physicsStore";
  import { backendKind, backendLabel } from "$lib/core/backend";
  import {
    paletteOpen,
    togglePalette,
  } from "$lib/core/services/CommandRegistry";
  import {
    initShortcutService,
    destroyShortcutService,
    chordIndicator,
    cheatSheetOpen,
    keybindPanelOpen,
  } from "$lib/core/services/ShortcutService";
  import CommandPalette from "$lib/components/ui/CommandPalette.svelte";
  import ContextMenu from "$lib/components/ui/ContextMenu.svelte";
  import CheatSheetOverlay from "$lib/components/ui/CheatSheetOverlay.svelte";
  import KeybindPanel from "$lib/components/settings/KeybindPanel.svelte";
  import TutorialOverlay from "$lib/components/ui/TutorialOverlay.svelte";
  import { tutorialActive } from "$lib/core/services/TutorialService";
  import { showContextMenu } from "$lib/stores/contextMenuStore";
  import { activeWorkspace } from "$lib/stores/layoutStore";
  import {
    initWorkspacePersistence,
    destroyWorkspacePersistence,
    workspacePersistenceHydrated,
  } from "$lib/stores/workspaceStore";
  import { initMainWindowSync } from "$lib/core/services/StoreSyncService";
  import Header from "$lib/components/layout/Header.svelte";
  import type { TheoreticalFramework } from "$lib/types/spire";

  function onFrameworkChange(value: TheoreticalFramework): void {
    activeFramework.set(value);
  }

  // ── Global Keybind Handler ─────────────────────────────────
  // Keyboard shortcuts are now managed by ShortcutService.ts which
  // owns a single capture-phase keydown listener.  This replaces
  // the previous inline handler with a centralised, chord-aware,
  // user-customisable engine.

  // ── Global Context Menu Interceptor ─────────────────────────────
  function handleGlobalContextMenu(event: MouseEvent): void {
    // Shift + Right-click bypasses SPIRE and opens the native browser menu
    if (event.shiftKey) return;
    event.preventDefault();
    // Default items - widgets can override via their own contextmenu handlers
    showContextMenu(event.clientX, event.clientY, [
      { type: "action", id: "ctx-palette", label: "Command Palette", shortcut: "Ctrl+K", action: () => togglePalette() },
    ]);
  }

  let stopMainSync: (() => void) | null = null;

  /** True when this layout is running inside a tear-off window. */
  const isTearOff =
    typeof window !== "undefined" &&
    window.location.pathname.startsWith("/window");

  onMount(async () => {
    initWorkspacePersistence();
    initShortcutService();
    window.addEventListener("contextmenu", handleGlobalContextMenu);

    // Only the main window acts as the state authority.
    // Tear-off windows initialise their own sync in window/+page.svelte.
    if (!isTearOff) {
      stopMainSync = await initMainWindowSync();
    }
  });

  onDestroy(() => {
    destroyShortcutService();
    destroyWorkspacePersistence();
    window.removeEventListener("contextmenu", handleGlobalContextMenu);
    stopMainSync?.();
  });
</script>

<div class="app-shell" style="--hl-symbol: {$activeWorkspace?.color ?? 'var(--color-accent)'};">
  <Header
    framework={$activeFramework}
    isModelLoaded={$isModelLoaded}
    modelName={$theoreticalModel?.name ?? null}
    backendKind={$backendKind}
    backendLabel={$backendLabel}
    on:frameworkChange={(event) => onFrameworkChange(event.detail)}
    on:togglePalette={() => togglePalette()}
  />

  <!-- Command Palette Overlay -->
  {#if $paletteOpen}
    <CommandPalette />
  {/if}

  <!-- Global Context Menu Overlay -->
  <ContextMenu />

  <!-- Shortcut Cheat Sheet -->
  {#if $cheatSheetOpen}
    <CheatSheetOverlay />
  {/if}

  <!-- Keybind Customisation Panel -->
  {#if $keybindPanelOpen}
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="keybind-panel-overlay" on:mousedown={() => keybindPanelOpen.set(false)}>
      <!-- svelte-ignore a11y-no-static-element-interactions -->
      <div class="keybind-panel-drawer" on:mousedown|stopPropagation>
        <KeybindPanel onClose={() => keybindPanelOpen.set(false)} />
      </div>
    </div>
  {/if}

  <!-- Tutorial Overlay -->
  {#if $tutorialActive}
    <TutorialOverlay />
  {/if}

  <!-- Chord Indicator -->
  {#if $chordIndicator}
    <div class="chord-indicator">{$chordIndicator}</div>
  {/if}

  <!-- Main Content Area -->
  <main class="main-content">
    {#if $workspacePersistenceHydrated}
      <slot />
    {:else}
      <div class="boot-status">Restoring workspace state…</div>
    {/if}
  </main>
</div>

<style>
  :global(:root) {
    --canvas-zoom: 1;
  }

  /* ── Font Scaling Floor ───────────────────────────────────── */
  /* When the infinite canvas is zoomed out aggressively, the CSS
     transform scale() shrinks all widget content proportionally.
     At extreme zoom-outs (<0.3×) text becomes microscopic and
     illegible.  The .zoom-floor-text utility detects this via
     --canvas-zoom and replaces text glyphs with translucent
     block indicators (wireframe placeholders) so the layout
     structure remains visible without wasting GPU cycles on
     sub-pixel glyph rasterisation.

     The floor threshold is set to 9px screen-space, assuming a
     base font size of ~12px.  zoom × 12 < 9  ⟹  zoom < 0.75. */
  :global(.zoom-floor-text) {
    color: transparent;
    background: var(--color-text-muted);
    opacity: 0.25;
    line-height: 1;
    user-select: none;
    pointer-events: none;
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

  /* ── Global Scrollbar Styling (dark & thin) ───────────────── */
  :global(*) {
    scrollbar-width: thin;                         /* Firefox */
    scrollbar-color: #444444 transparent;          /* Firefox: thumb track */
  }
  :global(*::-webkit-scrollbar) {
    width: 6px;
    height: 6px;
  }
  :global(*::-webkit-scrollbar-track) {
    background: transparent;
  }
  :global(*::-webkit-scrollbar-thumb) {
    background: color-mix(in srgb, var(--color-text-muted) 60%, transparent);
    border-radius: 3px !important;
  }
  :global(*::-webkit-scrollbar-thumb:hover) {
    background: color-mix(in srgb, var(--color-text-muted) 80%, transparent);
  }
  :global(*::-webkit-scrollbar-corner) {
    background: transparent;
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

  /* ── Main Content ─────────────────────────────────────────── */
  .main-content {
    flex: 1;
    overflow: hidden;
    padding: 0.5rem;
    min-height: 0;
    min-width: 0;
    max-width: 100vw;
  }

  .boot-status {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    color: var(--fg-secondary);
    font-size: 0.82rem;
    letter-spacing: 0.04em;
  }

  /* ── Responsive: small viewports ─────────────────────────── */
  @media (max-width: 768px) {
    .main-content { padding: 0.25rem; }
  }

  /* ── Chord Indicator ──────────────────────────────────────── */
  .chord-indicator {
    position: fixed;
    bottom: 1.2rem;
    left: 50%;
    transform: translateX(-50%);
    background: var(--color-bg-surface);
    color: var(--hl-value, #d4a017);
    border: 1px solid var(--hl-value, #d4a017);
    padding: 0.35rem 1rem;
    font-family: var(--font-mono);
    font-size: 0.72rem;
    letter-spacing: 0.04em;
    z-index: 11000;
    pointer-events: none;
    animation: chord-fadein 0.12s ease-out;
    white-space: nowrap;
  }
  @keyframes chord-fadein {
    from { opacity: 0; transform: translateX(-50%) translateY(6px); }
    to   { opacity: 1; transform: translateX(-50%) translateY(0); }
  }

  /* ── Keybind Customisation Panel ──────────────────────────── */
  .keybind-panel-overlay {
    position: fixed;
    inset: 0;
    z-index: 12500;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    justify-content: flex-end;
  }
  .keybind-panel-drawer {
    width: min(520px, 85vw);
    height: 100%;
    background: var(--color-bg-base);
    border-left: 1px solid var(--border, #333);
    box-shadow: -4px 0 24px rgba(0, 0, 0, 0.5);
    animation: drawer-slide-in 0.15s ease-out;
  }
  @keyframes drawer-slide-in {
    from { transform: translateX(100%); }
    to   { transform: translateX(0); }
  }
</style>
