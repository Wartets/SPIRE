/**
 * SPIRE - Glossary Service
 *
 * Data-driven registry of physics terms, constants, and concepts.
 * Each entry carries a human-readable definition, optional formula
 * (LaTeX), and an optional Particle Data Group (PDG) reference.
 *
 * The glossary is deliberately decoupled from UI components:
 *   - Core Standard Model terms are populated at import time.
 *   - BSM or UFO models can inject custom entries at runtime via
 *     `registerTerm()` / `registerTerms()`.
 *   - The `<HoverDef>` tooltip component queries this service by key.
 *
 * Keys follow a flat namespace: lowercase, underscores, no dots.
 * Example keys: "alpha_s", "m_z", "gamma_w", "ckm_matrix".
 */

import { writable, derived, get } from "svelte/store";
import glossarySeed from "$lib/data/glossary.json";

// ---------------------------------------------------------------------------
// Data Model
// ---------------------------------------------------------------------------

export interface GlossaryEntry {
  /** The canonical display name (e.g. "Strong Coupling Constant"). */
  term: string;
  /** Plain-text scientific definition (1–3 sentences). */
  definition: string;
  /** Optional LaTeX formula for the tooltip (e.g. "\\alpha_s(M_Z)"). */
  formula?: string;
  /** Optional current numerical value and units. */
  value?: string;
  /** Optional PDG reference string (e.g. "PDG 2024, §9.4"). */
  pdgRef?: string;
  /** Optional category for grouping (e.g. "Coupling", "Mass", "Concept"). */
  category?: string;
}

/**
 * Key-value glossary catalog shape used for bulk registration/loading.
 */
export type GlossaryCatalog = Record<string, GlossaryEntry>;

// ---------------------------------------------------------------------------
// Internal Store
// ---------------------------------------------------------------------------

const _glossaryMap = writable<Map<string, GlossaryEntry>>(new Map());

// ---------------------------------------------------------------------------
// Public Derived Stores
// ---------------------------------------------------------------------------

/**
 * All glossary entries as a read-only map.
 */
export const glossary = derived(_glossaryMap, ($map) => $map);

/**
 * Number of registered terms.
 */
export const glossarySize = derived(_glossaryMap, ($map) => $map.size);

// ---------------------------------------------------------------------------
// Registry API
// ---------------------------------------------------------------------------

/**
 * Register or replace a single glossary term.
 */
export function registerTerm(key: string, entry: GlossaryEntry): void {
  _glossaryMap.update((map) => {
    const next = new Map(map);
    next.set(key, entry);
    return next;
  });
}

/**
 * Register multiple glossary terms at once.
 */
export function registerTerms(entries: Record<string, GlossaryEntry>): void {
  _glossaryMap.update((map) => {
    const next = new Map(map);
    for (const [key, entry] of Object.entries(entries)) {
      next.set(key, entry);
    }
    return next;
  });
}

/**
 * Remove a glossary term by key.
 */
export function unregisterTerm(key: string): void {
  _glossaryMap.update((map) => {
    const next = new Map(map);
    next.delete(key);
    return next;
  });
}

/**
 * Look up a single glossary entry by key.
 * Returns `undefined` if the key is not registered.
 */
export function lookupTerm(key: string): GlossaryEntry | undefined {
  return get(_glossaryMap).get(key);
}

/**
 * List all terms sorted by key for deterministic UI rendering.
 */
export function listTerms(): Array<[string, GlossaryEntry]> {
  return Array.from(get(_glossaryMap).entries()).sort(([a], [b]) => a.localeCompare(b));
}

/**
 * Find terms by category label (case-insensitive).
 */
export function findTermsByCategory(category: string): Array<[string, GlossaryEntry]> {
  const needle = category.trim().toLowerCase();
  return listTerms().filter(([, entry]) => (entry.category ?? "").toLowerCase() === needle);
}

/**
 * Full-text search over key, term name, and definition.
 */
export function searchTerms(query: string): Array<[string, GlossaryEntry]> {
  const needle = query.trim().toLowerCase();
  if (!needle) return listTerms();
  return listTerms().filter(([key, entry]) => {
    return (
      key.toLowerCase().includes(needle)
      || entry.term.toLowerCase().includes(needle)
      || entry.definition.toLowerCase().includes(needle)
    );
  });
}

// ---------------------------------------------------------------------------
// Default Glossary Loading (JSON data source)
// ---------------------------------------------------------------------------

const DEFAULT_GLOSSARY = glossarySeed as GlossaryCatalog;

// Populate the glossary at module load time
registerTerms(DEFAULT_GLOSSARY);
