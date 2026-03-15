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
    dockingInsertPreference,
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
    reorderWorkspaces,
    renameWorkspace,
    setWorkspaceColor,
    duplicateWorkspace,
    WORKSPACE_COLORS,
    setWorkspaceDescription,
  } from "$lib/stores/layoutStore";
  import { showContextMenu } from "$lib/stores/contextMenuStore";
  import { activeFramework, theoreticalModel } from "$lib/stores/physicsStore";
  import {
    downloadWorkspace,
    resetWorkspace,
  } from "$lib/services/workspaceManager";
  import {
    applyLayoutPreset,
    BUILT_IN_PRESETS,
    listCustomPresets,
    saveCurrentLayoutPreset,
  } from "$lib/services/PresetService";
  import {
    registerCommand,
    unregisterCommand,
    openPalette,
  } from "$lib/core/services/CommandRegistry";
  import {
    pipelineGraph,
    replacePipelineGraph,
    runPipelineAutoLayout,
    runPipelineExecution,
  } from "$lib/stores/pipelineGraphStore";
  import {
    downloadPipelineToFile,
    openPipelineFilePicker,
  } from "$lib/core/pipeline/serialization";
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
  import WorkspaceTabs from "$lib/components/workspace/WorkspaceTabs.svelte";
  import { tooltip } from "$lib/actions/tooltip";


  let toolboxOpen = false;
  let customizerOpen = false;
  let workspaceControls: WorkspaceControls;
  let toolboxQuery = "";
  let widgetSortMode: "workflow" | "alpha" = "workflow";
  let showWidgetCount = true;
  let compactToolbar = false;
  let toolboxButtonEl: HTMLButtonElement | null = null;
  let toolboxMenuEl: HTMLDivElement | null = null;
  let customizerButtonEl: HTMLButtonElement | null = null;
  let customizerMenuEl: HTMLDivElement | null = null;
  const TOOLBOX_PREFS_KEY = "spire.toolbox.prefs.v1";

  $: sortedWidgetDefinitions =
    widgetSortMode === "alpha"
      ? [...WIDGET_DEFINITIONS].sort((a, b) => a.label.localeCompare(b.label))
      : [...WIDGET_DEFINITIONS];

  $: visibleWidgetDefinitions =
    toolboxQuery.trim().length === 0
      ? sortedWidgetDefinitions
      : sortedWidgetDefinitions.filter((def) => {
          const q = toolboxQuery.toLowerCase();
          return def.label.toLowerCase().includes(q) || def.type.toLowerCase().includes(q);
        });

  function spawnWidget(type: WidgetType): void {
    if ($viewMode === "canvas") {
      addCanvasItem(type);
    } else {
      addWidgetToLayout(type, "auto");
    }
    toolboxOpen = false;
  }

  function handleToolboxSearchKeydown(event: KeyboardEvent): void {
    if (event.key === "Enter" && visibleWidgetDefinitions.length > 0) {
      event.preventDefault();
      spawnWidget(visibleWidgetDefinitions[0].type);
      return;
    }
    if (event.key === "Escape") {
      toolboxOpen = false;
    }
  }

  function handleWindowPointerDown(event: MouseEvent): void {
    const target = event.target as Node | null;
    if (!target) return;

    const inToolbox =
      toolboxMenuEl?.contains(target) || toolboxButtonEl?.contains(target);
    if (!inToolbox) {
      toolboxOpen = false;
    }

    const inCustomizer =
      customizerMenuEl?.contains(target) || customizerButtonEl?.contains(target);
    if (!inCustomizer) {
      customizerOpen = false;
    }
  }

  function handleWindowEscape(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      toolboxOpen = false;
      customizerOpen = false;
    }
  }

  function loadToolboxPreferences(): void {
    try {
      const raw = localStorage.getItem(TOOLBOX_PREFS_KEY);
      if (!raw) return;
      const parsed = JSON.parse(raw) as {
        sort?: "workflow" | "alpha";
        showWidgetCount?: boolean;
        compactToolbar?: boolean;
      };
      if (parsed.sort === "workflow" || parsed.sort === "alpha") {
        widgetSortMode = parsed.sort;
      }
      if (typeof parsed.showWidgetCount === "boolean") {
        showWidgetCount = parsed.showWidgetCount;
      }
      if (typeof parsed.compactToolbar === "boolean") {
        compactToolbar = parsed.compactToolbar;
      }
    } catch {
      // Ignore malformed settings.
    }
  }

  $: if (typeof window !== "undefined") {
    localStorage.setItem(
      TOOLBOX_PREFS_KEY,
      JSON.stringify({
        sort: widgetSortMode,
        showWidgetCount,
        compactToolbar,
      }),
    );
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

  let unsubIdentityModel: (() => void) | null = null;
  let unsubIdentityFramework: (() => void) | null = null;

  function inferWorkspaceDescription(modelName: string | null, framework: string): string {
    const normalized = (modelName ?? "").trim().toLowerCase();
    if (normalized.includes("standard model") || normalized === "sm") {
      return "SM – Precision EW";
    }
    if (normalized.includes("mssm")) {
      return "MSSM – Scan #3";
    }
    if (normalized.includes("ufo")) {
      return `UFO [${modelName}]`;
    }
    if (modelName && modelName.trim().length > 0) {
      return `${modelName} – ${framework}`;
    }
    return `${framework} – Exploratory Session`;
  }

  function refreshActiveWorkspaceIdentity(): void {
    const ws = $workspaces.find((item) => item.id === $activeWorkspaceId);
    if (!ws || !ws.autoDescription) return;
    const description = inferWorkspaceDescription($theoreticalModel?.name ?? null, $activeFramework);
    if (description !== ws.description) {
      setWorkspaceDescription(ws.id, description, true);
    }
  }

  $: if ($activeWorkspaceId) {
    refreshActiveWorkspaceIdentity();
  }

  // ── Global Command IDs ──────────────────────────────────
  const GLOBAL_CMD_IDS = [
    "spire.ui.add_analysis",
    "spire.ui.add_event_display",
    "spire.ui.add_particle_atlas",
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
    "spire.layout.save_preset",
    "spire.layout.apply.theory-studio",
    "spire.layout.apply.analysis-focus",
    "spire.layout.apply.minimal",
    "spire.pipeline.format_graph",
    "spire.pipeline.run",
    "spire.pipeline.export",
    "spire.pipeline.import",
  ];

  let dynamicPresetCommandIds: string[] = [];

  function refreshCustomPresetCommands(): void {
    for (const id of dynamicPresetCommandIds) {
      unregisterCommand(id);
    }
    dynamicPresetCommandIds = [];

    for (const preset of listCustomPresets()) {
      const id = `spire.layout.apply.custom.${preset.id}`;
      dynamicPresetCommandIds.push(id);
      registerCommand({
        id,
        title: `Apply Layout Preset: ${preset.name}`,
        category: "Layout",
        execute: () => {
          applyLayoutPreset(preset.id);
        },
      });
    }
  }

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
      id: "spire.ui.add_particle_atlas",
      title: "Add Particle Atlas",
      category: "View",
      execute: () => spawnWidget("particle_atlas"),
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

    registerCommand({
      id: "spire.layout.save_preset",
      title: "Save Current Layout as Preset",
      category: "Layout",
      execute: () => {
        const name = window.prompt("Preset name:", "My Preset");
        if (!name) return;
        saveCurrentLayoutPreset(name);
        refreshCustomPresetCommands();
      },
    });

    for (const builtIn of BUILT_IN_PRESETS) {
      registerCommand({
        id: `spire.layout.apply.${builtIn.id}`,
        title: `Apply Layout Preset: ${builtIn.name}`,
        category: "Layout",
        execute: () => {
          applyLayoutPreset(builtIn.id);
        },
      });
    }

    registerCommand({
      id: "spire.pipeline.format_graph",
      title: "Format Pipeline Graph",
      category: "Pipeline",
      shortcut: "Mod+Shift+G",
      execute: () => runPipelineAutoLayout($canvasItems),
      icon: "⇢",
    });

    registerCommand({
      id: "spire.pipeline.run",
      title: "Run Pipeline",
      category: "Pipeline",
      shortcut: "Mod+Shift+R",
      execute: () => void runPipelineExecution(),
      icon: "▶",
    });

    registerCommand({
      id: "spire.pipeline.export",
      title: "Export Pipeline to File",
      category: "Pipeline",
      execute: () => downloadPipelineToFile($pipelineGraph),
      icon: "⇩",
    });

    registerCommand({
      id: "spire.pipeline.import",
      title: "Import Pipeline from File",
      category: "Pipeline",
      execute: async () => {
        const graph = await openPipelineFilePicker();
        if (!graph) return;
        replacePipelineGraph(graph);
      },
      icon: "⇧",
    });

    refreshCustomPresetCommands();
  }

  function unregisterGlobalCommands(): void {
    for (const id of GLOBAL_CMD_IDS) {
      unregisterCommand(id);
    }
    for (const id of dynamicPresetCommandIds) {
      unregisterCommand(id);
    }
    dynamicPresetCommandIds = [];
  }

  onMount(() => {
    loadToolboxPreferences();
    registerGlobalCommands();
    workspaceControls?.checkAutoSave();
    window.addEventListener("mousedown", handleWindowPointerDown, true);
    window.addEventListener("keydown", handleWindowEscape, true);

    unsubIdentityModel = theoreticalModel.subscribe(() => refreshActiveWorkspaceIdentity());
    unsubIdentityFramework = activeFramework.subscribe(() => refreshActiveWorkspaceIdentity());
    refreshActiveWorkspaceIdentity();
  });

  onDestroy(() => {
    unregisterGlobalCommands();
    window.removeEventListener("mousedown", handleWindowPointerDown, true);
    window.removeEventListener("keydown", handleWindowEscape, true);
    unsubIdentityModel?.();
    unsubIdentityFramework?.();
  });
</script>

<div class="workbench">
  <!-- ─── Workspace Tab Bar ─── -->
  <WorkspaceTabs />

  <!-- ─── Toolbox Bar ─── -->
  <div class="toolbox-bar" class:toolbox-compact={compactToolbar}>
    <button
      class="toolbox-toggle"
      bind:this={toolboxButtonEl}
      on:click={() => {
        toolboxOpen = !toolboxOpen;
        if (toolboxOpen) customizerOpen = false;
      }}
      aria-expanded={toolboxOpen}
    >
      + Add Widget
    </button>

    {#if toolboxOpen}
      <div class="toolbox-menu" bind:this={toolboxMenuEl}>
        <div class="toolbox-menu-controls">
          <input
            class="toolbox-search"
            type="search"
            placeholder="Find widget…"
            bind:value={toolboxQuery}
            on:keydown={handleToolboxSearchKeydown}
          />
          <select class="toolbox-sort" bind:value={widgetSortMode}>
            <option value="workflow">Workflow</option>
            <option value="alpha">A-Z</option>
          </select>
        </div>
        {#if visibleWidgetDefinitions.length === 0}
          <div class="toolbox-empty">No widgets match “{toolboxQuery}”.</div>
        {:else}
          {#each visibleWidgetDefinitions as def}
            <button class="toolbox-item" on:click={() => spawnWidget(def.type)}>
              {def.label}
            </button>
          {/each}
        {/if}
      </div>
    {/if}

    <button
      class="toolbox-customize"
      bind:this={customizerButtonEl}
      on:click={() => {
        customizerOpen = !customizerOpen;
        if (customizerOpen) toolboxOpen = false;
      }}
      use:tooltip={{ text: "Customize interface" }}
    >
      Customize ▾
    </button>

    {#if customizerOpen}
      <div class="customize-menu" bind:this={customizerMenuEl}>
        <label class="customize-toggle">
          <input type="checkbox" bind:checked={compactToolbar} />
          Compact toolbar
        </label>
        <label class="customize-toggle">
          <input type="checkbox" bind:checked={showWidgetCount} />
          Show widget count
        </label>

        <div class="customize-block">
          <span>Dock insertion</span>
          <div class="customize-options">
            <button
              class:active={$dockingInsertPreference === "smart"}
              on:click={() => dockingInsertPreference.set("smart")}
            >Smart</button>
            <button
              class:active={$dockingInsertPreference === "row"}
              on:click={() => dockingInsertPreference.set("row")}
            >Row</button>
            <button
              class:active={$dockingInsertPreference === "col"}
              on:click={() => dockingInsertPreference.set("col")}
            >Column</button>
          </div>
        </div>

        <button
          class="customize-keybinds"
          on:click={() => {
            keybindPanelOpen.set(true);
            customizerOpen = false;
          }}
        >
          Edit keyboard shortcuts
        </button>
      </div>
    {/if}

    <QuickToolbar />

    <button
      class="toolbox-mode-toggle"
      on:click={toggleViewMode}
      use:tooltip={{ text: "Toggle Canvas / Docking (Ctrl+Shift+C)" }}
    >
      {$viewMode === "docking" ? "⊞ Docking" : "◎ Canvas"}
    </button>

    <div class="toolbox-spacer"></div>

    <WorkspaceControls bind:this={workspaceControls} />

    {#if showWidgetCount}
      <span class="widget-count"
        >{$totalWidgetCount} widget{$totalWidgetCount !== 1 ? "s" : ""}</span
      >
    {/if}

    <button
      class="toolbox-reset"
      on:click={() => { resetDockingLayout(); clearCanvas(); }}
      use:tooltip={{ text: "Reset to default layout" }}
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
  .toolbox-bar.toolbox-compact {
    gap: 0.25rem;
    padding: 0.18rem 0.32rem;
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
    max-height: min(62vh, 560px);
    overflow-y: auto;
  }
  .toolbox-menu-controls {
    display: flex;
    gap: 0.25rem;
    padding: 0.35rem;
    border-bottom: 1px solid var(--border);
    background: var(--bg-inset);
    position: sticky;
    top: 0;
    z-index: 1;
  }
  .toolbox-search {
    flex: 1;
    min-width: 0;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    font-size: 0.7rem;
    padding: 0.2rem 0.3rem;
    font-family: var(--font-mono);
  }
  .toolbox-sort {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--fg-secondary);
    font-size: 0.68rem;
    padding: 0.15rem 0.2rem;
    font-family: var(--font-mono);
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
  .toolbox-empty {
    padding: 0.5rem 0.6rem;
    color: var(--fg-secondary);
    font-size: 0.7rem;
    font-style: italic;
  }
  .toolbox-customize {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--fg-secondary);
    padding: 0.2rem 0.5rem;
    font-size: 0.68rem;
    cursor: pointer;
    font-family: var(--font-mono);
  }
  .toolbox-customize:hover {
    border-color: var(--border-focus);
    color: var(--fg-primary);
  }
  .customize-menu {
    position: absolute;
    top: 100%;
    left: 9rem;
    z-index: 101;
    min-width: 14rem;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding: 0.45rem;
  }
  .customize-toggle {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.7rem;
    color: var(--fg-primary);
  }
  .customize-block {
    border-top: 1px solid var(--border);
    padding-top: 0.35rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.66rem;
    color: var(--fg-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .customize-options {
    display: flex;
    gap: 0.25rem;
  }
  .customize-options button,
  .customize-keybinds {
    border: 1px solid var(--border);
    background: var(--bg-surface);
    color: var(--fg-secondary);
    font-family: var(--font-mono);
    font-size: 0.65rem;
    padding: 0.15rem 0.35rem;
    cursor: pointer;
  }
  .customize-options button.active {
    color: var(--fg-accent);
    border-color: var(--hl-symbol);
  }
  .customize-keybinds {
    align-self: flex-start;
    margin-top: 0.1rem;
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

</style>
