/**
 * SPIRE - Spatial Inference Engine
 *
 * Pure utility for inferring a recursive docking tree from 2D canvas bounds.
 * This module is intentionally decoupled from Svelte stores and DOM APIs.
 */

export interface SpatialCanvasItem {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

export type SpatialSplitType = "row" | "col";

export interface SpatialSplitNode {
  type: SpatialSplitType;
  children: SpatialDockNode[];
}

export interface SpatialTabNode {
  type: "tab";
  itemIds: string[];
}

export type SpatialDockNode = SpatialSplitNode | SpatialTabNode;

interface AxisProjection {
  item: SpatialCanvasItem;
  start: number;
  end: number;
}

interface AxisSplit {
  index: number;
  gap: number;
  left: SpatialCanvasItem[];
  right: SpatialCanvasItem[];
}

const MIN_SPLIT_GAP = 8;

function project(items: SpatialCanvasItem[], axis: "x" | "y"): AxisProjection[] {
  return items
    .map((item) => ({
      item,
      start: axis === "x" ? item.x : item.y,
      end: axis === "x" ? item.x + item.width : item.y + item.height,
    }))
    .sort((a, b) => a.start - b.start);
}

function bestGapSplit(items: SpatialCanvasItem[], axis: "x" | "y"): AxisSplit | null {
  if (items.length < 2) return null;

  const projections = project(items, axis);
  const n = projections.length;
  const prefixMaxEnd: number[] = Array(n).fill(0);
  const suffixMinStart: number[] = Array(n).fill(0);

  prefixMaxEnd[0] = projections[0].end;
  for (let i = 1; i < n; i++) {
    prefixMaxEnd[i] = Math.max(prefixMaxEnd[i - 1], projections[i].end);
  }

  suffixMinStart[n - 1] = projections[n - 1].start;
  for (let i = n - 2; i >= 0; i--) {
    suffixMinStart[i] = Math.min(suffixMinStart[i + 1], projections[i].start);
  }

  let bestIndex = -1;
  let bestGap = Number.NEGATIVE_INFINITY;

  for (let i = 0; i < n - 1; i++) {
    const gap = suffixMinStart[i + 1] - prefixMaxEnd[i];
    if (gap > bestGap) {
      bestGap = gap;
      bestIndex = i;
    }
  }

  if (bestIndex < 0 || bestGap < MIN_SPLIT_GAP) {
    return null;
  }

  const left = projections.slice(0, bestIndex + 1).map((p) => p.item);
  const right = projections.slice(bestIndex + 1).map((p) => p.item);
  if (left.length === 0 || right.length === 0) return null;

  return { index: bestIndex, gap: bestGap, left, right };
}

function sortForTabs(items: SpatialCanvasItem[]): SpatialCanvasItem[] {
  return [...items].sort((a, b) => {
    const yCmp = a.y - b.y;
    if (yCmp !== 0) return yCmp;
    return a.x - b.x;
  });
}

function forcedBisect(items: SpatialCanvasItem[]): AxisSplit {
  const sorted = sortForTabs(items);

  const minX = Math.min(...sorted.map((item) => item.x));
  const maxX = Math.max(...sorted.map((item) => item.x + item.width));
  const minY = Math.min(...sorted.map((item) => item.y));
  const maxY = Math.max(...sorted.map((item) => item.y + item.height));

  const spanX = maxX - minX;
  const spanY = maxY - minY;
  const axis: "x" | "y" = spanX >= spanY ? "x" : "y";

  const projected = project(sorted, axis);
  const mid = Math.floor(projected.length / 2);
  const left = projected.slice(0, mid).map((entry) => entry.item);
  const right = projected.slice(mid).map((entry) => entry.item);

  return {
    index: Math.max(0, mid - 1),
    gap: 0,
    left: left.length > 0 ? left : [projected[0].item],
    right: right.length > 0 ? right : [projected[projected.length - 1].item],
  };
}

function inferRecursive(items: SpatialCanvasItem[]): SpatialDockNode {
  if (items.length <= 1) {
    return {
      type: "tab",
      itemIds: items.map((i) => i.id),
    };
  }

  const xSplit = bestGapSplit(items, "x");
  const ySplit = bestGapSplit(items, "y");

  if (!xSplit && !ySplit) {
    const split = forcedBisect(items);
    const chooseRow = (() => {
      const minX = Math.min(...items.map((item) => item.x));
      const maxX = Math.max(...items.map((item) => item.x + item.width));
      const minY = Math.min(...items.map((item) => item.y));
      const maxY = Math.max(...items.map((item) => item.y + item.height));
      return (maxX - minX) >= (maxY - minY);
    })();

    return {
      type: chooseRow ? "row" : "col",
      children: [inferRecursive(split.left), inferRecursive(split.right)],
    };
  }

  const useX =
    xSplit !== null &&
    (ySplit === null || xSplit.gap >= ySplit.gap);

  const split = useX ? xSplit : ySplit;
  if (!split) {
    const forced = forcedBisect(items);
    return {
      type: "row",
      children: [inferRecursive(forced.left), inferRecursive(forced.right)],
    };
  }

  return {
    type: useX ? "row" : "col",
    children: [inferRecursive(split.left), inferRecursive(split.right)],
  };
}

function normalize(node: SpatialDockNode): SpatialDockNode {
  if (node.type === "tab") {
    return { type: "tab", itemIds: [...node.itemIds] };
  }

  const flattened: SpatialDockNode[] = [];
  for (const child of node.children) {
    const normChild = normalize(child);
    if (normChild.type === node.type) {
      flattened.push(...normChild.children);
    } else {
      flattened.push(normChild);
    }
  }

  return {
    type: node.type,
    children: flattened,
  };
}

/**
 * Infer a recursive docking tree from freeform canvas rectangles.
 *
 * Deterministic behavior:
 * - uses largest inter-cluster gap splitting along x/y,
 * - falls back to tab stacking for overlapping clusters,
 * - normalizes adjacent split nodes of the same orientation.
 */
export function inferDockingTreeFromCanvas(items: SpatialCanvasItem[]): SpatialDockNode {
  if (items.length === 0) {
    return { type: "tab", itemIds: [] };
  }
  return normalize(inferRecursive(items));
}

/**
 * Internal self-checks for deterministic behavior.
 * Throws if invariants are violated.
 */
export function runSpatialInferenceSelfChecks(): void {
  const horizontal = inferDockingTreeFromCanvas([
    { id: "a", x: 0, y: 0, width: 100, height: 80 },
    { id: "b", x: 140, y: 0, width: 100, height: 80 },
    { id: "c", x: 280, y: 0, width: 100, height: 80 },
  ]);

  if (horizontal.type !== "row") {
    throw new Error("Spatial inference self-check failed: expected root row split.");
  }
  if (horizontal.children.length !== 3) {
    throw new Error("Spatial inference self-check failed: expected three horizontal children.");
  }
  if (!horizontal.children.every((child) => child.type === "tab" && child.itemIds.length === 1)) {
    throw new Error("Spatial inference self-check failed: expected single-item tab leaves.");
  }
}
