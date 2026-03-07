<!--
  SPIRE Desktop - Adaptive Workbench Page

  Dual-mode workspace: recursive docking panels (default) or infinite
  whiteboard canvas.  The layout tree is driven by layoutStore.
  A compact Toolbox bar lets the user spawn new widgets and toggle
  between view modes.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    WIDGET_DEFINITIONS,
  } from "$lib/stores/notebookStore";
  import type { WidgetType } from "$lib/stores/notebookStore";
  import {
    layoutRoot,
    viewMode,
    totalWidgetCount,
    addWidgetToLayout,
    addCanvasItem,
    resetDockingLayout,
    clearCanvas,
    toggleViewMode,
    canvasViewport,
    canvasItems,
    removeCanvasItem,
    workspaces,
    activeWorkspaceId,
    addWorkspace,
    switchWorkspace,
    removeWorkspace,
  } from "$lib/stores/layoutStore";
  import { workspaceInputsSnapshot } from "$lib/stores/workspaceInputsStore";
  import { activeFramework } from "$lib/stores/physicsStore";
  import {
    autoSave,
    debounce,
    downloadWorkspace,
    resetWorkspace,
  } from "$lib/services/workspaceManager";
  import {
    registerCommand,
    unregisterCommand,
    openPalette,
  } from "$lib/core/services/CommandRegistry";
  import {
    cheatSheetOpen,
    keybindPanelOpen,
  } from "$lib/core/services/ShortcutService";
  import { startTutorial } from "$lib/core/services/TutorialService";
  import { clearCitations } from "$lib/core/services/CitationRegistry";
  import {
    executeAllCells,
  } from "$lib/stores/notebookDocumentStore";
  import { generateMathematicalProof, loadProvenanceState } from "$lib/api";
  import LayoutRenderer from "$lib/components/layout/LayoutRenderer.svelte";
  import InfiniteCanvas from "$lib/components/layout/InfiniteCanvas.svelte";
  import WorkspaceControls from "$lib/components/workbench/WorkspaceControls.svelte";
  import QuickToolbar from "$lib/components/ui/QuickToolbar.svelte";


  let toolboxOpen = false;
  let workspaceControls: WorkspaceControls;

  function spawnWidget(type: WidgetType): void {
    if ($viewMode === "canvas") {
      addCanvasItem(type);
    } else {
      addWidgetToLayout(type);
    }
    toolboxOpen = false;
  }

  /** Download a proof document as a LaTeX .tex file. */
  function exportProofLatex(): void {
    // Proof generation is context-dependent; this stub triggers a download
    // of a placeholder proof. Widget-level context menus will supply
    // specific diagram data via generateMathematicalProof directly.
    const placeholder = `% SPIRE Proof Export\n% Select a diagram and use the context menu to generate a full proof.\n`;
    downloadAsText(placeholder, "spire_proof.tex");
  }

  /** Trigger a text file download in the browser. */
  function downloadAsText(content: string, filename: string): void {
    const blob = new Blob([content], { type: "text/plain;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }

  /** Load a provenance state from the clipboard or a prompt. */
  async function loadProvenanceFromClipboard(): Promise<void> {
    let json = "";
    try {
      json = await navigator.clipboard.readText();
    } catch {
      // Clipboard API may not be available; fall back to prompt.
      const input = window.prompt(
        "Paste the SPIRE Provenance Record JSON:",
      );
      if (!input) return;
      json = input;
    }
    if (!json.trim()) return;
    try {
      const state = await loadProvenanceState(json);
      console.log("[SPIRE] Provenance state restored:", state);
      // The returned state contains model, reaction, kinematics, seed.
      // Future phases will dispatch these to the appropriate stores.
      window.alert(
        `Provenance state loaded successfully.\nSeed: ${(state as Record<string,unknown>).seed ?? "unknown"}\nVersion: ${(state as Record<string,unknown>).version ?? "unknown"}`,
      );
    } catch (err) {
      console.error("[SPIRE] Provenance load failed:", err);
      window.alert(`Failed to load provenance: ${err}`);
    }
  }

  // --- Debounced auto-save (2 s) ---
  const debouncedAutoSave = debounce(autoSave, 2000);

  let unsubWidgets: (() => void) | null = null;
  let unsubInputs: (() => void) | null = null;
  let unsubFramework: (() => void) | null = null;

  // ── Global Command IDs ──────────────────────────────────
  const GLOBAL_CMD_IDS = [
    "spire.ui.add_analysis",
    "spire.ui.add_event_display",
    "spire.ui.add_lagrangian",
    "spire.ui.add_external_models",
    "spire.ui.add_compute_grid",
    "spire.ui.add_dalitz",
    "spire.ui.add_diagram_editor",
    "spire.ui.add_references",
    "spire.ui.add_telemetry",
    "spire.ui.add_plugin_manager",
    "spire.ui.add_global_fit_dashboard",
    "spire.ui.reset_layout",
    "spire.ui.toggle_log",
    "spire.view.toggle_canvas_mode",
    "spire.workspace.save",
    "spire.workspace.save_as",
    "spire.workspace.reset",
    "spire.palette.open",
    "spire.shortcut.cheatsheet",
    "spire.help.tutorial",
    "spire.references.clear_all",
    "spire.export.proof_latex",
    "spire.provenance.load",
    "spire.canvas.add_widget",
    "spire.canvas.zoom_in",
    "spire.canvas.zoom_out",
    "spire.canvas.reset_zoom",
    "spire.canvas.delete_selected",
    "spire.canvas.duplicate",
    "spire.canvas.select_all",
    "spire.notebook.run_cell",
    "spire.notebook.run_all",
    "spire.edit.undo",
    "spire.edit.redo",
    "spire.search.focus",
    "spire.settings.keybinds",
  ];

  function registerGlobalCommands(): void {
    registerCommand({
      id: "spire.palette.open",
      title: "Open Command Palette",
      category: "Navigation",
      shortcut: "Mod+K",
      execute: () => openPalette(),
      pinned: false,
    });
    registerCommand({
      id: "spire.ui.reset_layout",
      title: "Reset Layout",
      category: "View",
      shortcut: "Mod+Shift+R",
      execute: () => { resetDockingLayout(); clearCanvas(); },
      pinned: true,
      icon: "R",
    });
    registerCommand({
      id: "spire.ui.toggle_log",
      title: "Toggle Console",
      category: "View",
      shortcut: "Mod+J",
      execute: () => spawnWidget("log"),
    });
    registerCommand({
      id: "spire.view.toggle_canvas_mode",
      title: "Toggle Canvas / Docking Mode",
      category: "View",
      shortcut: "Mod+E",
      execute: () => toggleViewMode(),
    });
    registerCommand({
      id: "spire.workspace.save",
      title: "Save Workspace",
      category: "File",
      shortcut: "Mod+S",
      execute: () => downloadWorkspace(),
      pinned: true,
      icon: "S",
    });
    registerCommand({
      id: "spire.workspace.reset",
      title: "Reset Workspace",
      category: "File",
      execute: () => resetWorkspace(),
    });

    // Widget spawning commands
    registerCommand({
      id: "spire.ui.add_analysis",
      title: "Add Analysis Widget",
      category: "View",
      execute: () => spawnWidget("analysis"),
    });
    registerCommand({
      id: "spire.ui.add_event_display",
      title: "Add Event Display",
      category: "View",
      execute: () => spawnWidget("event_display"),
    });
    registerCommand({
      id: "spire.ui.add_lagrangian",
      title: "Add Lagrangian Workbench",
      category: "View",
      execute: () => spawnWidget("lagrangian_workbench"),
    });
    registerCommand({
      id: "spire.ui.add_external_models",
      title: "Add External Models Widget",
      category: "View",
      execute: () => spawnWidget("external_models"),
    });
    registerCommand({
      id: "spire.ui.add_compute_grid",
      title: "Add Compute Grid",
      category: "View",
      execute: () => spawnWidget("compute_grid"),
    });
    registerCommand({
      id: "spire.ui.add_dalitz",
      title: "Add Dalitz Plot",
      category: "View",
      execute: () => spawnWidget("dalitz"),
    });
    registerCommand({
      id: "spire.ui.add_diagram_editor",
      title: "Add Diagram Editor",
      category: "View",
      execute: () => spawnWidget("diagram_editor"),
    });
    registerCommand({
      id: "spire.ui.add_references",
      title: "Add References Panel",
      category: "View",
      execute: () => spawnWidget("references"),
    });
    registerCommand({
      id: "spire.ui.add_telemetry",
      title: "Add Telemetry Dashboard",
      category: "View",
      execute: () => spawnWidget("telemetry"),
    });
    registerCommand({
      id: "spire.ui.add_plugin_manager",
      title: "Add Plugin Manager",
      category: "View",
      execute: () => spawnWidget("plugin_manager"),
    });
    registerCommand({
      id: "spire.ui.add_global_fit_dashboard",
      title: "Add Global Fit Dashboard",
      category: "View",
      execute: () => spawnWidget("global_fit_dashboard"),
    });
    registerCommand({
      id: "spire.help.tutorial",
      title: "Start Tutorial",
      category: "Help",
      shortcut: "Mod+Shift+T",
      execute: () => startTutorial(),
      icon: "?",
    });
    registerCommand({
      id: "spire.references.clear_all",
      title: "Clear All Citations",
      category: "References",
      execute: () => clearCitations(),
    });
    registerCommand({
      id: "spire.export.proof_latex",
      title: "Export Proof (LaTeX)",
      category: "Export",
      execute: () => exportProofLatex(),
    });
    registerCommand({
      id: "spire.provenance.load",
      title: "Load Provenance State",
      category: "File",
      execute: () => loadProvenanceFromClipboard(),
    });

    // ── Canvas Commands ──
    registerCommand({
      id: "spire.canvas.add_widget",
      title: "Add Widget",
      category: "Canvas",
      shortcut: "Mod+N",
      execute: () => spawnWidget("notebook"),
      icon: "+",
    });
    registerCommand({
      id: "spire.canvas.zoom_in",
      title: "Zoom In",
      category: "Canvas",
      shortcut: "Mod+=",
      execute: () => {
        canvasViewport.update((vp) => ({
          ...vp,
          zoom: Math.min(5, vp.zoom * 1.15),
        }));
      },
    });
    registerCommand({
      id: "spire.canvas.zoom_out",
      title: "Zoom Out",
      category: "Canvas",
      shortcut: "Mod+-",
      execute: () => {
        canvasViewport.update((vp) => ({
          ...vp,
          zoom: Math.max(0.15, vp.zoom / 1.15),
        }));
      },
    });
    registerCommand({
      id: "spire.canvas.reset_zoom",
      title: "Reset Zoom",
      category: "Canvas",
      shortcut: "Mod+0",
      execute: () => {
        canvasViewport.update((vp) => ({ ...vp, zoom: 1 }));
      },
    });
    registerCommand({
      id: "spire.canvas.delete_selected",
      title: "Delete Selected Widget",
      category: "Canvas",
      shortcut: "Delete",
      execute: () => {
        // Canvas selection is managed locally by InfiniteCanvas;
        // this dispatches a custom event for the canvas to handle.
        window.dispatchEvent(new CustomEvent("spire:canvas:delete-selected"));
      },
    });
    registerCommand({
      id: "spire.canvas.duplicate",
      title: "Duplicate Widget",
      category: "Canvas",
      shortcut: "Mod+D",
      execute: () => {
        window.dispatchEvent(new CustomEvent("spire:canvas:duplicate-selected"));
      },
    });
    registerCommand({
      id: "spire.canvas.select_all",
      title: "Select All Widgets",
      category: "Canvas",
      shortcut: "Mod+A",
      execute: () => {
        window.dispatchEvent(new CustomEvent("spire:canvas:select-all"));
      },
    });

    // ── Notebook Commands ──
    registerCommand({
      id: "spire.notebook.run_cell",
      title: "Run Cell & Advance",
      category: "Notebook",
      shortcut: "Mod+Enter",
      execute: () => {
        // Notebook cells handle Shift+Enter locally; this is a global
        // fallback that broadcasts to the focused cell.
        window.dispatchEvent(new CustomEvent("spire:notebook:run-cell"));
      },
    });
    registerCommand({
      id: "spire.notebook.run_all",
      title: "Run All Cells",
      category: "Notebook",
      shortcut: "Mod+Shift+Enter",
      execute: () => executeAllCells(),
    });

    // ── Navigation ──
    registerCommand({
      id: "spire.shortcut.cheatsheet",
      title: "Keyboard Shortcuts",
      category: "Navigation",
      shortcut: "Mod+/",
      execute: () => cheatSheetOpen.update((v) => !v),
      icon: "⌨",
    });

    registerCommand({
      id: "spire.settings.keybinds",
      title: "Customize Keyboard Shortcuts",
      category: "Navigation",
      execute: () => keybindPanelOpen.update((v) => !v),
      icon: "⚙",
    });

    // ── Edit Commands ──
    registerCommand({
      id: "spire.edit.undo",
      title: "Undo",
      category: "Edit",
      shortcut: "Mod+Z",
      execute: () => document.execCommand("undo"),
    });
    registerCommand({
      id: "spire.edit.redo",
      title: "Redo",
      category: "Edit",
      shortcut: "Mod+Shift+Z",
      execute: () => document.execCommand("redo"),
    });
    registerCommand({
      id: "spire.search.focus",
      title: "Search",
      category: "Edit",
      shortcut: "Mod+F",
      execute: () => openPalette(),
    });

    // ── Chord: Save As ──
    registerCommand({
      id: "spire.workspace.save_as",
      title: "Save Workspace As…",
      category: "File",
      shortcut: "Mod+K Mod+S",
      execute: () => downloadWorkspace(),
    });
  }

  function unregisterGlobalCommands(): void {
    for (const id of GLOBAL_CMD_IDS) {
      unregisterCommand(id);
    }
  }

  onMount(() => {
    registerGlobalCommands();

    // Check for auto-save on mount
    workspaceControls?.checkAutoSave();

    // Subscribe to all state sources for auto-save
    unsubWidgets = layoutRoot.subscribe(() => debouncedAutoSave());
    unsubInputs = workspaceInputsSnapshot.subscribe(() => debouncedAutoSave());
    unsubFramework = activeFramework.subscribe(() => debouncedAutoSave());
  });

  onDestroy(() => {
    unregisterGlobalCommands();
    unsubWidgets?.();
    unsubInputs?.();
    unsubFramework?.();
  });
</script>

<div class="workbench">
  <!-- ─── Workspace Tab Bar ─── -->
  <div class="workspace-tabs">
    {#each $workspaces as ws (ws.id)}
      <div
        class="ws-tab"
        class:active={ws.id === $activeWorkspaceId}
        on:click={() => switchWorkspace(ws.id)}
        on:keydown={(e) => e.key === 'Enter' && switchWorkspace(ws.id)}
        title={ws.name}
        role="tab"
        tabindex="0"
        aria-selected={ws.id === $activeWorkspaceId}
      >
        <span class="ws-tab-label">{ws.name}</span>
        {#if $workspaces.length > 1}
          <button
            class="ws-tab-close"
            on:click|stopPropagation={() => removeWorkspace(ws.id)}
            aria-label="Close workspace"
          >&times;</button>
        {/if}
      </div>
    {/each}
    <button
      class="ws-tab ws-tab-add"
      on:click={() => addWorkspace()}
      title="New Workspace"
    >+</button>
  </div>

  <!-- ─── Toolbox Bar ─── -->
  <div class="toolbox-bar">
    <button
      class="toolbox-toggle"
      on:click={() => (toolboxOpen = !toolboxOpen)}
      aria-expanded={toolboxOpen}
    >
      + Add Widget
    </button>

    {#if toolboxOpen}
      <div class="toolbox-menu">
        {#each WIDGET_DEFINITIONS as def}
          <button class="toolbox-item" on:click={() => spawnWidget(def.type)}>
            {def.label}
          </button>
        {/each}
      </div>
    {/if}

    <QuickToolbar />

    <button
      class="toolbox-mode-toggle"
      on:click={toggleViewMode}
      title="Toggle Canvas / Docking (Ctrl+Shift+C)"
    >
      {$viewMode === "docking" ? "⊞ Docking" : "◎ Canvas"}
    </button>

    <div class="toolbox-spacer"></div>

    <WorkspaceControls bind:this={workspaceControls} />

    <span class="widget-count"
      >{$totalWidgetCount} widget{$totalWidgetCount !== 1 ? "s" : ""}</span
    >

    <button
      class="toolbox-reset"
      on:click={() => { resetDockingLayout(); clearCanvas(); }}
      title="Reset to default layout"
    >
      Reset Layout
    </button>
  </div>

  <!-- ─── Workspace Canvas ─── -->
  {#if $viewMode === "docking"}
    <div class="docking-canvas">
      <LayoutRenderer node={$layoutRoot} />
    </div>
  {:else}
    <InfiniteCanvas />
  {/if}
</div>

<style>
  .workbench {
    display: flex;
    flex-direction: column;
    height: 100%;
    gap: 0;
    overflow: hidden;
    max-width: 100vw;
  }

  /* ── Toolbox Bar ──────────────────────────────────────────── */
  .toolbox-bar {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.25rem 0.4rem;
    background: var(--bg-inset);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    position: relative;
    flex-wrap: wrap;
    min-width: 0;
  }
  .toolbox-toggle {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--fg-accent);
    padding: 0.2rem 0.6rem;
    font-size: 0.72rem;
    cursor: pointer;
    font-family: var(--font-mono);
    font-weight: 700;
    letter-spacing: 0.04em;
  }
  .toolbox-toggle:hover {
    border-color: var(--border-focus);
  }
  .toolbox-menu {
    position: absolute;
    top: 100%;
    left: 0.4rem;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    z-index: 100;
    min-width: 14rem;
  }
  .toolbox-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.35rem 0.6rem;
    background: none;
    border: none;
    border-bottom: 1px solid var(--border);
    color: var(--fg-primary);
    font-size: 0.75rem;
    cursor: pointer;
    font-family: var(--font-mono);
    text-align: left;
  }
  .toolbox-item:last-child {
    border-bottom: none;
  }
  .toolbox-item:hover {
    background: var(--bg-surface);
    color: var(--fg-accent);
  }
  .toolbox-spacer {
    flex: 1;
  }
  .toolbox-mode-toggle {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--fg-accent);
    padding: 0.2rem 0.6rem;
    font-size: 0.68rem;
    cursor: pointer;
    font-family: var(--font-mono);
    font-weight: 600;
    letter-spacing: 0.03em;
  }
  .toolbox-mode-toggle:hover {
    border-color: var(--border-focus);
  }
  .widget-count {
    font-size: 0.68rem;
    color: var(--fg-secondary);
  }
  .toolbox-reset {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--fg-secondary);
    padding: 0.2rem 0.5rem;
    font-size: 0.68rem;
    cursor: pointer;
    font-family: var(--font-mono);
  }
  .toolbox-reset:hover {
    color: var(--hl-error);
    border-color: var(--hl-error);
  }

  /* ── Docking Canvas ───────────────────────────────────────── */
  .docking-canvas {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    display: flex;
  }

  /* ── Workspace Tabs ──────────────────────────────────────── */
  .workspace-tabs {
    display: flex;
    align-items: stretch;
    background: var(--bg-primary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
    -ms-overflow-style: none;
    min-height: 1.5rem;
  }
  .workspace-tabs::-webkit-scrollbar { display: none; }

  .ws-tab {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.2rem 0.6rem;
    background: transparent;
    border: none;
    border-right: 1px solid var(--border);
    color: var(--fg-secondary);
    font-size: 0.65rem;
    font-family: var(--font-mono);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.1s, color 0.1s;
    flex-shrink: 0;
  }

  .ws-tab:hover {
    background: var(--bg-surface);
    color: var(--fg-primary);
  }

  .ws-tab.active {
    background: var(--bg-surface);
    color: var(--fg-accent);
    border-bottom: 2px solid var(--hl-symbol);
  }

  .ws-tab-label {
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 120px;
  }

  .ws-tab-close {
    background: none;
    border: none;
    font-size: 0.72rem;
    color: var(--fg-secondary);
    cursor: pointer;
    line-height: 1;
    padding: 0;
    font-family: var(--font-mono);
  }

  .ws-tab-close:hover {
    color: var(--hl-error);
  }

  .ws-tab-add {
    color: var(--fg-secondary);
    font-size: 0.82rem;
    font-weight: 700;
    padding: 0.2rem 0.5rem;
    border-right: none;
  }

  .ws-tab-add:hover {
    color: var(--hl-symbol);
    background: var(--bg-surface);
  }
</style>
