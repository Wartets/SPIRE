/**
 * SPIRE - Context Menu Store
 *
 * Reactive store controlling the global custom right-click context menu.
 * Any component can call `showContextMenu(x, y, items)` to display a
 * menu at the given viewport coordinates.  The store is consumed by
 * `ContextMenu.svelte`, which renders the overlay.
 *
 * ## Architecture
 *
 * A single `<ContextMenu />` component is mounted once in the root layout.
 * Menu items are described by the `ContextMenuItem` discriminated union
 * defined in `$lib/types/menu.ts`, which supports:
 *
 *   - `action`    — clickable command
 *   - `separator` — horizontal divider
 *   - `toggle`    — checkbox row
 *   - `submenu`   — recursive nested children
 *
 * ## Usage
 *
 * ```ts
 * import { showContextMenu } from "$lib/stores/contextMenuStore";
 * import type { ContextMenuItem } from "$lib/types/menu";
 *
 * function handleContextMenu(e: MouseEvent) {
 *   e.preventDefault();
 *   showContextMenu(e.clientX, e.clientY, items);
 * }
 * ```
 */

import { writable, derived } from "svelte/store";
import type { ContextMenuItem } from "$lib/types/menu";

// Re-export the type so existing imports keep working.
export type { ContextMenuItem } from "$lib/types/menu";

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

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
 *
 * Coordinates are stored as-is; viewport clamping is performed
 * inside the rendering component after measuring the DOM element.
 */
export function showContextMenu(
  x: number,
  y: number,
  items: ContextMenuItem[],
): void {
  contextMenu.set({ visible: true, x, y, items });
}

/** Hide the context menu and all open submenus. */
export function hideContextMenu(): void {
  contextMenu.set(initialState);
}
