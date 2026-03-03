/**
 * SPIRE — Widget Registry
 *
 * Maps each WidgetType to its human-readable label.
 * The actual Svelte component mapping is handled in the
 * WidgetCell.svelte template via {#if} blocks, because
 * SvelteKit's dynamic `<svelte:component>` needs static
 * imports at build time for type safety.
 */

import type { WidgetType } from "$lib/stores/notebookStore";

/** Human-readable labels for each widget type. */
export const WIDGET_LABELS: Record<WidgetType, string> = {
  model:      "Model Loader",
  reaction:   "Reaction Workspace",
  diagram:    "Diagram Visualizer",
  amplitude:  "Amplitude Panel",
  kinematics: "Kinematics",
  dalitz:     "Dalitz Plot",
  analysis:      "Analysis",
  event_display: "Event Display",
  log:           "Console",
};
