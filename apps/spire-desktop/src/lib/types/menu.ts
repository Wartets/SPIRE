/**
 * SPIRE - Context Menu Data Structures
 *
 * Rich, recursive menu item types for the global context menu system.
 * Every context menu in the application—canvas background, widget headers,
 * notebook cells, layout splitters—is built from these types.
 *
 * The architecture is intentionally generic so that future domain-specific
 * commands (e.g. "Extract Feynman Rules" on a Lagrangian AST node) can
 * be added without modifying the rendering engine.
 */

// ---------------------------------------------------------------------------
// Menu Item Types
// ---------------------------------------------------------------------------

/** Base properties shared by all menu item variants. */
interface MenuItemBase {
  /** Unique identifier within the menu scope. */
  id: string;
  /** Display label. */
  label: string;
  /** Optional inline SVG string or short identifier for an icon prefix. */
  icon?: string;
  /** Optional keyboard shortcut hint (display only, does not bind). */
  shortcut?: string;
  /** Whether this item is currently non-interactive. */
  disabled?: boolean;
  /** Explanatory text shown when hovering a disabled item. */
  tooltip?: string;
}

/** A standard clickable menu action. */
export interface MenuActionItem extends MenuItemBase {
  type: "action";
  /** Callback invoked when the item is activated. */
  action: () => void;
}

/** A visual separator line between groups of items. */
export interface MenuSeparatorItem {
  type: "separator";
  id: string;
}

/** A boolean toggle displayed as a checkbox row. */
export interface MenuToggleItem extends MenuItemBase {
  type: "toggle";
  /** Current checked state. */
  checked: boolean;
  /** Callback invoked with the new checked state when toggled. */
  action: (checked: boolean) => void;
}

/** A submenu that expands on hover to reveal nested children. */
export interface MenuSubmenuItem extends MenuItemBase {
  type: "submenu";
  /** Nested items rendered in the child panel. */
  children: ContextMenuItem[];
}

/**
 * Discriminated union of all menu item variants.
 *
 * ```ts
 * const item: ContextMenuItem = {
 *   type: "submenu",
 *   id: "add-widget",
 *   label: "Add Widget",
 *   icon: "+",
 *   children: [ ... ],
 * };
 * ```
 */
export type ContextMenuItem =
  | MenuActionItem
  | MenuSeparatorItem
  | MenuToggleItem
  | MenuSubmenuItem;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Create a separator item with a generated id. */
export function menuSeparator(id?: string): MenuSeparatorItem {
  return { type: "separator", id: id ?? `sep-${Math.random().toString(36).slice(2, 8)}` };
}

/** Create a simple action item. */
export function menuAction(
  id: string,
  label: string,
  action: () => void,
  opts?: Partial<Pick<MenuActionItem, "icon" | "shortcut" | "disabled" | "tooltip">>,
): MenuActionItem {
  return { type: "action", id, label, action, ...opts };
}

/** Create a submenu item. */
export function menuSubmenu(
  id: string,
  label: string,
  children: ContextMenuItem[],
  opts?: Partial<Pick<MenuSubmenuItem, "icon" | "disabled" | "tooltip">>,
): MenuSubmenuItem {
  return { type: "submenu", id, label, children, ...opts };
}

/** Create a toggle item. */
export function menuToggle(
  id: string,
  label: string,
  checked: boolean,
  action: (checked: boolean) => void,
  opts?: Partial<Pick<MenuToggleItem, "icon" | "disabled" | "tooltip">>,
): MenuToggleItem {
  return { type: "toggle", id, label, checked, action, ...opts };
}
