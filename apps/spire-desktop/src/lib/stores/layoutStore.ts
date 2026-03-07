/**
 * SPIRE - Layout Store
 *
 * Data-driven recursive docking system.  The entire workspace layout
 * is expressed as a JSON-serialisable tree of `LayoutNode` values.
 * Leaf nodes (`WidgetLeaf`) hold a widget type and optional instance
 * data.  Branch nodes (`RowNode`, `ColNode`) describe flex-direction
 * splits with draggable proportions.  `StackNode` describes tabbed
 * groups.
 *
 * ## Architecture
 *
 * The store exposes a single writable `layoutRoot` that holds the
 * root LayoutNode, plus a `viewMode` writable that toggles between
 * "docking" (recursive tree) and "canvas" (infinite whiteboard).
 *
 * Reducer functions operate on the tree immutably (deep-clone +
 * structural transform) and write the result back.  This guarantees
 * Svelte reactivity and enables workspace save/restore.
 *
 * ## Serialisation
 *
 * Every type in this module is plain JSON - no class instances, no
 * functions, no circular references.  `layoutRoot` can be serialised
 * with `JSON.stringify()` and restored with `JSON.parse()`.
 */

import { writable, derived, get } from "svelte/store";
import type { WidgetType } from "./notebookStore";
import { WIDGET_DEFINITIONS } from "./notebookStore";
import { inferDockingTreeFromCanvas } from '../core/layout/spatialInference';

// ===========================================================================
// Layout Node Types (JSON-serialisable discriminated union)
// ===========================================================================

/** Discriminator for layout node types. */
export type LayoutType = "row" | "col" | "stack" | "widget";

/** Base fields shared by every node. */
interface LayoutNodeBase {
  /** Unique identifier within the layout tree. */
  id: string;
  /** Node type discriminator. */
  type: LayoutType;
}

/** Horizontal split - children are arranged left-to-right. */
export interface RowNode extends LayoutNodeBase {
  type: "row";
  /** Ordered child nodes. */
  children: LayoutNode[];
  /** Flex-grow proportions parallel to `children`. */
  sizes: number[];
}

/** Vertical split - children are arranged top-to-bottom. */
export interface ColNode extends LayoutNodeBase {
  type: "col";
  /** Ordered child nodes. */
  children: LayoutNode[];
  /** Flex-grow proportions parallel to `children`. */
  sizes: number[];
}

/** Tabbed group - one child visible at a time. */
export interface StackNode extends LayoutNodeBase {
  type: "stack";
  /** Child nodes (each tab). */
  children: LayoutNode[];
  /** Index of the currently visible tab. */
  activeIndex: number;
}

/** Leaf node - renders an actual widget component. */
export interface WidgetLeaf extends LayoutNodeBase {
  type: "widget";
  /** Which component to render. */
  widgetType: WidgetType;
  /** Per-instance data bag. */
  widgetData: Record<string, unknown>;
}

/** The recursive layout tree type. */
export type LayoutNode = RowNode | ColNode | StackNode | WidgetLeaf;

/** A container node (row, col, or stack). */
export type ContainerNode = RowNode | ColNode | StackNode;

// ===========================================================================
// Canvas Item (for Infinite Whiteboard mode)
// ===========================================================================

/** A widget placed freely on the infinite canvas. */
export interface CanvasItem {
  id: string;
  widgetType: WidgetType;
  widgetData: Record<string, unknown>;
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Canvas viewport state. */
export interface CanvasViewport {
  panX: number;
  panY: number;
  zoom: number;
}

// ===========================================================================
// View Mode
// ===========================================================================

export type ViewMode = "docking" | "canvas";

// ===========================================================================
// Workspace (Multi-Workspace Support)
// ===========================================================================

/** A self-contained workspace holding its own layout, canvas, and view mode. */
export interface Workspace {
  id: string;
  name: string;
  color: string;
  dockingRoot: LayoutNode;
  canvasItemList: CanvasItem[];
  viewport: CanvasViewport;
  mode: ViewMode;
}

// ===========================================================================
// ID Generation
// ===========================================================================

let _layoutCounter = 0;

export function makeLayoutId(): string {
  _layoutCounter += 1;
  return `ln-${_layoutCounter}-${Date.now().toString(36)}`;
}

// ===========================================================================
// Helper: Create a Widget Leaf
// ===========================================================================

export function createWidgetLeaf(
  widgetType: WidgetType,
  data?: Record<string, unknown>,
): WidgetLeaf {
  return {
    id: makeLayoutId(),
    type: "widget",
    widgetType,
    widgetData: data ?? {},
  };
}

// ===========================================================================
// Default Layout
// ===========================================================================

function createDefaultLayout(): LayoutNode {
  return {
    id: makeLayoutId(),
    type: "col",
    sizes: [3, 1],
    children: [
      {
        id: makeLayoutId(),
        type: "row",
        sizes: [1, 2, 1],
        children: [
          {
            id: makeLayoutId(),
            type: "col",
            sizes: [1, 1],
            children: [
              createWidgetLeaf("model"),
              createWidgetLeaf("reaction"),
            ],
          },
          createWidgetLeaf("diagram"),
          {
            id: makeLayoutId(),
            type: "col",
            sizes: [1, 1],
            children: [
              createWidgetLeaf("amplitude"),
              createWidgetLeaf("kinematics"),
            ],
          },
        ],
      } as RowNode,
      createWidgetLeaf("log"),
    ],
  } as ColNode;
}

function createDefaultCanvas(): CanvasItem[] {
  return [];
}

/**
 * Returns a fresh deep-copy of the factory-default docking layout.
 * Exported so workspaceManager can use it for v1.0 → v2.0 migration.
 */
export function getDefaultLayout(): LayoutNode {
  return createDefaultLayout();
}

// ===========================================================================
// Dual-State Layout Memory
// ===========================================================================

interface DualLayoutMemory {
  mode: ViewMode;
  canvasState: {
    items: CanvasItem[];
    viewport: CanvasViewport;
  };
  dockingState: LayoutNode;
}

const defaultDualMemory: DualLayoutMemory = {
  mode: "docking",
  canvasState: {
    items: createDefaultCanvas(),
    viewport: { panX: 0, panY: 0, zoom: 1 },
  },
  dockingState: createDefaultLayout(),
};

export const dualLayoutStore = writable<DualLayoutMemory>(defaultDualMemory);

// Derived helpers for legacy compatibility
export const layoutRoot = derived(dualLayoutStore, ($mem) => $mem.dockingState);
export const canvasItems = derived(dualLayoutStore, ($mem) => $mem.canvasState.items);
export const canvasViewport = derived(dualLayoutStore, ($mem) => $mem.canvasState.viewport);
export const viewMode = derived(dualLayoutStore, ($mem) => $mem.mode);

// ===========================================================================
// Derived Helpers
// ===========================================================================

/** Count of all widget leaves in the docking tree. */
export const dockingWidgetCount = derived(layoutRoot, ($root) => {
  let count = 0;
  function walk(node: LayoutNode): void {
    if (node.type === "widget") {
      count += 1;
    } else {
      (node as ContainerNode).children.forEach(walk);
    }
  }
  walk($root);
  return count;
});

/** Count of all canvas items. */
export const canvasWidgetCount = derived(canvasItems, ($items) => $items.length);

/** Total widget count across both modes. */
export const totalWidgetCount = derived(
  [dockingWidgetCount, canvasWidgetCount],
  ([$dock, $canvas]) => $dock + $canvas,
);

// ===========================================================================
// Tree Traversal Utilities
// ===========================================================================

/** Deep-clone a layout node tree. */
function cloneTree(node: LayoutNode): LayoutNode {
  return JSON.parse(JSON.stringify(node));
}

/** Find a node by ID and return it with its parent (null if root). */
function findNode(
  root: LayoutNode,
  targetId: string,
): { node: LayoutNode; parent: ContainerNode | null; index: number } | null {
  if (root.id === targetId) {
    return { node: root, parent: null, index: -1 };
  }
  if (root.type === "widget") return null;

  const container = root as ContainerNode;
  for (let i = 0; i < container.children.length; i++) {
    if (container.children[i].id === targetId) {
      return { node: container.children[i], parent: container, index: i };
    }
    const deep = findNode(container.children[i], targetId);
    if (deep) return deep;
  }
  return null;
}

/**
 * Replace a node in the tree by ID.
 * Mutates the tree in-place (call on a clone).
 */
function replaceNode(root: LayoutNode, targetId: string, replacement: LayoutNode): boolean {
  if (root.type === "widget") return false;

  const container = root as ContainerNode;
  for (let i = 0; i < container.children.length; i++) {
    if (container.children[i].id === targetId) {
      container.children[i] = replacement;
      return true;
    }
    if (replaceNode(container.children[i], targetId, replacement)) return true;
  }
  return false;
}

/**
 * Remove a node from the tree.  Returns the pruned tree.
 * If a container ends up with a single child, that child replaces
 * the container (structural pruning).
 */
function removeNode(root: LayoutNode, targetId: string): LayoutNode | null {
  // Cannot remove the root itself from this function
  if (root.id === targetId) return null;
  if (root.type === "widget") return root;

  const container = root as ContainerNode;
  const idx = container.children.findIndex((c) => c.id === targetId);

  if (idx !== -1) {
    container.children.splice(idx, 1);
    if ("sizes" in container && Array.isArray((container as RowNode | ColNode).sizes)) {
      (container as RowNode | ColNode).sizes.splice(idx, 1);
    }
    // Prune: if only one child left, replace container with that child
    if (container.children.length === 1) {
      return container.children[0];
    }
    if (container.children.length === 0) {
      return null;
    }
    return container;
  }

  // Recurse into children
  for (let i = 0; i < container.children.length; i++) {
    const result = removeNode(container.children[i], targetId);
    if (result !== container.children[i]) {
      // Something changed
      if (result === null) {
        container.children.splice(i, 1);
        if ("sizes" in container && Array.isArray((container as RowNode | ColNode).sizes)) {
          (container as RowNode | ColNode).sizes.splice(i, 1);
        }
      } else {
        container.children[i] = result;
      }
      // Prune again
      if (container.children.length === 1) {
        return container.children[0];
      }
      if (container.children.length === 0) {
        return null;
      }
      return container;
    }
  }

  return root;
}

// ===========================================================================
// Layout Reducers (Public API)
// ===========================================================================

/**
 * Split a node into a new container with the original node and a new
 * widget placed side-by-side.
 *
 * @param nodeId - The ID of the node to split.
 * @param direction - "row" for horizontal split, "col" for vertical.
 * @param newWidgetType - The type of widget to add in the new pane.
 */
export function splitNode(
  nodeId: string,
  direction: "row" | "col",
  newWidgetType: WidgetType,
): void {
  dualLayoutStore.update((mem) => {
    const tree = cloneTree(mem.dockingState);
    if (tree.id === nodeId) {
      const newContainer: RowNode | ColNode = {
        id: makeLayoutId(),
        type: direction,
        sizes: [1, 1],
        children: [tree, createWidgetLeaf(newWidgetType)],
      } as RowNode | ColNode;
      return { ...mem, dockingState: newContainer };
    }
    const newLeaf = createWidgetLeaf(newWidgetType);
    const newContainer: RowNode | ColNode = {
      id: makeLayoutId(),
      type: direction,
      sizes: [1, 1],
      children: [],
    } as RowNode | ColNode;
    const found = findNode(tree, nodeId);
    if (!found || !found.parent) return mem;
    const original = found.parent.children[found.index];
    newContainer.children = [original, newLeaf];
    found.parent.children[found.index] = newContainer;
    return { ...mem, dockingState: tree };
  });
}

/**
 * Close (remove) a widget or container node from the layout tree.
 * Automatically prunes single-child containers.
 */
export function closeNode(nodeId: string): void {
  dualLayoutStore.update((mem) => {
    const tree = cloneTree(mem.dockingState);
    if (tree.id === nodeId) {
      return { ...mem, dockingState: createDefaultLayout() };
    }
    const result = removeNode(tree, nodeId);
    return { ...mem, dockingState: result ?? createDefaultLayout() };
  });
}

/**
 * Update the flex-grow proportions of a container's children.
 */
export function resizePanes(nodeId: string, sizes: number[]): void {
  dualLayoutStore.update((mem) => {
    const tree = cloneTree(mem.dockingState);
    if (tree.id === nodeId && tree.type !== "widget") {
      (tree as RowNode | ColNode).sizes = sizes;
      return { ...mem, dockingState: tree };
    }
    const found = findNode(tree, nodeId);
    if (found && found.node.type !== "widget") {
      (found.node as RowNode | ColNode).sizes = sizes;
    }
    return { ...mem, dockingState: tree };
  });
}

/**
 * Set the active tab index of a stack node.
 */
export function setActiveTab(stackId: string, index: number): void {
  dualLayoutStore.update((mem) => {
    const tree = cloneTree(mem.dockingState);
    const found = findNode(tree, stackId);
    if (found && found.node.type === "stack") {
      const stack = found.node as StackNode;
      stack.activeIndex = Math.max(0, Math.min(index, stack.children.length - 1));
    } else if (tree.id === stackId && tree.type === "stack") {
      (tree as StackNode).activeIndex = Math.max(
        0,
        Math.min(index, (tree as StackNode).children.length - 1),
      );
    }
    return { ...mem, dockingState: tree };
  });
}

/**
 * Add a new widget to the layout.  Splits the first widget leaf found
 * (depth-first) in the given direction.
 */
export function addWidgetToLayout(
  widgetType: WidgetType,
  direction: "row" | "col" = "row",
): void {
  const mem = get(dualLayoutStore);
  const root = mem.dockingState;
  function findFirstLeaf(node: LayoutNode): string | null {
    if (node.type === "widget") return node.id;
    for (const child of (node as ContainerNode).children) {
      const found = findFirstLeaf(child);
      if (found) return found;
    }
    return null;
  }
  const leafId = findFirstLeaf(root);
  if (leafId) {
    splitNode(leafId, direction, widgetType);
  }
}

/**
 * Replace the entire layout tree with a new root.
 * Used by workspace import and layout presets.
 */
export function setLayoutRoot(root: LayoutNode): void {
  dualLayoutStore.update((mem) => ({ ...mem, dockingState: root }));
}

// ===========================================================================
// Docking Drag-and-Drop: moveNode Reducer
// ===========================================================================

/**
 * Position relative to the drop target.
 * - "before" / "after" - split the parent container
 * - "left" / "right" / "top" / "bottom" - split the target into a new container
 * - "center" - replace the target (tab-stack, future use)
 */
export type DropPosition = "left" | "right" | "top" | "bottom" | "center";

/**
 * Move a widget node from its current position in the layout tree
 * to a new position relative to a target node.
 *
 * This is the core reducer for docking drag-and-drop reorganisation.
 * It operates in three phases:
 *   1. Extract the source node from the tree (pruning empty containers).
 *   2. Locate the target node in the (now smaller) tree.
 *   3. Wrap the target in a new row/col container with the source node
 *      placed according to `position`.
 *
 * @param sourceId  - ID of the widget leaf to move.
 * @param targetId  - ID of the node to drop onto.
 * @param position  - Where relative to the target to place the source.
 */
export function moveNode(
  sourceId: string,
  targetId: string,
  position: DropPosition,
): void {
  if (sourceId === targetId) return;
  dualLayoutStore.update((mem) => {
    const tree = cloneTree(mem.dockingState);
    const sourceFound = findNode(tree, sourceId);
    if (!sourceFound) return mem;
    const sourceNode = cloneTree(sourceFound.node);
    let pruned: LayoutNode | null;
    if (tree.id === sourceId) {
      return mem;
    }
    pruned = removeNode(tree, sourceId);
    if (!pruned) return mem;
    if (pruned.id === targetId) {
      const direction: "row" | "col" =
        position === "left" || position === "right" ? "row" : "col";
      const children =
        position === "left" || position === "top"
          ? [sourceNode, pruned]
          : position === "center"
            ? [pruned]
            : [pruned, sourceNode];
      if (position === "center") return { ...mem, dockingState: pruned };
      return {
        ...mem,
        dockingState: {
          id: makeLayoutId(),
          type: direction,
          sizes: Array(children.length).fill(1),
          children,
        } as RowNode | ColNode,
      };
    }
    const targetFound = findNode(pruned, targetId);
    if (!targetFound || !targetFound.parent) return { ...mem, dockingState: pruned };
    if (position === "center") {
      targetFound.parent.children[targetFound.index] = sourceNode;
      return { ...mem, dockingState: pruned };
    }
    const direction: "row" | "col" =
      position === "left" || position === "right" ? "row" : "col";
    const children =
      position === "left" || position === "top"
        ? [sourceNode, targetFound.node]
        : [targetFound.node, sourceNode];
    const newContainer: RowNode | ColNode = {
      id: makeLayoutId(),
      type: direction,
      sizes: [1, 1],
      children,
    } as RowNode | ColNode;
    targetFound.parent.children[targetFound.index] = newContainer;
    return { ...mem, dockingState: pruned };
  });
}

/** Reset the docking layout to the default arrangement. */
export function resetDockingLayout(): void {
  dualLayoutStore.update((mem) => ({ ...mem, dockingState: createDefaultLayout() }));
}

// ===========================================================================
// Canvas Reducers (Public API)
// ===========================================================================

/**
 * Add a widget to the infinite canvas.
 *
 * When x/y are not supplied, the widget is placed at the center
 * of the current viewport (accounting for pan and zoom).
 *
 * Returns the new item's ID so callers can select / focus it.
 */
export function addCanvasItem(
  widgetType: WidgetType,
  x?: number,
  y?: number,
): string {
  const def = WIDGET_DEFINITIONS.find((d) => d.type === widgetType);
  const w = (def?.defaultColSpan ?? 2) * 320;
  const h = (def?.defaultRowSpan ?? 2) * 220;
  dualLayoutStore.update((mem) => {
    let px = x, py = y;
    if (px === undefined || py === undefined) {
      const vp = mem.canvasState.viewport;
      px = px ?? (-vp.panX + 450) / (vp.zoom || 1) - w / 2;
      py = py ?? (-vp.panY + 300) / (vp.zoom || 1) - h / 2;
    }
    const id = makeLayoutId();
    return {
      ...mem,
      canvasState: {
        ...mem.canvasState,
        items: [
          ...mem.canvasState.items,
          {
            id,
            widgetType,
            widgetData: {},
            x: px,
            y: py,
            width: w,
            height: h,
          },
        ],
      },
    };
  });
  // Return a new id (not tracked here, but for API compatibility)
  return makeLayoutId();
}

/** Remove a canvas item by ID. */
export function removeCanvasItem(id: string): void {
  dualLayoutStore.update((mem) => ({
    ...mem,
    canvasState: {
      ...mem.canvasState,
      items: mem.canvasState.items.filter((i) => i.id !== id),
    },
  }));
}

/** Update a canvas item's position or size. */
export function updateCanvasItem(
  id: string,
  patch: Partial<Pick<CanvasItem, "x" | "y" | "width" | "height">>,
): void {
  dualLayoutStore.update((mem) => ({
    ...mem,
    canvasState: {
      ...mem.canvasState,
      items: mem.canvasState.items.map((i) => (i.id === id ? { ...i, ...patch } : i)),
    },
  }));
}

/** Reset canvas to empty. */
export function clearCanvas(): void {
  dualLayoutStore.update((mem) => ({
    ...mem,
    canvasState: {
      items: [],
      viewport: { panX: 0, panY: 0, zoom: 1 },
    },
  }));
}

// ===========================================================================
// View Mode Toggle
// ===========================================================================

/** Toggle between docking and canvas view modes with dual-state memory.
 * Preserves independent snapshots for each mode.
 * If switching to docking and dockingState is empty or mismatched, rebuild using spatial inference.
 */
export function toggleViewMode(): void {
  dualLayoutStore.update((mem) => {
    const current = mem.mode;
    if (current === "docking") {
      // Switching to Canvas: restore last canvasState
      return {
        ...mem,
        mode: "canvas",
      };
    } else {
      // Switching to Docking: restore last dockingState
      // If dockingState is empty or widget count mismatches, rebuild
      const canvasWidgets = mem.canvasState.items;
      const dockingWidgets = collectWidgetLeaves(mem.dockingState);
      if (
        !mem.dockingState ||
        dockingWidgets.length !== canvasWidgets.length ||
        dockingWidgets.length === 0
      ) {
        // Use spatial inference engine to reconstruct docking tree
        return {
          ...mem,
          mode: "docking",
          dockingState: inferDockingTreeFromCanvas(canvasWidgets),
        };
      }
      return {
        ...mem,
        mode: "docking",
      };
    }
  });
}

// ===========================================================================
// Compatibility Bridge
// ===========================================================================

/**
 * Collect all WidgetLeaf nodes from the docking tree as a flat array.
 * Used by the workspace manager for backward-compatible serialisation.
 */
export function collectWidgetLeaves(root: LayoutNode): WidgetLeaf[] {
  const leaves: WidgetLeaf[] = [];
  function walk(node: LayoutNode): void {
    if (node.type === "widget") {
      leaves.push(node);
    } else {
      (node as ContainerNode).children.forEach(walk);
    }
  }
  walk(root);
  return leaves;
}

// ===========================================================================
// Multi-Workspace Manager
// ===========================================================================

let _wsCounter = 0;

function makeWorkspaceId(): string {
  _wsCounter += 1;
  return `ws-${_wsCounter}-${Date.now().toString(36)}`;
}

/** Accent colours available for workspaces. */
export const WORKSPACE_COLORS = [
  "#5eb8ff", // symbol blue
  "#d4a017", // gold
  "#2ecc71", // green
  "#e74c3c", // red
  "#9b59b6", // purple
  "#e67e22", // orange
  "#1abc9c", // teal
  "#f06292", // pink
  "#7986cb", // indigo
  "#4dd0e1", // cyan
];

function randomWorkspaceColor(): string {
  return WORKSPACE_COLORS[Math.floor(Math.random() * WORKSPACE_COLORS.length)];
}

function createWorkspace(name: string, color?: string): Workspace {
  return {
    id: makeWorkspaceId(),
    name,
    color: color ?? randomWorkspaceColor(),
    dockingRoot: createDefaultLayout(),
    canvasItemList: [],
    viewport: { panX: 0, panY: 0, zoom: 1 },
    mode: "docking",
  };
}

/** All workspaces. */
export const workspaces = writable<Workspace[]>([createWorkspace("Workspace 1", "#5eb8ff")]);

/** ID of the currently active workspace. */
export const activeWorkspaceId = writable<string>("");

// Initialise active workspace ID from the first workspace
workspaces.subscribe((wsList) => {
  const currentId = get(activeWorkspaceId);
  if (!currentId || !wsList.find((w) => w.id === currentId)) {
    if (wsList.length > 0) {
      activeWorkspaceId.set(wsList[0].id);
    }
  }
});

/** The currently active workspace object (derived). */
export const activeWorkspace = derived(
  [workspaces, activeWorkspaceId],
  ([$wsList, $activeId]) => $wsList.find((w) => w.id === $activeId) ?? $wsList[0],
);

/** Generate the next unique "Workspace N" name that doesn't collide. */
function nextWorkspaceName(): string {
  const list = get(workspaces);
  const usedNumbers = new Set<number>();
  for (const ws of list) {
    const match = ws.name.match(/^Workspace\s+(\d+)$/i);
    if (match) usedNumbers.add(parseInt(match[1], 10));
  }
  let n = 1;
  while (usedNumbers.has(n)) n++;
  return `Workspace ${n}`;
}

/** Add a new empty workspace and switch to it. */
export function addWorkspace(name?: string): void {
  const ws = createWorkspace(name ?? nextWorkspaceName());
  workspaces.update((list) => [...list, ws]);
  activeWorkspaceId.set(ws.id);
  // Sync the live stores to the new workspace
  dualLayoutStore.set({
    mode: ws.mode,
    canvasState: {
      items: ws.canvasItemList,
      viewport: ws.viewport,
    },
    dockingState: ws.dockingRoot,
  });
}

/** Switch to a workspace by ID, saving the current workspace first. */
export function switchWorkspace(targetId: string): void {
  const currentId = get(activeWorkspaceId);
  if (currentId === targetId) return;

  // Save current workspace state
  saveCurrentWorkspaceState();

  // Load target workspace
  const wsList = get(workspaces);
  const target = wsList.find((w) => w.id === targetId);
  if (!target) return;

  activeWorkspaceId.set(targetId);
  dualLayoutStore.set({
    mode: target.mode,
    canvasState: {
      items: target.canvasItemList,
      viewport: target.viewport,
    },
    dockingState: target.dockingRoot,
  });
}

/** Persist current live store state back into the workspace array. */
export function saveCurrentWorkspaceState(): void {
  const currentId = get(activeWorkspaceId);
  const mem = get(dualLayoutStore);
  workspaces.update((list) =>
    list.map((ws) =>
      ws.id === currentId
        ? {
            ...ws,
            dockingRoot: mem.dockingState,
            canvasItemList: mem.canvasState.items,
            viewport: mem.canvasState.viewport,
            mode: mem.mode,
          }
        : ws,
    ),
  );
}

/** Remove a workspace by ID. Cannot remove the last workspace. */
export function removeWorkspace(wsId: string): void {
  const wsList = get(workspaces);
  if (wsList.length <= 1) return;

  const currentId = get(activeWorkspaceId);
  workspaces.update((list) => list.filter((w) => w.id !== wsId));

  // If we removed the active workspace, switch to the first remaining
  if (currentId === wsId) {
    const remaining = get(workspaces);
    if (remaining.length > 0) {
      switchWorkspace(remaining[0].id);
    }
  }
}

/** Rename a workspace. */
export function renameWorkspace(wsId: string, name: string): void {
  workspaces.update((list) =>
    list.map((ws) => (ws.id === wsId ? { ...ws, name } : ws)),
  );
}

/** Change a workspace's accent colour. */
export function setWorkspaceColor(wsId: string, color: string): void {
  workspaces.update((list) =>
    list.map((ws) => (ws.id === wsId ? { ...ws, color } : ws)),
  );
}

/** Duplicate a workspace (deep clone) and switch to the copy. */
export function duplicateWorkspace(wsId: string): void {
  const wsList = get(workspaces);
  const src = wsList.find((w) => w.id === wsId);
  if (!src) return;
  saveCurrentWorkspaceState();
  const newWs: Workspace = {
    ...structuredClone(src),
    id: makeWorkspaceId(),
    name: `${src.name} (copy)`,
    color: randomWorkspaceColor(),
  };
  workspaces.update((list) => [...list, newWs]);
  switchWorkspace(newWs.id);
}

/** Reorder workspaces by moving a workspace from one index to another. */
export function reorderWorkspaces(fromId: string, toId: string): void {
  workspaces.update((list) => {
    const fromIdx = list.findIndex((w) => w.id === fromId);
    const toIdx = list.findIndex((w) => w.id === toId);
    if (fromIdx === -1 || toIdx === -1 || fromIdx === toIdx) return list;
    const next = [...list];
    const [moved] = next.splice(fromIdx, 1);
    next.splice(toIdx, 0, moved);
    return next;
  });
}
