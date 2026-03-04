<!--
  SPIRE Desktop — Adaptive Workbench Page

  Dynamic grid canvas driven by the notebook store.
  Widgets are placed on a CSS Grid whose column / row
  positions come from each WidgetInstance.  A compact
  Toolbox bar lets the user spawn new widgets.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    widgets,
    addWidget,
    resetLayout,
    WIDGET_DEFINITIONS,
    GRID_COLUMNS,
  } from "$lib/stores/notebookStore";
  import type { WidgetType } from "$lib/stores/notebookStore";
  import { workspaceInputsSnapshot } from "$lib/stores/workspaceInputsStore";
  import { activeFramework } from "$lib/stores/physicsStore";
  import {
    autoSave,
    debounce,
    hasAutoSave,
    downloadWorkspace,
    resetWorkspace,
  } from "$lib/services/workspaceManager";
  import {
    registerCommand,
    unregisterCommand,
    openPalette,
  } from "$lib/core/services/CommandRegistry";
  import WidgetCell from "$lib/components/workbench/WidgetCell.svelte";
  import WorkspaceControls from "$lib/components/workbench/WorkspaceControls.svelte";
  import QuickToolbar from "$lib/components/ui/QuickToolbar.svelte";

  let toolboxOpen = false;
  let workspaceControls: WorkspaceControls;

  function spawnWidget(type: WidgetType): void {
    addWidget(type);
    toolboxOpen = false;
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
    "spire.ui.reset_layout",
    "spire.ui.toggle_log",
    "spire.workspace.save",
    "spire.workspace.reset",
    "spire.palette.open",
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
      execute: () => resetLayout(),
      pinned: true,
      icon: "R",
    });
    registerCommand({
      id: "spire.ui.toggle_log",
      title: "Toggle Console",
      category: "View",
      shortcut: "Mod+J",
      execute: () => addWidget("log"),
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
      execute: () => addWidget("analysis"),
    });
    registerCommand({
      id: "spire.ui.add_event_display",
      title: "Add Event Display",
      category: "View",
      execute: () => addWidget("event_display"),
    });
    registerCommand({
      id: "spire.ui.add_lagrangian",
      title: "Add Lagrangian Workbench",
      category: "View",
      execute: () => addWidget("lagrangian_workbench"),
    });
    registerCommand({
      id: "spire.ui.add_external_models",
      title: "Add External Models Widget",
      category: "View",
      execute: () => addWidget("external_models"),
    });
    registerCommand({
      id: "spire.ui.add_compute_grid",
      title: "Add Compute Grid",
      category: "View",
      execute: () => addWidget("compute_grid"),
    });
    registerCommand({
      id: "spire.ui.add_dalitz",
      title: "Add Dalitz Plot",
      category: "View",
      execute: () => addWidget("dalitz"),
    });
    registerCommand({
      id: "spire.ui.add_diagram_editor",
      title: "Add Diagram Editor",
      category: "View",
      execute: () => addWidget("diagram_editor"),
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
    unsubWidgets = widgets.subscribe(() => debouncedAutoSave());
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
            <span class="toolbox-size">{def.defaultColSpan}&times;{def.defaultRowSpan}</span>
          </button>
        {/each}
      </div>
    {/if}

    <QuickToolbar />

    <div class="toolbox-spacer"></div>

    <WorkspaceControls bind:this={workspaceControls} />

    <span class="widget-count">{$widgets.length} widget{$widgets.length !== 1 ? "s" : ""}</span>

    <button class="toolbox-reset" on:click={resetLayout} title="Reset to default layout">
      Reset Layout
    </button>
  </div>

  <!-- ─── Grid Canvas ─── -->
  <div
    class="grid-canvas"
    style="grid-template-columns: repeat({GRID_COLUMNS}, 1fr);"
  >
    {#each $widgets as instance (instance.id)}
      <div
        class="grid-cell"
        style="
          grid-column: {instance.col} / span {instance.colSpan};
          grid-row: {instance.row} / span {instance.rowSpan};
        "
      >
        <WidgetCell {instance} />
      </div>
    {/each}
  </div>
</div>

<style>
  .workbench {
    display: flex;
    flex-direction: column;
    height: 100%;
    gap: 0;
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
  .toolbox-size {
    font-size: 0.62rem;
    color: var(--fg-secondary);
  }
  .toolbox-spacer {
    flex: 1;
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

  /* ── Grid Canvas ──────────────────────────────────────────── */
  .grid-canvas {
    flex: 1;
    display: grid;
    gap: 0.35rem;
    padding: 0.35rem;
    grid-auto-rows: minmax(160px, 1fr);
    min-height: 0;
    overflow: auto;
  }
  .grid-cell {
    min-width: 0;
    min-height: 0;
  }

  /* ── Responsive ───────────────────────────────────────────── */
  @media (max-width: 900px) {
    .grid-canvas {
      grid-template-columns: 1fr !important;
    }
    .grid-cell {
      grid-column: 1 !important;
      grid-row: auto !important;
    }
  }
</style>
