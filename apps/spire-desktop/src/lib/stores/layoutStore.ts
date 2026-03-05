/**
 * SPIRE — Layout Store
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
 * Every type in this module is plain JSON — no class instances, no
 * functions, no circular references.  `layoutRoot` can be serialised
 * with `JSON.stringify()` and restored with `JSON.parse()`.
 */

import { writable, derived, get } from "svelte/store";
import type { WidgetType } from "./notebookStore";
import { WIDGET_DEFINITIONS } from "./notebookStore";

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

/** Horizontal split — children are arranged left-to-right. */
export interface RowNode extends LayoutNodeBase {
  type: "row";
  /** Ordered child nodes. */
  children: LayoutNode[];
  /** Flex-grow proportions parallel to `children`. */
  sizes: number[];
}

/** Vertical split — children are arranged top-to-bottom. */
export interface ColNode extends LayoutNodeBase {
  type: "col";
  /** Ordered child nodes. */
  children: LayoutNode[];
  /** Flex-grow proportions parallel to `children`. */
  sizes: number[];
}

/** Tabbed group — one child visible at a time. */
export interface StackNode extends LayoutNodeBase {
  type: "stack";
  /** Child nodes (each tab). */
  children: LayoutNode[];
  /** Index of the currently visible tab. */
  activeIndex: number;
}

/** Leaf node — renders an actual widget component. */
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
// Stores
// ===========================================================================

/** The root of the recursive layout tree (docking mode). */
export const layoutRoot = writable<LayoutNode>(createDefaultLayout());

/** Items on the infinite canvas (canvas mode). */
export const canvasItems = writable<CanvasItem[]>(createDefaultCanvas());

/** Canvas viewport state. */
export const canvasViewport = writable<CanvasViewport>({
  panX: 0,
  panY: 0,
  zoom: 1,
});

/** Current view mode: "docking" or "canvas". */
export const viewMode = writable<ViewMode>("docking");

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
  layoutRoot.update((root) => {
    const tree = cloneTree(root);

    if (tree.id === nodeId) {
      // Splitting the root: wrap in a new container
      const newContainer: RowNode | ColNode = {
        id: makeLayoutId(),
        type: direction,
        sizes: [1, 1],
        children: [tree, createWidgetLeaf(newWidgetType)],
      } as RowNode | ColNode;
      return newContainer;
    }

    const newLeaf = createWidgetLeaf(newWidgetType);
    const newContainer: RowNode | ColNode = {
      id: makeLayoutId(),
      type: direction,
      sizes: [1, 1],
      children: [], // will be populated below
    } as RowNode | ColNode;

    // Find the node and replace it with a container
    const found = findNode(tree, nodeId);
    if (!found || !found.parent) return tree;

    const original = found.parent.children[found.index];
    newContainer.children = [original, newLeaf];

    found.parent.children[found.index] = newContainer;
    if ("sizes" in found.parent) {
      // sizes array length doesn't change; the slot is reused
    }

    return tree;
  });
}

/**
 * Close (remove) a widget or container node from the layout tree.
 * Automatically prunes single-child containers.
 */
export function closeNode(nodeId: string): void {
  layoutRoot.update((root) => {
    const tree = cloneTree(root);

    if (tree.id === nodeId) {
      // Closing the root → reset to default
      return createDefaultLayout();
    }

    const result = removeNode(tree, nodeId);
    return result ?? createDefaultLayout();
  });
}

/**
 * Update the flex-grow proportions of a container's children.
 */
export function resizePanes(nodeId: string, sizes: number[]): void {
  layoutRoot.update((root) => {
    const tree = cloneTree(root);

    if (tree.id === nodeId && tree.type !== "widget") {
      (tree as RowNode | ColNode).sizes = sizes;
      return tree;
    }

    const found = findNode(tree, nodeId);
    if (found && found.node.type !== "widget") {
      (found.node as RowNode | ColNode).sizes = sizes;
    }
    return tree;
  });
}

/**
 * Set the active tab index of a stack node.
 */
export function setActiveTab(stackId: string, index: number): void {
  layoutRoot.update((root) => {
    const tree = cloneTree(root);
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
    return tree;
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
  const root = get(layoutRoot);

  // Find the first widget leaf to split
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
  layoutRoot.set(root);
}

// ===========================================================================
// Docking Drag-and-Drop: moveNode Reducer
// ===========================================================================

/**
 * Position relative to the drop target.
 * - "before" / "after" — split the parent container
 * - "left" / "right" / "top" / "bottom" — split the target into a new container
 * - "center" — replace the target (tab-stack, future use)
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
 * @param sourceId  — ID of the widget leaf to move.
 * @param targetId  — ID of the node to drop onto.
 * @param position  — Where relative to the target to place the source.
 */
export function moveNode(
  sourceId: string,
  targetId: string,
  position: DropPosition,
): void {
  if (sourceId === targetId) return;

  layoutRoot.update((root) => {
    const tree = cloneTree(root);

    // Phase 1: Extract the source node
    const sourceFound = findNode(tree, sourceId);
    if (!sourceFound) return tree;

    // Deep-clone the source node before removal
    const sourceNode = cloneTree(sourceFound.node);

    // Remove source from tree
    let pruned: LayoutNode | null;
    if (tree.id === sourceId) {
      // Cannot move the root itself
      return tree;
    }
    pruned = removeNode(tree, sourceId);
    if (!pruned) return tree;

    // Phase 2: Locate the target in the pruned tree
    if (pruned.id === targetId) {
      // Target is the new root — wrap it
      const direction: "row" | "col" =
        position === "left" || position === "right" ? "row" : "col";
      const children =
        position === "left" || position === "top"
          ? [sourceNode, pruned]
          : position === "center"
            ? [pruned] // center: no-op for now
            : [pruned, sourceNode];

      if (position === "center") return pruned;

      return {
        id: makeLayoutId(),
        type: direction,
        sizes: Array(children.length).fill(1),
        children,
      } as RowNode | ColNode;
    }

    const targetFound = findNode(pruned, targetId);
    if (!targetFound || !targetFound.parent) return pruned;

    // Phase 3: Insert source relative to target
    if (position === "center") {
      // Replace the target with the source (swap)
      targetFound.parent.children[targetFound.index] = sourceNode;
      return pruned;
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
    // sizes array length doesn't change; the slot is reused.

    return pruned;
  });
}

/** Reset the docking layout to the default arrangement. */
export function resetDockingLayout(): void {
  layoutRoot.set(createDefaultLayout());
}

// ===========================================================================
// Canvas Reducers (Public API)
// ===========================================================================

/**
 * Add a widget to the infinite canvas.
 *
 * When x/y are not supplied, the widget is placed at the center
 * of the current viewport (accounting for pan and zoom).
 */
export function addCanvasItem(
  widgetType: WidgetType,
  x?: number,
  y?: number,
): void {
  const def = WIDGET_DEFINITIONS.find((d) => d.type === widgetType);
  const w = (def?.defaultColSpan ?? 2) * 200;
  const h = (def?.defaultRowSpan ?? 2) * 160;

  // Default to viewport center if no coordinates specified
  if (x === undefined || y === undefined) {
    const vp = get(canvasViewport);
    // Assume a ~900×600 visible area (reasonable default)
    x = x ?? (-vp.panX + 450) / (vp.zoom || 1) - w / 2;
    y = y ?? (-vp.panY + 300) / (vp.zoom || 1) - h / 2;
  }

  canvasItems.update((items) => [
    ...items,
    {
      id: makeLayoutId(),
      widgetType,
      widgetData: {},
      x,
      y,
      width: w,
      height: h,
    },
  ]);
}

/** Remove a canvas item by ID. */
export function removeCanvasItem(id: string): void {
  canvasItems.update((items) => items.filter((i) => i.id !== id));
}

/** Update a canvas item's position or size. */
export function updateCanvasItem(
  id: string,
  patch: Partial<Pick<CanvasItem, "x" | "y" | "width" | "height">>,
): void {
  canvasItems.update((items) =>
    items.map((i) => (i.id === id ? { ...i, ...patch } : i)),
  );
}

/** Reset canvas to empty. */
export function clearCanvas(): void {
  canvasItems.set([]);
  canvasViewport.set({ panX: 0, panY: 0, zoom: 1 });
}

// ===========================================================================
// View Mode Toggle
// ===========================================================================

/** Toggle between docking and canvas view modes.
 *
 * When switching paradigms, synchronise the widget set:
 * - Docking → Canvas: populate canvas items from the docking tree leaves
 *   (if canvas is currently empty), preserving widgetType and widgetData.
 * - Canvas → Docking: rebuild a docking tree from the canvas items
 *   (if the docking tree contains only the default layout).
 *
 * This ensures the researcher never loses widgets when changing views.
 */
export function toggleViewMode(): void {
  const current = get(viewMode);

  if (current === "docking") {
    // Switching to Canvas — sync docking widgets into canvas
    const currentCanvas = get(canvasItems);
    if (currentCanvas.length === 0) {
      const leaves = collectWidgetLeaves(get(layoutRoot));
      const vp = get(canvasViewport);
      // Arrange in a grid around the viewport center
      const centerX = (-vp.panX + 400) / (vp.zoom || 1);
      const centerY = (-vp.panY + 300) / (vp.zoom || 1);
      const cols = Math.max(2, Math.ceil(Math.sqrt(leaves.length)));
      const spacingX = 420;
      const spacingY = 360;

      const newItems: CanvasItem[] = leaves.map((leaf, i) => {
        const def = WIDGET_DEFINITIONS.find((d) => d.type === leaf.widgetType);
        const w = (def?.defaultColSpan ?? 2) * 200;
        const h = (def?.defaultRowSpan ?? 2) * 160;
        const col = i % cols;
        const row = Math.floor(i / cols);
        return {
          id: leaf.id,
          widgetType: leaf.widgetType,
          widgetData: { ...leaf.widgetData },
          x: centerX + col * spacingX,
          y: centerY + row * spacingY,
          width: w,
          height: h,
        };
      });
      canvasItems.set(newItems);
    }
  } else {
    // Switching to Docking — sync canvas widgets into docking tree
    const currentCanvas = get(canvasItems);
    if (currentCanvas.length > 0) {
      // Rebuild docking tree: single column of all canvas widget types
      const leaves = currentCanvas.map((ci) =>
        createWidgetLeaf(ci.widgetType, { ...ci.widgetData }),
      );

      if (leaves.length === 1) {
        layoutRoot.set(leaves[0]);
      } else {
        // Build a balanced column layout
        const root: ColNode = {
          id: makeLayoutId(),
          type: "col",
          sizes: leaves.map(() => 1),
          children: leaves,
        };
        layoutRoot.set(root);
      }
    }
  }

  viewMode.set(current === "docking" ? "canvas" : "docking");
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

function createWorkspace(name: string): Workspace {
  return {
    id: makeWorkspaceId(),
    name,
    dockingRoot: createDefaultLayout(),
    canvasItemList: [],
    viewport: { panX: 0, panY: 0, zoom: 1 },
    mode: "docking",
  };
}

/** All workspaces. */
export const workspaces = writable<Workspace[]>([createWorkspace("Workspace 1")]);

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

/** Add a new empty workspace and switch to it. */
export function addWorkspace(name?: string): void {
  const ws = createWorkspace(name ?? `Workspace ${get(workspaces).length + 1}`);
  workspaces.update((list) => [...list, ws]);
  activeWorkspaceId.set(ws.id);
  // Sync the live stores to the new workspace
  layoutRoot.set(ws.dockingRoot);
  canvasItems.set(ws.canvasItemList);
  canvasViewport.set(ws.viewport);
  viewMode.set(ws.mode);
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
  layoutRoot.set(target.dockingRoot);
  canvasItems.set(target.canvasItemList);
  canvasViewport.set(target.viewport);
  viewMode.set(target.mode);
}

/** Persist current live store state back into the workspace array. */
export function saveCurrentWorkspaceState(): void {
  const currentId = get(activeWorkspaceId);
  workspaces.update((list) =>
    list.map((ws) =>
      ws.id === currentId
        ? {
            ...ws,
            dockingRoot: get(layoutRoot),
            canvasItemList: get(canvasItems),
            viewport: get(canvasViewport),
            mode: get(viewMode),
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
