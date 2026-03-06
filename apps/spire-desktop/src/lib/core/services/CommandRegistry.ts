/**
 * SPIRE - Command Registry
 *
 * Centralised, decoupled service for registering, resolving, and
 * executing named commands across the entire application.  Any module
 * or UI component can register executable commands at mount time and
 * clean them up on unmount.  The Command Palette, Quick Toolbar, and
 * Global Keybind Manager all consume this registry without knowing
 * anything about the commands' implementations.
 *
 * Architecture:
 *   - Internal state is a Svelte writable<Map<string, Command>>
 *   - Derived stores expose sorted arrays and filtered subsets
 *   - Shortcut strings are parsed once and matched against KeyboardEvents
 */

import { writable, derived } from "svelte/store";

// ---------------------------------------------------------------------------
// Command Interface
// ---------------------------------------------------------------------------

export interface Command {
  /** Unique, dot-namespaced identifier (e.g. "spire.model.load"). */
  id: string;
  /** Human-readable title shown in the palette (e.g. "Load Model"). */
  title: string;
  /** Grouping category for visual separation (e.g. "Model", "View"). */
  category: string;
  /** Optional keyboard shortcut (e.g. "Mod+Shift+L", "Ctrl+K"). */
  shortcut?: string;
  /** The action to perform when the command is invoked. */
  execute: () => void;
  /** Optional text-based icon for the Quick Toolbar. */
  icon?: string;
  /** If true, the command appears in the Quick Toolbar. */
  pinned?: boolean;
}

// ---------------------------------------------------------------------------
// Parsed Shortcut Representation
// ---------------------------------------------------------------------------

export interface ParsedShortcut {
  /** Meta / Cmd on macOS, Ctrl on Windows/Linux. */
  mod: boolean;
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
  /** The primary key in lowercase (e.g. "k", "enter", "r"). */
  key: string;
}

// ---------------------------------------------------------------------------
// Internal Store
// ---------------------------------------------------------------------------

const _commandMap = writable<Map<string, Command>>(new Map());

// ---------------------------------------------------------------------------
// Public Derived Stores
// ---------------------------------------------------------------------------

/**
 * All registered commands as a sorted array.
 * Sorted by category (alphabetical), then by title within each category.
 */
export const commands = derived(_commandMap, ($map) => {
  const arr = Array.from($map.values());
  arr.sort((a, b) => {
    const catCmp = a.category.localeCompare(b.category);
    if (catCmp !== 0) return catCmp;
    return a.title.localeCompare(b.title);
  });
  return arr;
});

/**
 * Subset of commands where `pinned === true`, for the Quick Toolbar.
 */
export const pinnedCommands = derived(commands, ($cmds) =>
  $cmds.filter((c) => c.pinned === true),
);

// ---------------------------------------------------------------------------
// Registry API
// ---------------------------------------------------------------------------

/**
 * Register a command in the global registry.
 * If a command with the same `id` already exists, it is replaced.
 */
export function registerCommand(cmd: Command): void {
  _commandMap.update((map) => {
    const next = new Map(map);
    next.set(cmd.id, cmd);
    return next;
  });
}

/**
 * Remove a command from the registry by its unique ID.
 */
export function unregisterCommand(id: string): void {
  _commandMap.update((map) => {
    const next = new Map(map);
    next.delete(id);
    return next;
  });
}

/**
 * Execute a command by its ID.  Returns `true` if the command
 * was found and invoked, `false` otherwise.
 */
export function executeCommand(id: string): boolean {
  let found = false;
  _commandMap.update((map) => {
    const cmd = map.get(id);
    if (cmd) {
      cmd.execute();
      found = true;
    }
    return map;
  });
  return found;
}

/**
 * Look up a single command by ID.
 */
export function getCommand(id: string): Command | undefined {
  let result: Command | undefined;
  _commandMap.update((map) => {
    result = map.get(id);
    return map;
  });
  return result;
}

// ---------------------------------------------------------------------------
// Shortcut Parsing & Matching
// ---------------------------------------------------------------------------

const IS_MAC =
  typeof navigator !== "undefined" &&
  /Mac|iPhone|iPad|iPod/.test(navigator.platform);

/**
 * Parse a shortcut string like "Mod+Shift+K" into a structured object.
 *
 * "Mod" resolves to Meta (Cmd) on macOS and Ctrl on Windows/Linux.
 * Tokens are case-insensitive and order-independent.
 */
export function parseShortcut(shortcut: string): ParsedShortcut {
  const tokens = shortcut
    .split("+")
    .map((t) => t.trim().toLowerCase());

  let mod = false;
  let ctrl = false;
  let shift = false;
  let alt = false;
  let key = "";

  for (const token of tokens) {
    switch (token) {
      case "mod":
        mod = true;
        break;
      case "ctrl":
      case "control":
        ctrl = true;
        break;
      case "shift":
        shift = true;
        break;
      case "alt":
      case "option":
        alt = true;
        break;
      default:
        key = token;
        break;
    }
  }

  return { mod, ctrl, shift, alt, key };
}

/**
 * Test whether a KeyboardEvent matches a parsed shortcut.
 */
export function matchesShortcut(
  event: KeyboardEvent,
  parsed: ParsedShortcut,
): boolean {
  // Resolve "Mod" to the platform-appropriate modifier
  const modActive = IS_MAC ? event.metaKey : event.ctrlKey;
  if (parsed.mod && !modActive) return false;
  if (parsed.ctrl && !event.ctrlKey) return false;
  if (parsed.shift && !event.shiftKey) return false;
  if (parsed.alt && !event.altKey) return false;

  // Prevent false positives: if the parsed shortcut does NOT require
  // a modifier but the event has one pressed, skip matching for bare keys.
  if (!parsed.mod && !parsed.ctrl && event.ctrlKey) return false;
  if (!parsed.mod && !parsed.ctrl && event.metaKey) return false;
  if (!parsed.shift && event.shiftKey) return false;
  if (!parsed.alt && event.altKey) return false;

  // Match the primary key
  const eventKey = event.key.toLowerCase();
  if (parsed.key === "enter") return eventKey === "enter";
  if (parsed.key === "escape") return eventKey === "escape";
  if (parsed.key === "space") return eventKey === " " || eventKey === "space";
  if (parsed.key === "tab") return eventKey === "tab";

  return eventKey === parsed.key;
}

/**
 * Find the first command whose shortcut matches the given event.
 * Returns `undefined` if no match is found.
 */
export function findCommandByShortcut(
  event: KeyboardEvent,
  cmds: Command[],
): Command | undefined {
  for (const cmd of cmds) {
    if (!cmd.shortcut) continue;
    const parsed = parseShortcut(cmd.shortcut);
    if (matchesShortcut(event, parsed)) {
      return cmd;
    }
  }
  return undefined;
}

// ---------------------------------------------------------------------------
// Fuzzy Search
// ---------------------------------------------------------------------------

interface ScoredCommand {
  command: Command;
  score: number;
}

/**
 * Fuzzy-search commands by matching the query characters sequentially
 * against the title and category.  Returns matching commands sorted
 * by relevance (lower score = better match).
 *
 * Falls back to substring matching if no fuzzy matches are found.
 */
export function fuzzySearchCommands(
  query: string,
  cmds: Command[],
): Command[] {
  if (!query.trim()) return cmds;

  const lowerQuery = query.toLowerCase();
  const scored: ScoredCommand[] = [];

  for (const cmd of cmds) {
    const target = `${cmd.title} ${cmd.category}`.toLowerCase();

    // Fuzzy match: characters must appear in order
    let qi = 0;
    let score = 0;
    let lastMatchPos = -1;
    for (let ti = 0; ti < target.length && qi < lowerQuery.length; ti++) {
      if (target[ti] === lowerQuery[qi]) {
        // Bonus for consecutive matches (gap penalty)
        const gap = lastMatchPos >= 0 ? ti - lastMatchPos - 1 : 0;
        score += gap;
        lastMatchPos = ti;
        qi++;
      }
    }

    if (qi === lowerQuery.length) {
      // All characters matched - add word-boundary bonus
      const titleLower = cmd.title.toLowerCase();
      const startsWithBonus = titleLower.startsWith(lowerQuery) ? -100 : 0;
      scored.push({ command: cmd, score: score + startsWithBonus });
    }
  }

  if (scored.length > 0) {
    scored.sort((a, b) => a.score - b.score);
    return scored.map((s) => s.command);
  }

  // Fallback: simple substring search on title or category
  return cmds.filter((cmd) => {
    const target = `${cmd.title} ${cmd.category}`.toLowerCase();
    return target.includes(lowerQuery);
  });
}

// ---------------------------------------------------------------------------
// Palette State (open/close)
// ---------------------------------------------------------------------------

/**
 * Reactive boolean controlling the Command Palette visibility.
 * Managed here so that both the keybind handler and the palette
 * component share the same state without circular imports.
 */
export const paletteOpen = writable<boolean>(false);

/** Toggle the Command Palette visibility. */
export function togglePalette(): void {
  paletteOpen.update((v) => !v);
}

/** Open the Command Palette. */
export function openPalette(): void {
  paletteOpen.set(true);
}

/** Close the Command Palette. */
export function closePalette(): void {
  paletteOpen.set(false);
}
