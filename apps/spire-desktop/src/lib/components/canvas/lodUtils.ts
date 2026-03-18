/**
 * SPIRE - Level-of-Detail Utilities
 *
 * Shared types, constants, and helper functions for the canvas LOD system.
 *
 * The LOD architecture renders each widget at one of three detail tiers
 * depending on the current canvas zoom level:
 *
 *   • full    (zoom UI ≥ 45%)  — Standard interactive component
 *   • summary (25% ≤ zoom UI < 45%) — Simplified card with title + key readouts
 *   • minimal (zoom UI < 25%)  — Dense title-only rectangle
 *
 * Additionally, widgets that fall entirely outside the visible viewport
 * (plus a configurable overscan margin) are not mounted at all, replaced
 * by lightweight placeholder divs to maintain layout geometry.
 */

import type { WidgetType } from "$lib/stores/notebookStore";

// ---------------------------------------------------------------------------
// LOD Tiers
// ---------------------------------------------------------------------------

export type LodLevel = "full" | "summary" | "minimal";

/**
 * Zoom thresholds for LOD transitions, expressed in the SAME unit as UI zoom
 * indicator (percent, lower bound inclusive).
 */
export const LOD_THRESHOLD_FULL_UI_PERCENT = 25;
export const LOD_THRESHOLD_SUMMARY_UI_PERCENT = 45;

/** Internal zoom-factor thresholds derived from UI percent values. */
export const LOD_THRESHOLD_FULL = LOD_THRESHOLD_FULL_UI_PERCENT / 100;
export const LOD_THRESHOLD_SUMMARY = LOD_THRESHOLD_SUMMARY_UI_PERCENT / 100;

/** Convert zoom factor to the exact UI value shown to users (percent). */
export function zoomToUiPercent(zoom: number): number {
  return Math.round(zoom * 100);
}

/** Derive the current LOD level from the canvas zoom factor. */
export function zoomToLod(zoom: number): LodLevel {
  const uiPercent = zoomToUiPercent(zoom);
  // Keep transitions robust even when thresholds are edited in reverse order.
  const fullThreshold = Math.max(LOD_THRESHOLD_FULL_UI_PERCENT, LOD_THRESHOLD_SUMMARY_UI_PERCENT);
  const summaryThreshold = Math.min(LOD_THRESHOLD_FULL_UI_PERCENT, LOD_THRESHOLD_SUMMARY_UI_PERCENT);

  if (uiPercent >= fullThreshold) return "full";
  if (uiPercent >= summaryThreshold) return "summary";
  return "minimal";
}

// ---------------------------------------------------------------------------
// Frustum Culling (Viewport Intersection)
// ---------------------------------------------------------------------------

/** Fraction of the viewport dimensions added as overscan margin. */
export const FRUSTUM_OVERSCAN = 0.1;

export interface ViewportRect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/**
 * Compute the visible world-space rectangle from the canvas viewport state.
 *
 * @param panX    Horizontal pan offset (screen pixels)
 * @param panY    Vertical pan offset (screen pixels)
 * @param zoom    Zoom multiplier
 * @param screenW Viewport width in screen pixels
 * @param screenH Viewport height in screen pixels
 */
export function computeViewport(
  panX: number,
  panY: number,
  zoom: number,
  screenW: number,
  screenH: number,
): ViewportRect {
  const left   = -panX / zoom;
  const top    = -panY / zoom;
  const right  = left + screenW / zoom;
  const bottom = top + screenH / zoom;
  return { left, top, right, bottom };
}

/**
 * Expand a viewport rectangle by the overscan margin.
 *
 * The overscan prevents widgets from popping in/out during fast panning.
 */
export function expandByOverscan(vp: ViewportRect): ViewportRect {
  const w = vp.right - vp.left;
  const h = vp.bottom - vp.top;
  const ox = w * FRUSTUM_OVERSCAN;
  const oy = h * FRUSTUM_OVERSCAN;
  return {
    left:   vp.left   - ox,
    top:    vp.top    - oy,
    right:  vp.right  + ox,
    bottom: vp.bottom + oy,
  };
}

/**
 * AABB intersection test.  Returns `true` if the widget's world-space
 * bounding box overlaps the (expanded) viewport rectangle.
 */
export function isVisible(
  widgetX: number,
  widgetY: number,
  widgetW: number,
  widgetH: number,
  viewport: ViewportRect,
): boolean {
  return !(
    widgetX + widgetW < viewport.left  ||
    widgetX           > viewport.right ||
    widgetY + widgetH < viewport.top   ||
    widgetY           > viewport.bottom
  );
}

// ---------------------------------------------------------------------------
// Widget Icons (for minimal LOD cards)
// ---------------------------------------------------------------------------

// Shared SVG wrapper attributes: 16×16 viewBox, stroke-based, currentColor.
const S = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round">';
const E = '</svg>';

/** Inline SVG icon for each widget type, used in collapsed minimal/summary cards. */
export const WIDGET_ICONS: Record<WidgetType, string> = {
  // Cube — model / particle definition
  model:
    `${S}<path d="M8 1.5 L14 4.5 L14 11.5 L8 14.5 L2 11.5 L2 4.5 Z" /><path d="M8 14.5 L8 7.5" /><path d="M14 4.5 L8 7.5 L2 4.5" />${E}`,

  // Atom — reaction / process
  reaction:
    `${S}<circle cx="8" cy="8" r="2" /><ellipse cx="8" cy="8" rx="7" ry="3" /><ellipse cx="8" cy="8" rx="7" ry="3" transform="rotate(60 8 8)" /><ellipse cx="8" cy="8" rx="7" ry="3" transform="rotate(120 8 8)" />${E}`,

  // Diamond / graph — Feynman diagram
  diagram:
    `${S}<path d="M8 1 L15 8 L8 15 L1 8 Z" /><line x1="4" y1="4" x2="12" y2="12" /><line x1="12" y1="4" x2="4" y2="12" />${E}`,

  // Sigma / summation — amplitude calculation
  amplitude:
    `${S}<path d="M12 2 L4 2 L9 8 L4 14 L12 14" />${E}`,

  // Momentum arrow — kinematics
  kinematics:
    `${S}<line x1="2" y1="14" x2="14" y2="2" /><polyline points="8,2 14,2 14,8" /><line x1="2" y1="2" x2="2" y2="14" /><line x1="2" y1="14" x2="14" y2="14" />${E}`,

  // Triangle — Dalitz plot
  dalitz:
    `${S}<path d="M8 2 L14 13 L2 13 Z" /><circle cx="8" cy="9.5" r="0.8" fill="currentColor" stroke="none" />${E}`,

  // Bar chart — analysis
  analysis:
    `${S}<rect x="2" y="8" width="3" height="6" rx="0.5" /><rect x="6.5" y="4" width="3" height="10" rx="0.5" /><rect x="11" y="6" width="3" height="8" rx="0.5" />${E}`,

  // 3D event — event display
  event_display:
    `${S}<circle cx="8" cy="8" r="6.5" /><path d="M3 5.5 Q8 8 13 5.5" /><path d="M3 10.5 Q8 8 13 10.5" /><line x1="8" y1="1.5" x2="8" y2="14.5" />${E}`,

  // Periodic tiles — particle atlas
  particle_atlas:
    `${S}<rect x="1.5" y="1.5" width="13" height="13" rx="1" /><line x1="1.5" y1="6" x2="14.5" y2="6" /><line x1="6" y1="1.5" x2="6" y2="14.5" /><text x="8" y="11" text-anchor="middle" font-size="5" fill="currentColor" stroke="none">P</text>${E}`,

  // Pen nib — diagram editor
  diagram_editor:
    `${S}<path d="M11.5 1.5 L14.5 4.5 L5 14 L1.5 14.5 L2 11 Z" /><line x1="9.5" y1="3.5" x2="12.5" y2="6.5" />${E}`,

  // Lagrangian L script — lagrangian workbench
  lagrangian_workbench:
    `${S}<path d="M4 2 L4 12 Q4 14 6 14 L12 14" /><line x1="2" y1="8" x2="7" y2="8" />${E}`,

  // Import/export arrows — external models
  external_models:
    `${S}<line x1="5" y1="2" x2="5" y2="14" /><polyline points="2,5 5,2 8,5" /><line x1="11" y1="2" x2="11" y2="14" /><polyline points="8,11 11,14 14,11" />${E}`,

  // Grid — compute grid
  compute_grid:
    `${S}<rect x="1.5" y="1.5" width="13" height="13" rx="1" /><line x1="5.5" y1="1.5" x2="5.5" y2="14.5" /><line x1="10.5" y1="1.5" x2="10.5" y2="14.5" /><line x1="1.5" y1="5.5" x2="14.5" y2="5.5" /><line x1="1.5" y1="10.5" x2="14.5" y2="10.5" />${E}`,

  // Book — references
  references:
    `${S}<path d="M2 2 Q2 1 4 1 L12 1 Q14 1 14 2 L14 13 Q14 14 12 14 L4 14 Q2 14 2 13 Z" /><line x1="5" y1="1" x2="5" y2="14" /><line x1="7" y1="5" x2="12" y2="5" /><line x1="7" y1="8" x2="11" y2="8" />${E}`,

  // Antenna — telemetry
  telemetry:
    `${S}<line x1="8" y1="6" x2="8" y2="15" /><circle cx="8" cy="4" r="1.5" /><path d="M4 2 Q8 -1 12 2" /><path d="M2 1 Q8 -3 14 1" />${E}`,

  // Terminal lines — log
  log:
    `${S}<rect x="1.5" y="2" width="13" height="12" rx="1.5" /><line x1="4" y1="5.5" x2="7" y2="5.5" /><line x1="4" y1="8" x2="10" y2="8" /><line x1="4" y1="10.5" x2="8" y2="10.5" />${E}`,

  // Notebook — notebook
  notebook:
    `${S}<rect x="3" y="1.5" width="11" height="13" rx="1" /><line x1="6" y1="1.5" x2="6" y2="14.5" /><line x1="1" y1="4" x2="3" y2="4" /><line x1="1" y1="7" x2="3" y2="7" /><line x1="1" y1="10" x2="3" y2="10" />${E}`,

  // Sweep / scanner — parameter scanner
  parameter_scanner:
    `${S}<line x1="2" y1="14" x2="14" y2="14" /><line x1="2" y1="2" x2="2" y2="14" /><polyline points="3,11 6,7 9,9 12,3" /><circle cx="12" cy="3" r="1.2" fill="currentColor" stroke="none" />${E}`,

  // Branching arrow — decay calculator
  decay_calculator:
    `${S}<circle cx="4" cy="8" r="2" /><line x1="6" y1="8" x2="10" y2="8" /><line x1="10" y1="8" x2="14" y2="4" /><line x1="10" y1="8" x2="14" y2="12" /><circle cx="14" cy="4" r="1" fill="currentColor" stroke="none" /><circle cx="14" cy="12" r="1" fill="currentColor" stroke="none" />${E}`,

  // Globe — cosmology
  cosmology:
    `${S}<circle cx="8" cy="8" r="6.5" /><ellipse cx="8" cy="8" rx="3" ry="6.5" /><path d="M2 5.5 L14 5.5" /><path d="M2 10.5 L14 10.5" />${E}`,

  // Nu / neutrino oscillation — flavor workbench
  flavor_workbench:
    `${S}<path d="M2 12 Q5 2 8 8 Q11 14 14 4" /><circle cx="2" cy="12" r="1.2" fill="currentColor" stroke="none" /><circle cx="14" cy="4" r="1.2" fill="currentColor" stroke="none" />${E}`,

  // Gear / cog — plugin manager
  plugin_manager:
    `${S}<circle cx="8" cy="8" r="2.5" /><path d="M8 1 L8 3 M8 13 L8 15 M1 8 L3 8 M13 8 L15 8 M3.1 3.1 L4.5 4.5 M11.5 11.5 L12.9 12.9 M12.9 3.1 L11.5 4.5 M4.5 11.5 L3.1 12.9" />${E}`,

  // Cross-hair / target — global fit dashboard
  global_fit_dashboard:
    `${S}<circle cx="8" cy="8" r="6" /><circle cx="8" cy="8" r="2.5" /><line x1="8" y1="1" x2="8" y2="4" /><line x1="8" y1="12" x2="8" y2="15" /><line x1="1" y1="8" x2="4" y2="8" /><line x1="12" y1="8" x2="15" y2="8" />${E}`,
};

// ---------------------------------------------------------------------------
// Widget Accent Colours (for minimal LOD card left-border tinting)
// ---------------------------------------------------------------------------

/** Accent colour for each widget type's minimal card. */
export const WIDGET_ACCENT: Record<WidgetType, string> = {
  model:                 "#2ecc71",
  reaction:              "#5eb8ff",
  diagram:               "#d4a017",
  amplitude:             "#9b59b6",
  kinematics:            "#e67e22",
  dalitz:                "#1abc9c",
  analysis:              "#3498db",
  event_display:         "#e74c3c",
  particle_atlas:        "#5eb8ff",
  diagram_editor:        "#f39c12",
  lagrangian_workbench:  "#8e44ad",
  external_models:       "#2980b9",
  compute_grid:          "#27ae60",
  references:            "#95a5a6",
  telemetry:             "#e74c3c",
  log:                   "#7f8c8d",
  notebook:              "#f1c40f",
  parameter_scanner:     "#e67e22",
  decay_calculator:      "#1abc9c",
  cosmology:             "#2c3e50",
  flavor_workbench:      "#9b59b6",
  plugin_manager:        "#34495e",
  global_fit_dashboard:  "#3498db",
};
