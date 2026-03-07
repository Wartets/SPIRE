/**
 * SPIRE - Keyboard Shortcut Service
 *
 * Centralised, global keybind engine supporting:
 *   - Single-key shortcuts  (e.g. "Mod+N", "Delete")
 *   - Two-step chord sequences (e.g. "Mod+K → Mod+S")
 *   - Platform-normalised modifier resolution (Cmd on macOS, Ctrl elsewhere)
 *   - User-customisable overrides persisted to localStorage
 *   - Conflict detection & resolution helpers
 *   - Transient status indicator for pending chord state
 *
 * ## Architecture
 *
 * The service owns a single capture-phase `keydown` listener on `window`.
 * Every keystroke is normalised into a canonical string (e.g. "mod+shift+r")
 * and matched against the active binding table.  When a match is found the
 * corresponding `commandId` is dispatched through the CommandRegistry.
 *
 * The binding table is the merge of DEFAULT_KEYBINDINGS (immutable) with
 * user overrides loaded from localStorage.  This merge is performed once
 * on initialisation and re-applied whenever the user edits a binding.
 *
 * Chord sequences use a lightweight state machine:
 *   1. First key sets `pendingChord` and starts a 1500ms timeout.
 *   2. If the second key arrives in time and matches, the command fires.
 *   3. Otherwise the pending state is cleared.
 *
 * ## Usage
 *
 * ```ts
 * import { initShortcutService, destroyShortcutService } from "./ShortcutService";
 *
 * onMount(() => initShortcutService());
 * onDestroy(() => destroyShortcutService());
 * ```
 */

import { writable, get } from "svelte/store";
import {
  executeCommand,
  paletteOpen,
} from "$lib/core/services/CommandRegistry";

// ===========================================================================
// Types
// ===========================================================================

/** A single keyboard binding entry. */
export interface Keybinding {
  /** Command ID dispatched through CommandRegistry (e.g. "spire.workspace.save"). */
  commandId: string;
  /** Normalised primary key combo (e.g. "mod+s", "mod+k"). */
  key: string;
  /** Optional second key of a chord (e.g. "mod+s" for the chord Mod+K → Mod+S). */
  chordKey?: string;
  /** Logical grouping for the UI panel and cheat sheet. */
  category: KeybindCategory;
}

export type KeybindCategory =
  | "File"
  | "View"
  | "Canvas"
  | "Notebook"
  | "Navigation"
  | "Edit"
  | "Help";

/** Serialisable user override stored in localStorage. */
export interface KeybindOverride {
  commandId: string;
  key: string;
  chordKey?: string;
}

// ===========================================================================
// Platform Detection
// ===========================================================================

const IS_MAC =
  typeof navigator !== "undefined" &&
  /Mac|iPhone|iPad|iPod/.test(navigator.platform);

// ===========================================================================
// Event Normalisation
// ===========================================================================

/**
 * Convert a `KeyboardEvent` into a canonical, order-stable string.
 *
 * Modifiers are emitted in the fixed order `mod+ctrl+shift+alt+<key>`.
 * "mod" abstracts the platform modifier (Meta on macOS, Ctrl elsewhere).
 * If the event carries only Ctrl on Windows this yields "mod+<key>".
 *
 * Examples:
 *   - Ctrl+Shift+R on Windows → "mod+shift+r"
 *   - Cmd+S on macOS           → "mod+s"
 *   - Plain Delete             → "delete"
 *   - Ctrl+K on Windows        → "mod+k"
 *   - Alt+1                    → "alt+1"
 */
export function normaliseKeyEvent(event: KeyboardEvent): string {
  const parts: string[] = [];

  // Platform modifier ("mod") — Cmd on Mac, Ctrl on Windows/Linux
  const modHeld = IS_MAC ? event.metaKey : event.ctrlKey;
  if (modHeld) parts.push("mod");

  // Explicit ctrl on Mac (rare but possible; never duplicate with mod on Windows)
  if (IS_MAC && event.ctrlKey) parts.push("ctrl");

  if (event.shiftKey) parts.push("shift");
  if (event.altKey) parts.push("alt");

  // Normalise the primary key
  let key = event.key.toLowerCase();
  // Map whitespace / special names
  if (key === " ") key = "space";
  if (key === "escape") key = "escape";
  // Skip lone modifier keys
  if (["control", "shift", "alt", "meta"].includes(key)) return "";

  parts.push(key);
  return parts.join("+");
}

/**
 * Produce a human-readable label from a normalised key string.
 * E.g. "mod+shift+r" → "Ctrl+Shift+R" (Windows) or "⌘+Shift+R" (Mac).
 */
export function formatKeyLabel(normalised: string): string {
  if (!normalised) return "";
  return normalised
    .split("+")
    .map((tok) => {
      switch (tok) {
        case "mod":
          return IS_MAC ? "⌘" : "Ctrl";
        case "ctrl":
          return "Ctrl";
        case "shift":
          return "Shift";
        case "alt":
          return IS_MAC ? "⌥" : "Alt";
        default:
          // Capitalise single letters, keep special names title-cased
          if (tok.length === 1) return tok.toUpperCase();
          return tok.charAt(0).toUpperCase() + tok.slice(1);
      }
    })
    .join("+");
}

/**
 * Format a full Keybinding as a display string.
 * Chords are joined with " → ".
 */
export function formatBindingLabel(kb: Keybinding): string {
  const first = formatKeyLabel(kb.key);
  if (kb.chordKey) {
    return `${first} → ${formatKeyLabel(kb.chordKey)}`;
  }
  return first;
}

// ===========================================================================
// Default Keybindings
// ===========================================================================

export const DEFAULT_KEYBINDINGS: Keybinding[] = [
  // ── File ──
  { commandId: "spire.workspace.save",      key: "mod+s",                      category: "File" },
  { commandId: "spire.workspace.save_as",   key: "mod+k", chordKey: "mod+s",   category: "File" },

  // ── View ──
  { commandId: "spire.view.toggle_canvas_mode", key: "mod+e",                  category: "View" },
  { commandId: "spire.ui.reset_layout",         key: "mod+shift+r",            category: "View" },
  { commandId: "spire.ui.toggle_log",           key: "mod+j",                  category: "View" },
  { commandId: "spire.help.tutorial",            key: "mod+shift+t",            category: "View" },

  // ── Canvas ──
  { commandId: "spire.canvas.add_widget",    key: "mod+n",                     category: "Canvas" },
  { commandId: "spire.canvas.zoom_in",       key: "mod+=",                     category: "Canvas" },
  { commandId: "spire.canvas.zoom_out",      key: "mod+-",                     category: "Canvas" },
  { commandId: "spire.canvas.reset_zoom",    key: "mod+0",                     category: "Canvas" },
  { commandId: "spire.canvas.delete_selected", key: "delete",                  category: "Canvas" },
  { commandId: "spire.canvas.duplicate",     key: "mod+d",                     category: "Canvas" },
  { commandId: "spire.canvas.select_all",    key: "mod+a",                     category: "Canvas" },

  // ── Notebook ──
  { commandId: "spire.notebook.run_cell",    key: "mod+enter",                 category: "Notebook" },
  { commandId: "spire.notebook.run_all",     key: "mod+shift+enter",           category: "Notebook" },

  // ── Navigation ──
  { commandId: "spire.palette.open",         key: "mod+k",                     category: "Navigation" },
  { commandId: "spire.shortcut.cheatsheet",  key: "mod+/",                     category: "Navigation" },

  // ── Edit ──
  { commandId: "spire.edit.undo",            key: "mod+z",                     category: "Edit" },
  { commandId: "spire.edit.redo",            key: "mod+shift+z",               category: "Edit" },
  { commandId: "spire.search.focus",         key: "mod+f",                     category: "Edit" },
];

// ===========================================================================
// Reactive Stores
// ===========================================================================

/** Currently active keybindings (defaults merged with user overrides). */
export const activeKeybindings = writable<Keybinding[]>([...DEFAULT_KEYBINDINGS]);

/** Transient chord indicator text (empty when idle). */
export const chordIndicator = writable<string>("");

/** Whether the cheat sheet overlay is visible. */
export const cheatSheetOpen = writable<boolean>(false);
export const keybindPanelOpen = writable<boolean>(false);

// ===========================================================================
// localStorage Persistence
// ===========================================================================

const STORAGE_KEY = "spire_keybind_overrides";

function loadOverrides(): KeybindOverride[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    return JSON.parse(raw) as KeybindOverride[];
  } catch {
    return [];
  }
}

function saveOverrides(overrides: KeybindOverride[]): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(overrides));
  } catch {
    // Silently ignore quota errors
  }
}

/**
 * Merge user overrides onto the default binding table.
 * An override replaces the entry with the same commandId.
 */
function mergeOverrides(overrides: KeybindOverride[]): Keybinding[] {
  const result = DEFAULT_KEYBINDINGS.map((kb) => ({ ...kb }));
  for (const ov of overrides) {
    const existing = result.find((kb) => kb.commandId === ov.commandId);
    if (existing) {
      existing.key = ov.key;
      existing.chordKey = ov.chordKey;
    }
  }
  return result;
}

/** Reload bindings from localStorage and push into the store. */
export function reloadBindings(): void {
  const overrides = loadOverrides();
  activeKeybindings.set(mergeOverrides(overrides));
}

// ===========================================================================
// User Customisation API
// ===========================================================================

export interface ConflictInfo {
  existingCommandId: string;
  existingKey: string;
  existingChordKey?: string;
}

/**
 * Check if a proposed key (+ optional chordKey) conflicts with any
 * existing binding other than `excludeCommandId`.
 * Returns the conflicting binding info, or `null` if no conflict.
 */
export function detectConflict(
  key: string,
  chordKey: string | undefined,
  excludeCommandId: string,
): ConflictInfo | null {
  const bindings = get(activeKeybindings);
  for (const kb of bindings) {
    if (kb.commandId === excludeCommandId) continue;
    // For chords, only conflict if both parts match
    if (chordKey) {
      if (kb.key === key && kb.chordKey === chordKey) {
        return { existingCommandId: kb.commandId, existingKey: kb.key, existingChordKey: kb.chordKey };
      }
    } else {
      // Single-key binding: conflicts with another single-key or is the first
      // half of a chord. Only exact single-key match counts as conflict.
      if (kb.key === key && !kb.chordKey) {
        return { existingCommandId: kb.commandId, existingKey: kb.key };
      }
    }
  }
  return null;
}

/**
 * Update a binding for a given commandId.  Persists to localStorage.
 */
export function updateBinding(
  commandId: string,
  newKey: string,
  newChordKey?: string,
): void {
  const overrides = loadOverrides();
  const idx = overrides.findIndex((o) => o.commandId === commandId);
  const entry: KeybindOverride = { commandId, key: newKey, chordKey: newChordKey };
  if (idx >= 0) {
    overrides[idx] = entry;
  } else {
    overrides.push(entry);
  }
  saveOverrides(overrides);
  reloadBindings();
}

/**
 * Remove the user override for a command (reverts to default).
 */
export function resetBinding(commandId: string): void {
  const overrides = loadOverrides().filter((o) => o.commandId !== commandId);
  saveOverrides(overrides);
  reloadBindings();
}

/**
 * Unbind a command entirely (set its key to empty).
 */
export function unbindCommand(commandId: string): void {
  updateBinding(commandId, "");
}

/**
 * Swap the bindings of two commands.
 */
export function swapBindings(commandIdA: string, commandIdB: string): void {
  const bindings = get(activeKeybindings);
  const a = bindings.find((kb) => kb.commandId === commandIdA);
  const b = bindings.find((kb) => kb.commandId === commandIdB);
  if (!a || !b) return;
  updateBinding(commandIdA, b.key, b.chordKey);
  updateBinding(commandIdB, a.key, a.chordKey);
}

/**
 * Clear all user overrides and reset to defaults.
 */
export function resetAllBindings(): void {
  saveOverrides([]);
  reloadBindings();
}

// ===========================================================================
// Chord State Machine
// ===========================================================================

let pendingChord: { key: string; bindings: Keybinding[] } | null = null;
let chordTimeout: ReturnType<typeof setTimeout> | null = null;

const CHORD_TIMEOUT_MS = 1500;

function clearChord(): void {
  pendingChord = null;
  if (chordTimeout) {
    clearTimeout(chordTimeout);
    chordTimeout = null;
  }
  chordIndicator.set("");
}

// ===========================================================================
// Global Keydown Handler
// ===========================================================================

function handleKeydown(event: KeyboardEvent): void {
  const normalised = normaliseKeyEvent(event);
  if (!normalised) return; // Lone modifier press

  const bindings = get(activeKeybindings);

  // ── Chord second-key resolution ──
  if (pendingChord) {
    const matched = pendingChord.bindings.find(
      (kb) => kb.chordKey === normalised,
    );
    if (matched && matched.key) {
      event.preventDefault();
      event.stopPropagation();
      clearChord();
      executeCommand(matched.commandId);
      return;
    }
    // No match — clear and fall through to normal handling
    clearChord();
  }

  // ── Skip when user is typing in an input ──
  const target = event.target as HTMLElement;
  const isInput =
    target.tagName === "INPUT" ||
    target.tagName === "TEXTAREA" ||
    target.tagName === "SELECT" ||
    target.isContentEditable;

  // Always allow the palette toggle (Mod+K) and Escape regardless of focus
  if (normalised === "mod+k") {
    // Mod+K is both a palette toggle AND the first key of several chords.
    // If there are chord bindings starting with mod+k, enter chord mode.
    const chordBindings = bindings.filter(
      (kb) => kb.key === "mod+k" && kb.chordKey,
    );
    // Also check if mod+k itself is a standalone binding (palette)
    const standalone = bindings.find(
      (kb) => kb.key === "mod+k" && !kb.chordKey,
    );

    if (chordBindings.length > 0) {
      event.preventDefault();
      event.stopPropagation();
      pendingChord = { key: normalised, bindings: chordBindings };
      chordIndicator.set(
        `${formatKeyLabel("mod+k")} pressed — waiting for second key…`,
      );
      chordTimeout = setTimeout(() => {
        // Timeout: if there was also a standalone, execute it
        if (standalone) {
          executeCommand(standalone.commandId);
        }
        clearChord();
      }, CHORD_TIMEOUT_MS);
      return;
    }
    // No chords — treat as standalone
    if (standalone) {
      event.preventDefault();
      event.stopPropagation();
      executeCommand(standalone.commandId);
      return;
    }
  }

  // Escape closes cheat sheet
  if (normalised === "escape") {
    if (get(cheatSheetOpen)) {
      event.preventDefault();
      cheatSheetOpen.set(false);
      return;
    }
    if (get(paletteOpen)) {
      event.preventDefault();
      paletteOpen.set(false);
      return;
    }
    return;
  }

  // Skip shortcut resolution when palette is open or focus is in an input
  if (get(paletteOpen) || isInput) return;

  // ── Check for chord-start keys (any binding that has a chordKey) ──
  const chordStarters = bindings.filter(
    (kb) => kb.key === normalised && kb.chordKey,
  );
  if (chordStarters.length > 0) {
    // Also check if there's a standalone binding with this key
    const standalone = bindings.find(
      (kb) => kb.key === normalised && !kb.chordKey,
    );
    event.preventDefault();
    event.stopPropagation();
    pendingChord = { key: normalised, bindings: chordStarters };
    chordIndicator.set(
      `${formatKeyLabel(normalised)} pressed — waiting for second key…`,
    );
    chordTimeout = setTimeout(() => {
      if (standalone) {
        executeCommand(standalone.commandId);
      }
      clearChord();
    }, CHORD_TIMEOUT_MS);
    return;
  }

  // ── Single-key binding resolution ──
  const match = bindings.find(
    (kb) => kb.key === normalised && !kb.chordKey,
  );
  if (match && match.key) {
    event.preventDefault();
    event.stopPropagation();
    executeCommand(match.commandId);
  }
}

// ===========================================================================
// Lifecycle
// ===========================================================================

let _initialised = false;

/**
 * Initialise the global shortcut listener.
 * Safe to call multiple times — only the first call attaches the listener.
 */
export function initShortcutService(): void {
  if (_initialised) return;
  _initialised = true;
  reloadBindings();
  window.addEventListener("keydown", handleKeydown, { capture: true });
}

/**
 * Tear down the global shortcut listener.
 */
export function destroyShortcutService(): void {
  if (!_initialised) return;
  _initialised = false;
  window.removeEventListener("keydown", handleKeydown, { capture: true });
  clearChord();
}
