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
    insertWidgetRelative,
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
  let showQuickToolbar = true;
  let showWorkspaceActions = true;
  let showPaletteLauncher = true;
  let showTutorialLauncher = true;
  let showViewModeToggle = true;
  let showResetButton = true;
  let showToolbarShortcuts = true;
  let toolboxButtonEl: HTMLButtonElement | null = null;
  let toolboxMenuEl: HTMLDivElement | null = null;
  let customizerButtonEl: HTMLButtonElement | null = null;
  let customizerMenuEl: HTMLDivElement | null = null;
  let placementPointerId: number | null = null;
  let placementWidgetType: WidgetType | null = null;
  let placementStartX = 0;
  let placementStartY = 0;
  let placementClientX = 0;
  let placementClientY = 0;
  let placementActive = false;
  let suppressToolboxClickForType: WidgetType | null = null;

  type DockPreviewZone = "left" | "right" | "top" | "bottom" | "center";
  let dockPlacementPreview:
    | { left: number; top: number; width: number; height: number; zone: DockPreviewZone }
    | null = null;
  const PLACEMENT_DRAG_THRESHOLD = 8;
  const TOOLBOX_PREFS_KEY = "spire.toolbox.prefs.v2";

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

  function handleToolboxItemClick(type: WidgetType): void {
    if (suppressToolboxClickForType === type) {
      suppressToolboxClickForType = null;
      return;
    }
    spawnWidget(type);
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

  function handleWindowPointerDown(event: PointerEvent): void {
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

  function handleToolboxItemPointerDown(event: PointerEvent, type: WidgetType): void {
    if (event.button !== 0) return;
    event.preventDefault();
    placementPointerId = event.pointerId;
    placementWidgetType = type;
    placementStartX = event.clientX;
    placementStartY = event.clientY;
    placementClientX = event.clientX;
    placementClientY = event.clientY;
    placementActive = false;
  }

  function handleToolboxItemDragStart(event: DragEvent, type: WidgetType): void {
    if (!event.dataTransfer) return;
    event.dataTransfer.effectAllowed = "copy";
    event.dataTransfer.setData("application/x-spire-widget-type", type);
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
        showQuickToolbar?: boolean;
        showWorkspaceActions?: boolean;
        showPaletteLauncher?: boolean;
        showTutorialLauncher?: boolean;
        showViewModeToggle?: boolean;
        showResetButton?: boolean;
        showToolbarShortcuts?: boolean;
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
      if (typeof parsed.showQuickToolbar === "boolean") {
        showQuickToolbar = parsed.showQuickToolbar;
      }
      if (typeof parsed.showWorkspaceActions === "boolean") {
        showWorkspaceActions = parsed.showWorkspaceActions;
      }
      if (typeof parsed.showPaletteLauncher === "boolean") {
        showPaletteLauncher = parsed.showPaletteLauncher;
      }
      if (typeof parsed.showTutorialLauncher === "boolean") {
        showTutorialLauncher = parsed.showTutorialLauncher;
      }
      if (typeof parsed.showViewModeToggle === "boolean") {
        showViewModeToggle = parsed.showViewModeToggle;
      }
      if (typeof parsed.showResetButton === "boolean") {
        showResetButton = parsed.showResetButton;
      }
      if (typeof parsed.showToolbarShortcuts === "boolean") {
        showToolbarShortcuts = parsed.showToolbarShortcuts;
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
        showQuickToolbar,
        showWorkspaceActions,
        showPaletteLauncher,
        showTutorialLauncher,
        showViewModeToggle,
        showResetButton,
        showToolbarShortcuts,
      }),
    );
  }

  function detectDockDropZone(target: HTMLElement, clientX: number, clientY: number): "left" | "right" | "top" | "bottom" | "center" {
    const rect = target.getBoundingClientRect();
    const relX = (clientX - rect.left) / Math.max(rect.width, 1);
    const relY = (clientY - rect.top) / Math.max(rect.height, 1);
    const edgeThreshold = 0.25;
    if (relX < edgeThreshold) return "left";
    if (relX > 1 - edgeThreshold) return "right";
    if (relY < edgeThreshold) return "top";
    if (relY > 1 - edgeThreshold) return "bottom";
    return "center";
  }

  function resolveDockingTargetAtPoint(clientX: number, clientY: number): {
    targetId: string;
    zone: DockPreviewZone;
    rect: DOMRect;
  } | null {
    const hitElements = document.elementsFromPoint(clientX, clientY) as HTMLElement[];
    const dockingTarget = hitElements.find((el) => el.dataset?.dockingDropTarget);
    const targetId = dockingTarget?.dataset?.dockingDropTarget;
    if (!dockingTarget || !targetId) return null;

    const zone = detectDockDropZone(dockingTarget, clientX, clientY);
    const rect = dockingTarget.getBoundingClientRect();
    return { targetId, zone, rect };
  }

  function updateDockPlacementPreview(clientX: number, clientY: number): void {
    if ($viewMode !== "docking") {
      dockPlacementPreview = null;
      return;
    }

    const target = resolveDockingTargetAtPoint(clientX, clientY);
    if (!target) {
      dockPlacementPreview = null;
      return;
    }

    dockPlacementPreview = {
      left: target.rect.left,
      top: target.rect.top,
      width: target.rect.width,
      height: target.rect.height,
      zone: target.zone,
    };
  }

  function placeWidgetAtPointer(type: WidgetType, clientX: number, clientY: number): void {
    const hitElements = document.elementsFromPoint(clientX, clientY) as HTMLElement[];

    if ($viewMode === "canvas") {
      const canvasSurface = hitElements.find((el) => el.classList.contains("infinite-canvas"));
      if (canvasSurface) {
        const rect = canvasSurface.getBoundingClientRect();
        const viewport = $canvasViewport;
        const worldX = (clientX - rect.left - viewport.panX) / viewport.zoom;
        const worldY = (clientY - rect.top - viewport.panY) / viewport.zoom;
        addCanvasItem(type, worldX, worldY);
        toolboxOpen = false;
        return;
      }
    }

    const dockingTarget = hitElements.find((el) => el.dataset?.dockingDropTarget);
    if (dockingTarget?.dataset.dockingDropTarget) {
      const dropZone = detectDockDropZone(dockingTarget, clientX, clientY);
      insertWidgetRelative(dockingTarget.dataset.dockingDropTarget, dropZone, type);
      dockPlacementPreview = null;
      toolboxOpen = false;
      return;
    }

    spawnWidget(type);
  }

  function clearToolboxPlacement(): void {
    placementPointerId = null;
    placementWidgetType = null;
    placementActive = false;
    dockPlacementPreview = null;
  }

  function handleToolboxPlacementMove(event: PointerEvent): void {
    if (placementPointerId === null || event.pointerId !== placementPointerId) return;
    placementClientX = event.clientX;
    placementClientY = event.clientY;

    if (!placementActive) {
      const dx = event.clientX - placementStartX;
      const dy = event.clientY - placementStartY;
      if (Math.hypot(dx, dy) >= PLACEMENT_DRAG_THRESHOLD) {
        placementActive = true;
        // In docking mode, close the toolbox menu so it doesn't block
        // document.elementsFromPoint() from finding docking drop targets.
        if ($viewMode === "docking") toolboxOpen = false;
      }
    }

    if (placementActive) {
      updateDockPlacementPreview(event.clientX, event.clientY);
      event.preventDefault();
    }
  }

  function handleToolboxPlacementEnd(event: PointerEvent): void {
    if (placementPointerId === null || event.pointerId !== placementPointerId || !placementWidgetType) return;
    const type = placementWidgetType;
    const didPlaceByDrag = placementActive;
    const dropX = event.clientX;
    const dropY = event.clientY;
    clearToolboxPlacement();
    suppressToolboxClickForType = type;

    if (didPlaceByDrag) {
      placeWidgetAtPointer(type, dropX, dropY);
      return;
    }

    spawnWidget(type);
  }

  function handleToolboxPlacementCancel(event: PointerEvent): void {
    if (placementPointerId !== null && event.pointerId === placementPointerId) {
      clearToolboxPlacement();
    }
  }

  function handleDockingCanvasDragOver(event: DragEvent): void {
    if (!$viewMode || $viewMode !== "docking") return;
    if (!event.dataTransfer?.types.includes("application/x-spire-widget-type")) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "copy";
    updateDockPlacementPreview(event.clientX, event.clientY);
  }

  function handleDockingCanvasDrop(event: DragEvent): void {
    if ($viewMode !== "docking") return;
    const type = event.dataTransfer?.getData("application/x-spire-widget-type");
    if (!type) return;
    event.preventDefault();
    const target = resolveDockingTargetAtPoint(event.clientX, event.clientY);
    if (target) {
      insertWidgetRelative(target.targetId, target.zone, type as WidgetType);
    } else {
      addWidgetToLayout(type as WidgetType, "auto");
    }
    dockPlacementPreview = null;
  }

  function handleDockingCanvasDragLeave(event: DragEvent): void {
    const next = event.relatedTarget as Node | null;
    if (next && (event.currentTarget as HTMLElement)?.contains(next)) return;
    dockPlacementPreview = null;
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
      title: "Add Unified Diagram Visualizer",
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
      icon: "CFG",
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
      icon: "RUN",
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
    window.addEventListener("pointerdown", handleWindowPointerDown, true);
    window.addEventListener("pointermove", handleToolboxPlacementMove, { passive: false });
    window.addEventListener("pointerup", handleToolboxPlacementEnd, true);
    window.addEventListener("pointercancel", handleToolboxPlacementCancel, true);
    window.addEventListener("keydown", handleWindowEscape, true);

    unsubIdentityModel = theoreticalModel.subscribe(() => refreshActiveWorkspaceIdentity());
    unsubIdentityFramework = activeFramework.subscribe(() => refreshActiveWorkspaceIdentity());
    refreshActiveWorkspaceIdentity();
  });

  onDestroy(() => {
    unregisterGlobalCommands();
    window.removeEventListener("pointerdown", handleWindowPointerDown, true);
    window.removeEventListener("pointermove", handleToolboxPlacementMove);
    window.removeEventListener("pointerup", handleToolboxPlacementEnd, true);
    window.removeEventListener("pointercancel", handleToolboxPlacementCancel, true);
    window.removeEventListener("keydown", handleWindowEscape, true);
    unsubIdentityModel?.();
    unsubIdentityFramework?.();
  });
</script>

<div class="workbench">
  <!-- ─── Workspace Tab Bar ─── -->
  <div data-tour-id="workspace-tabs">
    <WorkspaceTabs />
  </div>

  <!-- ─── Toolbox Bar ─── -->
  <div class="toolbox-bar" class:toolbox-compact={compactToolbar}>
    <div class="toolbar-group toolbar-launchers">
      <div class="toolbar-menu-anchor">
        <button
          class="toolbox-toggle"
          bind:this={toolboxButtonEl}
          on:click={() => {
            toolboxOpen = !toolboxOpen;
            if (toolboxOpen) customizerOpen = false;
          }}
          aria-expanded={toolboxOpen}
          data-tour-id="add-widget-button"
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
            <div class="toolbox-drag-hint">Click to add instantly, or press and drag from the list to place directly in canvas or docking mode.</div>
            {#if visibleWidgetDefinitions.length === 0}
              <div class="toolbox-empty">No widgets match “{toolboxQuery}”.</div>
            {:else}
              {#each visibleWidgetDefinitions as def}
                <button
                  class="toolbox-item"
                  draggable="true"
                  on:click={() => handleToolboxItemClick(def.type)}
                  on:pointerdown={(event) => handleToolboxItemPointerDown(event, def.type)}
                  on:dragstart={(event) => handleToolboxItemDragStart(event, def.type)}
                >
                  <span>{def.label}</span>
                </button>
              {/each}
            {/if}
          </div>
        {/if}
      </div>

      <div class="toolbar-menu-anchor">
        <button
          class="toolbox-customize"
          bind:this={customizerButtonEl}
          on:click={() => {
            customizerOpen = !customizerOpen;
            if (customizerOpen) toolboxOpen = false;
          }}
          use:tooltip={{ text: "Customize interface" }}
          data-tour-id="customize-interface-button"
        >
          Customize ▾
        </button>

        {#if customizerOpen}
          <div class="customize-menu" bind:this={customizerMenuEl}>
            <div class="customize-section">
              <span class="customize-heading">Toolbar Layout</span>
              <label class="customize-toggle">
                <input type="checkbox" bind:checked={compactToolbar} />
                Compact toolbar
              </label>
              <label class="customize-toggle">
                <input type="checkbox" bind:checked={showWidgetCount} />
                Show widget count
              </label>
              <label class="customize-toggle">
                <input type="checkbox" bind:checked={showQuickToolbar} />
                Show quick toolbar
              </label>
              <label class="customize-toggle">
                <input type="checkbox" bind:checked={showWorkspaceActions} />
                Show workspace controls
              </label>
            </div>

            <div class="customize-section">
              <span class="customize-heading">Helpers</span>
              <label class="customize-toggle">
                <input type="checkbox" bind:checked={showPaletteLauncher} />
                Show palette launcher
              </label>
              <label class="customize-toggle">
                <input type="checkbox" bind:checked={showTutorialLauncher} />
                Show tutorial launcher
              </label>
              <label class="customize-toggle">
                <input type="checkbox" bind:checked={showViewModeToggle} />
                Show view-mode toggle
              </label>
              <label class="customize-toggle">
                <input type="checkbox" bind:checked={showToolbarShortcuts} />
                Show palette/tutorial chips
              </label>
              <label class="customize-toggle">
                <input type="checkbox" bind:checked={showResetButton} />
                Show reset button
              </label>
            </div>

            <div class="customize-section customize-block">
              <span class="customize-heading">Workflow links</span>
              <div class="customize-links">
                <button class="customize-link" on:click={() => openPalette()}>Command palette ↗</button>
                <button class="customize-link" on:click={() => { keybindPanelOpen.set(true); customizerOpen = false; }}>Keyboard shortcuts ↗</button>
                <button class="customize-link" on:click={() => { startTutorial(); customizerOpen = false; }}>Tutorial ↗</button>
              </div>
            </div>

            <div class="customize-section customize-block">
              <span class="customize-heading">Dock insertion</span>
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

            <div class="customize-actions">
              <button class="customize-keybinds" on:click={() => openPalette()}>
                Open command palette
              </button>
              <button
                class="customize-keybinds"
                on:click={() => {
                  keybindPanelOpen.set(true);
                  customizerOpen = false;
                }}
              >
                Edit keyboard shortcuts
              </button>
              <button
                class="customize-keybinds"
                on:click={() => {
                  startTutorial();
                  customizerOpen = false;
                }}
              >
                Start guided tutorial
              </button>
            </div>
          </div>
        {/if}
      </div>
    </div>

    {#if showToolbarShortcuts}
    <div class="toolbar-group toolbar-shortcuts">
      {#if showPaletteLauncher}
        <button
          class="toolbar-chip"
          on:click={() => openPalette()}
          use:tooltip={{ text: "Open Command Palette" }}
          data-tour-id="palette-launcher"
        >
          Palette
        </button>
      {/if}

      {#if showTutorialLauncher}
        <button
          class="toolbar-chip"
          on:click={() => startTutorial()}
          use:tooltip={{ text: "Start guided tutorial" }}
        >
          Tutorial
        </button>
      {/if}
    </div>
    {/if}

    {#if showQuickToolbar}
      <div data-tour-id="quick-toolbar">
        <QuickToolbar />
      </div>
    {/if}

    {#if showViewModeToggle}
      <button
        class="toolbox-mode-toggle"
        on:click={toggleViewMode}
        use:tooltip={{ text: "Toggle Canvas / Docking (Ctrl+Shift+C)" }}
        data-tour-id="view-mode-toggle"
      >
        {$viewMode === "docking" ? "⊞ Docking" : "◎ Canvas"}
      </button>
    {/if}

    <div class="toolbox-spacer"></div>

    {#if showWorkspaceActions}
      <WorkspaceControls bind:this={workspaceControls} />
    {/if}

    {#if showWidgetCount}
      <span class="widget-count"
        >{$totalWidgetCount} widget{$totalWidgetCount !== 1 ? "s" : ""}</span
      >
    {/if}

    {#if showResetButton}
      <button
        class="toolbox-reset"
        on:click={() => { resetDockingLayout(); clearCanvas(); }}
        use:tooltip={{ text: "Reset to default layout" }}
      >
        Reset Layout
      </button>
    {/if}
  </div>

  {#if placementActive && placementWidgetType}
    <div
      class="toolbox-placement-ghost"
      style="left: {placementClientX + 14}px; top: {placementClientY + 14}px;"
      aria-hidden="true"
    >
      Place {WIDGET_DEFINITIONS.find((def) => def.type === placementWidgetType)?.label ?? placementWidgetType}
    </div>
  {/if}

  {#if placementActive && dockPlacementPreview}
    <div
      class="docking-placement-preview"
      class:preview-left={dockPlacementPreview.zone === "left"}
      class:preview-right={dockPlacementPreview.zone === "right"}
      class:preview-top={dockPlacementPreview.zone === "top"}
      class:preview-bottom={dockPlacementPreview.zone === "bottom"}
      class:preview-center={dockPlacementPreview.zone === "center"}
      style="left: {dockPlacementPreview.left}px; top: {dockPlacementPreview.top}px; width: {dockPlacementPreview.width}px; height: {dockPlacementPreview.height}px;"
      aria-hidden="true"
    ></div>
  {/if}

  <!-- ─── Workspace Canvas ─── -->
  {#if $viewMode === "docking"}
    <div
      class="docking-canvas"
      on:dragover={handleDockingCanvasDragOver}
      on:dragleave={handleDockingCanvasDragLeave}
      on:drop={handleDockingCanvasDrop}
      role="region"
      aria-label="Docking workspace"
    >
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
  .toolbar-group {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    min-width: 0;
  }
  .toolbar-launchers,
  .toolbar-shortcuts {
    position: relative;
  }
  .toolbar-menu-anchor {
    position: relative;
    display: flex;
    align-items: center;
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
    left: 0;
    margin-top: 0.35rem;
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
  .toolbox-drag-hint {
    padding: 0.35rem 0.55rem;
    border-bottom: 1px solid var(--border);
    font-size: 0.66rem;
    color: var(--fg-secondary);
    background: color-mix(in srgb, var(--bg-inset) 70%, transparent);
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
    user-select: none;
    touch-action: none;
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
    right: 0;
    margin-top: 0.35rem;
    z-index: 101;
    width: min(24rem, calc(100vw - 1rem));
    background: var(--bg-primary);
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    padding: 0.45rem;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.32);
  }
  .customize-section {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .customize-heading {
    font-size: 0.62rem;
    color: var(--fg-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
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
  }
  .customize-options {
    display: flex;
    gap: 0.25rem;
    flex-wrap: wrap;
  }
  .customize-options button,
  .customize-keybinds,
  .customize-link {
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
    align-self: stretch;
    text-align: left;
  }
  .customize-links {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
    gap: 0.25rem;
  }
  .customize-link {
    text-align: left;
  }
  .customize-actions {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    border-top: 1px solid var(--border);
    padding-top: 0.35rem;
  }
  .toolbox-spacer {
    flex: 1;
  }
  .toolbar-chip {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--fg-secondary);
    padding: 0.2rem 0.5rem;
    font-size: 0.68rem;
    cursor: pointer;
    font-family: var(--font-mono);
  }
  .toolbar-chip:hover {
    border-color: var(--border-focus);
    color: var(--fg-primary);
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
  .toolbox-placement-ghost {
    position: fixed;
    z-index: 250;
    pointer-events: none;
    border: 1px solid var(--hl-symbol);
    background: color-mix(in srgb, var(--bg-primary) 92%, transparent);
    color: var(--fg-primary);
    font-family: var(--font-mono);
    font-size: 0.68rem;
    padding: 0.22rem 0.45rem;
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.28);
  }

  .docking-placement-preview {
    position: fixed;
    pointer-events: none;
    z-index: 240;
    border: 1px solid color-mix(in srgb, var(--hl-symbol) 55%, transparent);
    background: color-mix(in srgb, var(--hl-symbol) 12%, transparent);
  }

  .docking-placement-preview.preview-left {
    border-left: 4px solid var(--hl-symbol);
  }

  .docking-placement-preview.preview-right {
    border-right: 4px solid var(--hl-symbol);
  }

  .docking-placement-preview.preview-top {
    border-top: 4px solid var(--hl-symbol);
  }

  .docking-placement-preview.preview-bottom {
    border-bottom: 4px solid var(--hl-symbol);
  }

  .docking-placement-preview.preview-center {
    outline: 2px dashed var(--hl-symbol);
    outline-offset: -2px;
  }

  /* ── Docking Canvas ───────────────────────────────────────── */
  .docking-canvas {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    display: flex;
  }

</style>
