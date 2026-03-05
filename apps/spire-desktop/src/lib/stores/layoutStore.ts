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

/** Reset the docking layout to the default arrangement. */
export function resetDockingLayout(): void {
  layoutRoot.set(createDefaultLayout());
}

// ===========================================================================
// Canvas Reducers (Public API)
// ===========================================================================

/**
 * Add a widget to the infinite canvas at the given position.
 */
export function addCanvasItem(
  widgetType: WidgetType,
  x = 100,
  y = 100,
): void {
  const def = WIDGET_DEFINITIONS.find((d) => d.type === widgetType);
  const w = (def?.defaultColSpan ?? 2) * 200;
  const h = (def?.defaultRowSpan ?? 2) * 160;

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

/** Toggle between docking and canvas view modes. */
export function toggleViewMode(): void {
  viewMode.update((m) => (m === "docking" ? "canvas" : "docking"));
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
