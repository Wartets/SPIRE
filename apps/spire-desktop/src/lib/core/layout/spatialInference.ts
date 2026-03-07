/**
 * Spatial Inference Engine: Converts 2D canvas widget positions into a recursive docking tree.
 * Pure mathematical utility, strictly decoupled from Svelte stores and DOM.
 */

import type { CanvasItem } from '../../stores/layoutStore';
import type { LayoutNode, RowNode, ColNode, StackNode, WidgetLeaf } from '../../stores/layoutStore';

/**
 * Infer a recursive docking tree from an array of canvas widget bounding boxes.
 * @param items Array of CanvasItem {id, x, y, width, height, widgetType, widgetData}
 * @returns LayoutNode (RowNode, ColNode, StackNode, WidgetLeaf)
 */
export function inferDockingTreeFromCanvas(items: CanvasItem[]): LayoutNode {
  if (items.length === 0) {
    throw new Error('No canvas items provided');
  }
  if (items.length === 1) {
    // Single widget: return as WidgetLeaf
    return {
      id: items[0].id,
      type: 'widget',
      widgetType: items[0].widgetType,
      widgetData: items[0].widgetData,
    } as WidgetLeaf;
  }

  // Helper: bounding box overlap
  function overlaps(a: CanvasItem, b: CanvasItem): boolean {
    return (
      a.x < b.x + b.width &&
      a.x + a.width > b.x &&
      a.y < b.y + b.height &&
      a.y + a.height > b.y
    );
  }

  // Find largest gap (horizontal or vertical)
  let maxGap = 0;
  let splitAxis: 'x' | 'y' = 'x';
  let splitPos = 0;

  // Sort by x and y
  const sortedX = [...items].sort((a, b) => a.x - b.x);
  const sortedY = [...items].sort((a, b) => a.y - b.y);

  // Horizontal gaps
  for (let i = 0; i < sortedX.length - 1; i++) {
    const right = sortedX[i].x + sortedX[i].width;
    const left = sortedX[i + 1].x;
    const gap = left - right;
    if (gap > maxGap) {
      maxGap = gap;
      splitAxis = 'x';
      splitPos = (right + left) / 2;
    }
  }
  // Vertical gaps
  for (let i = 0; i < sortedY.length - 1; i++) {
    const bottom = sortedY[i].y + sortedY[i].height;
    const top = sortedY[i + 1].y;
    const gap = top - bottom;
    if (gap > maxGap) {
      maxGap = gap;
      splitAxis = 'y';
      splitPos = (bottom + top) / 2;
    }
  }

  // If no significant gap, treat as overlapping: stack as tabs
  if (maxGap < 32) {
    // All overlap: create StackNode
    return {
      id: 'stack-' + items.map(i => i.id).join('-'),
      type: 'stack',
      children: items.map(i => ({
        id: i.id,
        type: 'widget',
        widgetType: i.widgetType,
        widgetData: i.widgetData,
      }) as WidgetLeaf),
      activeIndex: 0,
    } as StackNode;
  }

  // Split items into two groups
  const groupA: CanvasItem[] = [];
  const groupB: CanvasItem[] = [];
  for (const item of items) {
    if (splitAxis === 'x') {
      // Split by x
      if (item.x + item.width / 2 < splitPos) {
        groupA.push(item);
      } else {
        groupB.push(item);
      }
    } else {
      // Split by y
      if (item.y + item.height / 2 < splitPos) {
        groupA.push(item);
      } else {
        groupB.push(item);
      }
    }
  }

  // Recursively build RowNode or ColNode
  if (splitAxis === 'x') {
    return {
      id: 'row-' + groupA.map(i => i.id).join('-') + '-' + groupB.map(i => i.id).join('-'),
      type: 'row',
      children: [
        inferDockingTreeFromCanvas(groupA),
        inferDockingTreeFromCanvas(groupB),
      ],
      sizes: [1, 1],
    } as RowNode;
  } else {
    return {
      id: 'col-' + groupA.map(i => i.id).join('-') + '-' + groupB.map(i => i.id).join('-'),
      type: 'col',
      children: [
        inferDockingTreeFromCanvas(groupA),
        inferDockingTreeFromCanvas(groupB),
      ],
      sizes: [1, 1],
    } as ColNode;
  }
}

// Unit test example (to be expanded)
export function testSpatialInference(): void {
  const items = [
    { id: 'a', x: 0, y: 0, width: 100, height: 100, widgetType: 'model', widgetData: {} },
    { id: 'b', x: 120, y: 0, width: 100, height: 100, widgetType: 'diagram', widgetData: {} },
    { id: 'c', x: 240, y: 0, width: 100, height: 100, widgetType: 'log', widgetData: {} },
  ];
  const tree = inferDockingTreeFromCanvas(items);
  console.log(JSON.stringify(tree, null, 2));
}
