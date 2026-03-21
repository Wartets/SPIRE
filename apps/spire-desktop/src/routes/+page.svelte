<!--
  SPIRE Desktop - Adaptive Workbench Page

  Dual-mode workspace: recursive docking panels (default) or infinite
  whiteboard canvas.  The layout tree is driven by layoutStore.
  A compact Toolbox bar lets the user spawn new widgets and toggle
  between view modes.
-->
<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import {
    WIDGET_DEFINITIONS,
  } from "$lib/stores/notebookStore";
  import type { WidgetType } from "$lib/stores/notebookStore";
  import {
    layoutRoot,
    setLayoutRoot,
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
  import type { LayoutNode } from "$lib/stores/layoutStore";
  import { showContextMenu } from "$lib/stores/contextMenuStore";
  import { openPopup, openTextInputPopup } from "$lib/stores/popupStore";
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
  import {
    generateMathematicalProof,
    loadProvenanceState,
    validateSessionIntegrity,
  } from "$lib/api";
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
  let showQuickToolbar = false;
  let showWorkspaceActions = true;
  let showPaletteLauncher = false;
  let showTutorialLauncher = true;
  let showViewModeToggle = true;
  let showResetButton = true;
  let showToolbarShortcuts = true;
  let toolboxButtonEl: HTMLButtonElement | null = null;
  let toolboxMenuEl: HTMLDivElement | null = null;
  let customizerButtonEl: HTMLButtonElement | null = null;
  let customizerMenuEl: HTMLDivElement | null = null;
  let customizerMenuStyle = "";
  let placementPointerId: number | null = null;
  let placementWidgetType: WidgetType | null = null;
  let activeWorkspaceName = "Workspace";
  let placementStartX = 0;
  let placementStartY = 0;
  let placementClientX = 0;
  let placementClientY = 0;
  let placementActive = false;
  let suppressToolboxClickForType: WidgetType | null = null;
  let workspaceInfoOpen = false;
  let workspaceInfoButtonEl: HTMLButtonElement | null = null;
  let workspaceInfoMenuEl: HTMLDivElement | null = null;
  let activeWorkspaceDescription = "Untitled workspace";
  let activeWorkspaceColor = "#5eb8ff";

  type DockPreviewZone = "left" | "right" | "top" | "bottom" | "center";
  let dockPlacementPreview:
    | { left: number; top: number; width: number; height: number; zone: DockPreviewZone }
    | null = null;
  const PLACEMENT_DRAG_THRESHOLD = 8;
  const TOOLBOX_PREFS_KEY = "spire.toolbox.prefs.v4";
  const CANVAS_AUTO_GAP = 32;
  const CANVAS_SMART_GAP = 24;
  const CANVAS_GRID_SIZE = 24;

  function layoutHasWidgetType(node: LayoutNode, target: WidgetType): boolean {
    if (node.type === "widget") {
      return node.widgetType === target;
    }
    return node.children.some((child) => layoutHasWidgetType(child, target));
  }

  function ensureAnalysisWidgetForTutorial(): void {
    if ($viewMode === "canvas") {
      const existing = $canvasItems.find((item) => item.widgetType === "analysis");
      if (!existing) {
        addCanvasItem("analysis");
        return;
      }
      window.dispatchEvent(
        new CustomEvent("spire:canvas:focus-widget", {
          detail: { widgetId: existing.id },
        }),
      );
      return;
    }

    const hasAnalysis = layoutHasWidgetType($layoutRoot, "analysis");
    if (!hasAnalysis) {
      addWidgetToLayout("analysis", "auto");
    }
  }

  function handleTutorialEnsureAnalysisWidget(): void {
    ensureAnalysisWidgetForTutorial();
  }

  async function updateCustomizerMenuPosition(): Promise<void> {
    if (!customizerOpen || !customizerButtonEl || !customizerMenuEl) return;

    const margin = 8;
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const buttonRect = customizerButtonEl.getBoundingClientRect();

    const preferredWidth = Math.min(384, vw - margin * 2);
    customizerMenuStyle = `width: ${preferredWidth}px; max-width: ${vw - margin * 2}px;`;
    await tick();

    const menuW = Math.min(preferredWidth, customizerMenuEl.offsetWidth || preferredWidth);
    const menuH = customizerMenuEl.offsetHeight || 320;

    let left = buttonRect.right - menuW;
    left = Math.max(margin, Math.min(left, vw - menuW - margin));

    const spaceBelow = vh - buttonRect.bottom - margin;
    const spaceAbove = buttonRect.top - margin;
    const openAbove = spaceBelow < Math.min(menuH, 360) && spaceAbove > spaceBelow;

    const top = openAbove
      ? Math.max(margin, buttonRect.top - Math.min(menuH, vh - margin * 2) - 6)
      : Math.min(vh - margin - 40, buttonRect.bottom + 6);

    const maxHeight = openAbove
      ? Math.max(220, buttonRect.top - margin - 10)
      : Math.max(220, vh - top - margin);

    customizerMenuStyle = `left: ${left}px; top: ${top}px; width: ${preferredWidth}px; max-width: ${vw - margin * 2}px; max-height: ${maxHeight}px;`;
  }

  function arrangeCanvasNoOverlapFixedGap(gap = CANVAS_AUTO_GAP): void {
    canvasItems.update((items) => {
      if (items.length <= 1) return items;

      const ordered = [...items].sort((a, b) => a.y - b.y || a.x - b.x);
      const maxWidth = Math.max(...ordered.map((item) => Math.max(1, item.width)));
      const maxHeight = Math.max(...ordered.map((item) => Math.max(1, item.height)));
      const columns = Math.max(1, Math.ceil(Math.sqrt(ordered.length)));
      const startX = Math.min(...ordered.map((item) => item.x));
      const startY = Math.min(...ordered.map((item) => item.y));

      return ordered.map((item, index) => {
        const col = index % columns;
        const row = Math.floor(index / columns);
        return {
          ...item,
          x: startX + col * (maxWidth + gap),
          y: startY + row * (maxHeight + gap),
        };
      });
    });
  }

  function snapCanvasWidgetsToGrid(gridSize = CANVAS_GRID_SIZE): void {
    const step = Math.max(4, Math.floor(gridSize));
    canvasItems.update((items) =>
      items.map((item) => ({
        ...item,
        x: Math.round(item.x / step) * step,
        y: Math.round(item.y / step) * step,
      })),
    );
  }

  function arrangeCanvasSmartPack(gap = CANVAS_SMART_GAP): void {
    canvasItems.update((items) => {
      if (items.length <= 1) return items;

      const ordered = [...items].sort((a, b) => a.y - b.y || a.x - b.x);
      const startX = Math.min(...ordered.map((item) => item.x));
      const startY = Math.min(...ordered.map((item) => item.y));
      const totalArea = ordered.reduce(
        (acc, item) => acc + Math.max(1, item.width) * Math.max(1, item.height),
        0,
      );
      const widest = Math.max(...ordered.map((item) => Math.max(1, item.width)));
      const targetRowWidth = Math.max(widest * 2, Math.sqrt(totalArea) * 1.8);

      let cursorX = 0;
      let cursorY = 0;
      let rowHeight = 0;

      return ordered.map((item) => {
        const width = Math.max(1, item.width);
        const height = Math.max(1, item.height);
        const wraps = cursorX > 0 && cursorX + width > targetRowWidth;

        if (wraps) {
          cursorY += rowHeight + gap;
          cursorX = 0;
          rowHeight = 0;
        }

        const next = {
          ...item,
          x: startX + cursorX,
          y: startY + cursorY,
        };

        cursorX += width + gap;
        rowHeight = Math.max(rowHeight, height);
        return next;
      });
    });
  }

  function equalizeDockingPaneSizesRecursive(node: LayoutNode): LayoutNode {
    if (node.type === "widget") {
      return node;
    }

    if (node.type === "stack") {
      return {
        ...node,
        children: node.children.map((child) => equalizeDockingPaneSizesRecursive(child)),
      };
    }

    return {
      ...node,
      sizes: node.children.map(() => 1),
      children: node.children.map((child) => equalizeDockingPaneSizesRecursive(child)),
    };
  }

  function equalizeDockingPanes(): void {
    setLayoutRoot(equalizeDockingPaneSizesRecursive($layoutRoot));
  }

  function resetCustomizationDefaults(): void {
    widgetSortMode = "workflow";
    showWidgetCount = true;
    compactToolbar = false;
    showQuickToolbar = false;
    showWorkspaceActions = true;
    showPaletteLauncher = false;
    showTutorialLauncher = true;
    showViewModeToggle = true;
    showResetButton = true;
    showToolbarShortcuts = true;
  }

  function openToolbarContextMenu(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();

    showContextMenu(event.clientX, event.clientY, [
      {
        type: "submenu",
        id: "ctx-toolbar-layout",
        label: "Toolbar Layout",
        icon: "UI",
        children: [
          {
            type: "toggle",
            id: "ctx-toggle-compact",
            label: "Compact toolbar",
            checked: compactToolbar,
            action: (checked) => {
              compactToolbar = checked;
            },
          },
          {
            type: "toggle",
            id: "ctx-toggle-widget-count",
            label: "Show widget count",
            checked: showWidgetCount,
            action: (checked) => {
              showWidgetCount = checked;
            },
          },
          {
            type: "toggle",
            id: "ctx-toggle-quick-toolbar",
            label: "Show quick toolbar",
            checked: showQuickToolbar,
            action: (checked) => {
              showQuickToolbar = checked;
            },
          },
          {
            type: "toggle",
            id: "ctx-toggle-workspace-controls",
            label: "Show workspace controls",
            checked: showWorkspaceActions,
            action: (checked) => {
              showWorkspaceActions = checked;
            },
          },
        ],
      },
      {
        type: "submenu",
        id: "ctx-toolbar-helpers",
        label: "Helpers",
        icon: "?",
        children: [
          {
            type: "toggle",
            id: "ctx-toggle-shortcuts",
            label: "Show palette/tutorial chips",
            checked: showToolbarShortcuts,
            action: (checked) => {
              showToolbarShortcuts = checked;
            },
          },
          {
            type: "toggle",
            id: "ctx-toggle-palette",
            label: "Show palette launcher",
            checked: showPaletteLauncher,
            action: (checked) => {
              showPaletteLauncher = checked;
            },
          },
          {
            type: "toggle",
            id: "ctx-toggle-tutorial",
            label: "Show tutorial launcher",
            checked: showTutorialLauncher,
            action: (checked) => {
              showTutorialLauncher = checked;
            },
          },
          {
            type: "toggle",
            id: "ctx-toggle-view-mode",
            label: "Show view-mode toggle",
            checked: showViewModeToggle,
            action: (checked) => {
              showViewModeToggle = checked;
            },
          },
          {
            type: "toggle",
            id: "ctx-toggle-reset",
            label: "Show reset button",
            checked: showResetButton,
            action: (checked) => {
              showResetButton = checked;
            },
          },
        ],
      },
      {
        type: "submenu",
        id: "ctx-toolbar-docking",
        label: "Dock insertion",
        icon: "↔",
        children: [
          {
            type: "toggle",
            id: "ctx-dock-smart",
            label: "Smart",
            checked: $dockingInsertPreference === "smart",
            action: (checked) => {
              if (checked) dockingInsertPreference.set("smart");
            },
          },
          {
            type: "toggle",
            id: "ctx-dock-row",
            label: "Row",
            checked: $dockingInsertPreference === "row",
            action: (checked) => {
              if (checked) dockingInsertPreference.set("row");
            },
          },
          {
            type: "toggle",
            id: "ctx-dock-col",
            label: "Column",
            checked: $dockingInsertPreference === "col",
            action: (checked) => {
              if (checked) dockingInsertPreference.set("col");
            },
          },
        ],
      },
      {
        type: "submenu",
        id: "ctx-toolbar-layout-tools",
        label: "Layout Tools",
        icon: "layout",
        children: [
          {
            type: "action",
            id: "ctx-canvas-arrange-fixed-gap",
            label: `Canvas: Auto-arrange (gap ${CANVAS_AUTO_GAP}px)`,
            icon: "column",
            disabled: $viewMode !== "canvas",
            action: () => arrangeCanvasNoOverlapFixedGap(CANVAS_AUTO_GAP),
          },
          {
            type: "action",
            id: "ctx-canvas-arrange-smart",
            label: `Canvas: Smart pack (gap ${CANVAS_SMART_GAP}px)`,
            icon: "panels",
            disabled: $viewMode !== "canvas",
            action: () => arrangeCanvasSmartPack(CANVAS_SMART_GAP),
          },
          {
            type: "action",
            id: "ctx-canvas-snap-grid",
            label: `Canvas: Snap all to ${CANVAS_GRID_SIZE}px grid`,
            icon: "hash",
            disabled: $viewMode !== "canvas",
            action: () => snapCanvasWidgetsToGrid(CANVAS_GRID_SIZE),
          },
          { type: "separator", id: "ctx-layout-tools-sep" },
          {
            type: "action",
            id: "ctx-docking-equalize-panes",
            label: "Docking: Equalize all pane sizes",
            icon: "row",
            disabled: $viewMode !== "docking",
            action: () => equalizeDockingPanes(),
          },
        ],
      },
      { type: "separator", id: "ctx-toolbar-sep-1" },
      {
        type: "action",
        id: "ctx-toolbar-open-customizer",
        label: "Open Customize panel",
        icon: "CFG",
        action: () => {
          customizerOpen = true;
          toolboxOpen = false;
        },
      },
      {
        type: "action",
        id: "ctx-toolbar-reset-defaults",
        label: "Reset customization defaults",
        icon: "↺",
        action: () => {
          resetCustomizationDefaults();
        },
      },
    ]);
  }

  function toolbarContextMenu(node: HTMLElement): { destroy: () => void } {
    const handler = (event: MouseEvent) => openToolbarContextMenu(event);
    node.addEventListener("contextmenu", handler);
    return {
      destroy: () => node.removeEventListener("contextmenu", handler),
    };
  }

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

    const inWorkspaceInfo =
      workspaceInfoMenuEl?.contains(target) || workspaceInfoButtonEl?.contains(target);
    if (!inWorkspaceInfo) {
      workspaceInfoOpen = false;
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
      workspaceInfoOpen = false;
    }
  }

  function handleWindowResizeOrScroll(): void {
    if (customizerOpen) {
      void updateCustomizerMenuPosition();
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
      // Clipboard API may not be available; fall back to popup-managed input.
      const input = await openTextInputPopup({
        title: "Load Provenance Record",
        message: "Paste a valid SPIRE provenance JSON payload.",
        label: "Provenance JSON",
        placeholder: "{\n  \"version\": \"...\",\n  \"seed\": 42\n}",
        multiline: true,
        rows: 10,
        confirmLabel: "Load",
        cancelLabel: "Cancel",
        requireNonEmpty: true,
        maxWidth: 760,
      });
      if (!input) return;
      json = input;
    }
    if (!json.trim()) return;
    try {
      const integrity = await validateSessionIntegrity(json);
      if (!integrity.ok) {
        const choice = await openPopup({
          title: "Session Integrity Mismatch",
          tone: "danger",
          message:
            "The saved provenance lock does not match your local PDG environment. Choose a deterministic remediation path.",
          meta: [
            { label: "Saved edition", value: integrity.saved_edition },
            { label: "Local edition", value: integrity.local_edition },
            { label: "Saved fingerprint", value: integrity.saved_fingerprint ?? "missing" },
            { label: "Local fingerprint", value: integrity.local_fingerprint ?? "missing" },
            { label: "Reason", value: integrity.reason },
          ],
          actions: [
            { id: "override_local_pdg", label: "Override with local PDG", variant: "danger" },
            { id: "fallback_analytical", label: "Fallback to analytical mode", variant: "ghost" },
            { id: "abort_restore", label: "Abort restore", variant: "ghost", autofocus: true },
          ],
          closeActionId: "abort_restore",
        });

        if (choice === "abort_restore" || !choice) {
          return;
        }

        if (choice === "fallback_analytical") {
          await openPopup({
            title: "Analytical Fallback Selected",
            tone: "warn",
            message:
              "Restore aborted. Continue in analytical mode and re-import PDG data when ready.",
            actions: [{ id: "ok", label: "OK", variant: "primary", autofocus: true }],
            closeActionId: "ok",
          });
          return;
        }
      }

      const state = await loadProvenanceState(json);
      console.log("[SPIRE] Provenance state restored:", state);
      // The returned state contains model, reaction, kinematics, seed.
      // Future phases will dispatch these to the appropriate stores.
      await openPopup({
        title: "Provenance Loaded",
        tone: "success",
        message: "Provenance state restored successfully.",
        meta: [
          { label: "Seed", value: `${(state as Record<string, unknown>).seed ?? "unknown"}` },
          { label: "Version", value: `${(state as Record<string, unknown>).version ?? "unknown"}` },
        ],
        actions: [{ id: "ok", label: "OK", variant: "primary", autofocus: true }],
        closeActionId: "ok",
      });
    } catch (err) {
      console.error("[SPIRE] Provenance load failed:", err);
      await openPopup({
        title: "Provenance Load Failed",
        tone: "danger",
        message: `${err}`,
        actions: [{ id: "ok", label: "OK", variant: "danger", autofocus: true }],
        closeActionId: "ok",
      });
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

  $: {
    const activeWorkspaceData =
      $workspaces.find((item) => item.id === $activeWorkspaceId) ?? null;
    activeWorkspaceName = activeWorkspaceData?.name ?? "Workspace";
    activeWorkspaceDescription = activeWorkspaceData?.description ?? "Untitled workspace";
    activeWorkspaceColor = activeWorkspaceData?.color ?? "#5eb8ff";
  }

  $: if (typeof document !== "undefined") {
    document.title = `SPIRE - ${activeWorkspaceName}`;
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
    "spire.canvas.toggle_automation",
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
      id: "spire.canvas.toggle_automation",
      title: "Toggle Canvas Automation",
      category: "Canvas",
      execute: () => {
        window.dispatchEvent(new CustomEvent("spire:canvas:toggle-automation"));
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
      execute: async () => {
        const name = await openTextInputPopup({
          title: "Save Layout Preset",
          message: "Choose a name for the current layout preset.",
          label: "Preset name",
          value: "My Preset",
          confirmLabel: "Save",
          cancelLabel: "Cancel",
          requireNonEmpty: true,
          maxWidth: 560,
        });
        if (!name) return;
        saveCurrentLayoutPreset(name.trim());
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
    window.addEventListener("resize", handleWindowResizeOrScroll);
    window.addEventListener("scroll", handleWindowResizeOrScroll, true);
    window.addEventListener("spire:tutorial:ensure-analysis-widget", handleTutorialEnsureAnalysisWidget as EventListener);

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
    window.removeEventListener("resize", handleWindowResizeOrScroll);
    window.removeEventListener("scroll", handleWindowResizeOrScroll, true);
    window.removeEventListener("spire:tutorial:ensure-analysis-widget", handleTutorialEnsureAnalysisWidget as EventListener);
    unsubIdentityModel?.();
    unsubIdentityFramework?.();
  });

  $: if (customizerOpen) {
    void tick().then(() => updateCustomizerMenuPosition());
  }
</script>

<div class="workbench">
  <!-- ─── Workspace Tab Bar ─── -->
  <div data-tour-id="workspace-tabs">
    <WorkspaceTabs />
  </div>

  <!-- ─── Toolbox Bar ─── -->
  <div class="toolbox-bar" class:toolbox-compact={compactToolbar} use:toolbarContextMenu role="toolbar" tabindex="-1" aria-label="Workbench toolbar">
    <div class="toolbar-group toolbar-workspace-info">
      <div class="toolbar-menu-anchor">
        <button
          class="toolbar-info-btn"
          bind:this={workspaceInfoButtonEl}
          on:click={() => {
            workspaceInfoOpen = !workspaceInfoOpen;
            if (workspaceInfoOpen) {
              toolboxOpen = false;
              customizerOpen = false;
            }
          }}
          use:tooltip={{ text: "Workspace info" }}
          aria-expanded={workspaceInfoOpen}
          aria-label="Workspace information"
        >
          {compactToolbar ? "ⓘ" : "Info"}
        </button>

        {#if workspaceInfoOpen}
          <div class="workspace-info-menu" bind:this={workspaceInfoMenuEl}>
            <div class="workspace-info-row">
              <span class="workspace-info-label">Name</span>
              <strong>{activeWorkspaceName}</strong>
            </div>
            <div class="workspace-info-row">
              <span class="workspace-info-label">Description</span>
              <span>{activeWorkspaceDescription}</span>
            </div>
            <div class="workspace-info-row">
              <span class="workspace-info-label">Color</span>
              <span class="workspace-info-color" style={`--ws-color: ${activeWorkspaceColor};`}>
                {activeWorkspaceColor}
              </span>
            </div>
          </div>
        {/if}
      </div>
    </div>

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
          {compactToolbar ? "+ Widget" : "+ Add Widget"}
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
          {compactToolbar ? "⚙" : "Customize ▾"}
        </button>

        {#if customizerOpen}
          <div class="customize-menu" bind:this={customizerMenuEl} style={customizerMenuStyle}>
            <div class="customize-section">
              <span class="customize-heading">Toolbar Layout</span>
              <label class="customize-toggle">
                <span>Compact toolbar</span>
                <input type="checkbox" bind:checked={compactToolbar} />
              </label>
              <label class="customize-toggle">
                <span>Show widget count</span>
                <input type="checkbox" bind:checked={showWidgetCount} />
              </label>
              <label class="customize-toggle">
                <span>Show quick toolbar</span>
                <input type="checkbox" bind:checked={showQuickToolbar} />
              </label>
              <label class="customize-toggle">
                <span>Show workspace controls</span>
                <input type="checkbox" bind:checked={showWorkspaceActions} />
              </label>
            </div>

            <div class="customize-section">
              <span class="customize-heading">Toolbar Actions</span>
              <label class="customize-toggle">
                <span>Show helper chips</span>
                <input type="checkbox" bind:checked={showToolbarShortcuts} />
              </label>
              <label class="customize-toggle">
                <span>Show command palette chip</span>
                <input type="checkbox" bind:checked={showPaletteLauncher} />
              </label>
              <label class="customize-toggle">
                <span>Show tutorial chip</span>
                <input type="checkbox" bind:checked={showTutorialLauncher} />
              </label>
              <label class="customize-toggle">
                <span>Show view-mode toggle</span>
                <input type="checkbox" bind:checked={showViewModeToggle} />
              </label>
              <label class="customize-toggle">
                <span>Show reset button</span>
                <input type="checkbox" bind:checked={showResetButton} />
              </label>
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
          {compactToolbar ? "Cmd" : "Palette"}
        </button>
      {/if}

      {#if showTutorialLauncher}
        <button
          class="toolbar-chip"
          on:click={() => startTutorial()}
          use:tooltip={{ text: "Start guided tutorial" }}
        >
          {compactToolbar ? "Guide" : "Tutorial"}
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
        {compactToolbar
          ? ($viewMode === "docking" ? "⊞ Dock" : "◎ Canvas")
          : ($viewMode === "docking" ? "⊞ Docking" : "◎ Canvas")}
      </button>
    {/if}

    <div class="toolbox-spacer"></div>

    {#if showWorkspaceActions}
      <WorkspaceControls bind:this={workspaceControls} />
    {/if}

    {#if showWidgetCount}
      <span class="widget-count"
        >{compactToolbar
          ? `${$totalWidgetCount}`
          : `${$totalWidgetCount} widget${$totalWidgetCount !== 1 ? "s" : ""}`}</span
      >
    {/if}

    {#if showResetButton}
      <button
        class="toolbox-reset"
        on:click={() => { resetDockingLayout(); clearCanvas(); }}
        use:tooltip={{ text: "Reset to default layout" }}
      >
        {compactToolbar ? "Reset" : "Reset Layout"}
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
    gap: 0.18rem;
    padding: 0.12rem 0.24rem;
  }
  .toolbox-bar.toolbox-compact .toolbar-info-btn {
    font-size: 0.62rem;
    line-height: 1;
    padding: 0.14rem 0.34rem;
  }
  .toolbox-bar.toolbox-compact .toolbar-group {
    gap: 0.2rem;
  }
  .toolbox-bar.toolbox-compact .toolbox-toggle,
  .toolbox-bar.toolbox-compact .toolbox-customize,
  .toolbox-bar.toolbox-compact .toolbar-chip,
  .toolbox-bar.toolbox-compact .toolbox-mode-toggle,
  .toolbox-bar.toolbox-compact .toolbox-reset {
    font-size: 0.62rem;
    line-height: 1;
    padding: 0.14rem 0.34rem;
    letter-spacing: 0.02em;
  }
  .toolbox-bar.toolbox-compact .widget-count {
    font-size: 0.6rem;
    opacity: 0.85;
  }
  .toolbar-group {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    min-width: 0;
  }
  .toolbar-workspace-info {
    padding-right: 0.3rem;
    border-right: 1px solid var(--border);
    margin-right: 0.15rem;
  }
  .toolbar-info-btn {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--fg-secondary);
    padding: 0.2rem 0.55rem;
    font-size: 0.68rem;
    cursor: pointer;
    font-family: var(--font-mono);
    font-weight: 600;
  }
  .toolbar-info-btn:hover {
    border-color: var(--border-focus);
    color: var(--fg-primary);
  }
  .workspace-info-menu {
    position: absolute;
    top: 100%;
    left: 0;
    margin-top: 0.35rem;
    min-width: 14rem;
    max-width: min(22rem, calc(100vw - 1rem));
    background: var(--bg-primary);
    border: 1px solid var(--border);
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.28);
    z-index: 102;
    padding: 0.45rem 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .workspace-info-row {
    display: flex;
    flex-direction: column;
    gap: 0.12rem;
    font-size: 0.72rem;
    color: var(--fg-primary);
  }
  .workspace-info-label {
    font-size: 0.58rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--fg-secondary);
  }
  .workspace-info-color {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    font-family: var(--font-mono);
  }
  .workspace-info-color::before {
    content: "";
    width: 0.6rem;
    height: 0.6rem;
    border-radius: 50%;
    background: var(--ws-color);
    border: 1px solid color-mix(in srgb, var(--ws-color) 75%, #fff 25%);
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
    position: fixed;
    z-index: 101;
    width: min(24rem, calc(100vw - 1rem));
    max-width: calc(100vw - 0.75rem);
    max-height: calc(100vh - 4.75rem);
    background: var(--bg-primary);
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    padding: 0.45rem;
    overflow-y: auto;
    overflow-x: hidden;
    overscroll-behavior: contain;
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
    justify-content: space-between;
    gap: 0.6rem;
    font-size: 0.7rem;
    color: var(--fg-primary);
    border: 1px solid color-mix(in srgb, var(--border) 85%, transparent);
    background: color-mix(in srgb, var(--bg-surface) 88%, transparent);
    padding: 0.24rem 0.42rem;
  }
  .customize-toggle > span {
    min-width: 0;
  }
  .customize-toggle input {
    appearance: none;
    position: relative;
    width: 1.9rem;
    height: 1rem;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: color-mix(in srgb, var(--bg-primary) 88%, black 12%);
    cursor: pointer;
    flex-shrink: 0;
    transition: border-color 0.15s ease, background 0.15s ease;
  }
  .customize-toggle input::after {
    content: "";
    position: absolute;
    top: 0.08rem;
    left: 0.1rem;
    width: 0.72rem;
    height: 0.72rem;
    border-radius: 50%;
    background: var(--fg-secondary);
    transition: transform 0.15s ease, background 0.15s ease;
  }
  .customize-toggle input:checked {
    border-color: color-mix(in srgb, var(--hl-symbol) 60%, var(--border));
    background: color-mix(in srgb, var(--hl-symbol) 28%, var(--bg-surface));
  }
  .customize-toggle input:checked::after {
    transform: translateX(0.86rem);
    background: color-mix(in srgb, var(--hl-symbol) 30%, var(--fg-primary));
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
    align-self: stretch;
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

  .docking-canvas > :global(*) {
    flex: 1 1 auto;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
  }

</style>
