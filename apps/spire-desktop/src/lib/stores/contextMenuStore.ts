/**
 * SPIRE — Context Menu Store
 *
 * Reactive store controlling a global custom right-click context menu.
 * Widgets populate the menu items dynamically based on what was clicked.
 * The store is consumed by `ContextMenu.svelte`, which renders the
 * overlay at the stored coordinates.
 *
 * ## Usage
 *
 * 1. Call `showContextMenu(x, y, items)` to display the menu.
 * 2. Call `hideContextMenu()` to dismiss it.
 * 3. Subscribe to `contextMenu` in `ContextMenu.svelte`.
 */

import { writable, derived } from "svelte/store";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** A single item in the context menu. */
export interface ContextMenuItem {
  /** Unique identifier for this menu item. */
  id: string;
  /** Display label. */
  label: string;
  /** Optional leading icon character/emoji. */
  icon?: string;
  /** Optional keyboard shortcut hint (display only). */
  shortcut?: string;
  /** Whether this item is currently disabled. */
  disabled?: boolean;
  /** If true, render a horizontal separator BEFORE this item. */
  separator?: boolean;
  /** Callback invoked when this item is clicked. */
  action: () => void;
}

/** Full context menu state. */
export interface ContextMenuState {
  /** Whether the menu is currently visible. */
  visible: boolean;
  /** Viewport X coordinate (left edge of menu). */
  x: number;
  /** Viewport Y coordinate (top edge of menu). */
  y: number;
  /** The items to display. */
  items: ContextMenuItem[];
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

const initialState: ContextMenuState = {
  visible: false,
  x: 0,
  y: 0,
  items: [],
};

export const contextMenu = writable<ContextMenuState>(initialState);

/** Whether the context menu is currently visible. */
export const contextMenuVisible = derived(contextMenu, ($cm) => $cm.visible);

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

/**
 * Display the context menu at the given viewport coordinates
 * with the given set of items.
 */
export function showContextMenu(
  x: number,
  y: number,
  items: ContextMenuItem[],
): void {
  // Clamp so the menu doesn't overflow the viewport
  const clampedX = Math.min(x, window.innerWidth - 200);
  const clampedY = Math.min(y, window.innerHeight - 200);
  contextMenu.set({ visible: true, x: clampedX, y: clampedY, items });
}

/** Hide the context menu. */
export function hideContextMenu(): void {
  contextMenu.set(initialState);
}
