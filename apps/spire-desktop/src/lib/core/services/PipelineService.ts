/**
 * SPIRE - Pipeline Service
 *
 * Publish/subscribe event bus that wires widget outputs to widget
 * inputs.  Implements the "piping" paradigm where the output of one
 * widget (a "Source") automatically drives the input of another
 * (a "Sink").
 *
 * ## Architecture
 *
 * - Each widget may declare itself as a **Source** of a particular
 *   data type (e.g., "reaction", "model", "decay_table").
 * - Each widget may declare itself as a **Sink** that consumes data
 *   of a particular type.
 * - The user creates **Links** connecting a Source to a Sink.
 * - When a Source publishes new data via `publishData()`, all linked
 *   Sinks receive the updated payload via their registered callbacks.
 *
 * Links are stored in a Svelte writable store so the UI can render
 * connection indicators in widget headers.
 *
 * ## Data Flow
 *
 * ```
 *   [Model Loader] --model--> [Decay Calculator]
 *   [Reaction WS]  --reaction--> [Feynman Diagrams]
 *   [Feynman Diagrams] --diagram--> [Amplitude Panel]
 * ```
 */

import { writable, derived, get } from "svelte/store";

// ===========================================================================
// Types
// ===========================================================================

/** The kinds of data that can flow between widgets. */
export type PipelineDataType =
  | "model"
  | "reaction"
  | "diagram"
  | "amplitude"
  | "kinematics"
  | "decay_table"
  | "analysis_result";

/** A declared port on a widget (either source or sink). */
export interface PipelinePort {
  /** The widget instance ID. */
  widgetId: string;
  /** Human-readable label for the port. */
  label: string;
  /** The data type this port produces or consumes. */
  dataType: PipelineDataType;
}

/** A directional link connecting one source port to one sink port. */
export interface PipelineLink {
  /** Unique link identifier. */
  id: string;
  /** The producing port. */
  source: PipelinePort;
  /** The consuming port. */
  sink: PipelinePort;
}

/** Callback signature for sink subscribers. */
export type SinkCallback = (payload: unknown) => void;

// ===========================================================================
// Internal State
// ===========================================================================

/** Registry of declared source ports. */
const sourceRegistry = new Map<string, PipelinePort>();

/** Registry of declared sink ports with callbacks. */
const sinkRegistry = new Map<string, { port: PipelinePort; callback: SinkCallback }>();

/** ID counter for links. */
let _linkCounter = 0;

function makeLinkId(): string {
  _linkCounter += 1;
  return `pipe-${_linkCounter}-${Date.now().toString(36)}`;
}

// ===========================================================================
// Stores
// ===========================================================================

/** All active pipeline links. */
export const pipelineLinks = writable<PipelineLink[]>([]);

/** All registered source ports (reactive). */
export const registeredSources = writable<PipelinePort[]>([]);

/** All registered sink ports (reactive). */
export const registeredSinks = writable<PipelinePort[]>([]);

/** Count of active links. */
export const pipelineLinkCount = derived(pipelineLinks, ($links) => $links.length);

// ===========================================================================
// Source Registration
// ===========================================================================

/**
 * Register a widget as a data source.
 *
 * @param widgetId - The unique widget instance ID.
 * @param dataType - The type of data this widget produces.
 * @param label    - Human-readable port label.
 */
export function registerSource(
  widgetId: string,
  dataType: PipelineDataType,
  label: string,
): void {
  const port: PipelinePort = { widgetId, dataType, label };
  sourceRegistry.set(widgetId, port);
  registeredSources.update((prev) => [...prev.filter((p) => p.widgetId !== widgetId), port]);
}

/**
 * Unregister a source port (e.g., when a widget is closed).
 */
export function unregisterSource(widgetId: string): void {
  sourceRegistry.delete(widgetId);
  registeredSources.update((prev) => prev.filter((p) => p.widgetId !== widgetId));
  // Also remove any links from this source
  pipelineLinks.update((links) => links.filter((l) => l.source.widgetId !== widgetId));
}

// ===========================================================================
// Sink Registration
// ===========================================================================

/**
 * Register a widget as a data sink.
 *
 * @param widgetId - The unique widget instance ID.
 * @param dataType - The type of data this widget consumes.
 * @param label    - Human-readable port label.
 * @param callback - Function invoked when new data arrives.
 */
export function registerSink(
  widgetId: string,
  dataType: PipelineDataType,
  label: string,
  callback: SinkCallback,
): void {
  const port: PipelinePort = { widgetId, dataType, label };
  sinkRegistry.set(widgetId, { port, callback });
  registeredSinks.update((prev) => [...prev.filter((p) => p.widgetId !== widgetId), port]);
}

/**
 * Unregister a sink port (e.g., when a widget is closed).
 */
export function unregisterSink(widgetId: string): void {
  sinkRegistry.delete(widgetId);
  registeredSinks.update((prev) => prev.filter((p) => p.widgetId !== widgetId));
  // Also remove any links to this sink
  pipelineLinks.update((links) => links.filter((l) => l.sink.widgetId !== widgetId));
}

// ===========================================================================
// Link Management
// ===========================================================================

/**
 * Create a link between a source and a sink.
 *
 * @param sourceWidgetId - The widget ID of the source.
 * @param sinkWidgetId   - The widget ID of the sink.
 * @returns The created PipelineLink, or null if ports are incompatible.
 */
export function createLink(
  sourceWidgetId: string,
  sinkWidgetId: string,
): PipelineLink | null {
  const source = sourceRegistry.get(sourceWidgetId);
  const sinkEntry = sinkRegistry.get(sinkWidgetId);

  if (!source || !sinkEntry) return null;
  if (source.dataType !== sinkEntry.port.dataType) return null;

  // Prevent duplicate links
  const existing = get(pipelineLinks);
  const duplicate = existing.find(
    (l) => l.source.widgetId === sourceWidgetId && l.sink.widgetId === sinkWidgetId,
  );
  if (duplicate) return duplicate;

  const link: PipelineLink = {
    id: makeLinkId(),
    source,
    sink: sinkEntry.port,
  };

  pipelineLinks.update((links) => [...links, link]);
  return link;
}

/**
 * Remove a link by its ID.
 */
export function removeLink(linkId: string): void {
  pipelineLinks.update((links) => links.filter((l) => l.id !== linkId));
}

/**
 * Remove all links for a given widget (both source and sink).
 */
export function removeAllLinksForWidget(widgetId: string): void {
  pipelineLinks.update((links) =>
    links.filter(
      (l) => l.source.widgetId !== widgetId && l.sink.widgetId !== widgetId,
    ),
  );
}

// ===========================================================================
// Data Publishing
// ===========================================================================

/**
 * Publish data from a source widget. All linked sinks of the same
 * data type receive the payload via their registered callbacks.
 *
 * @param sourceWidgetId - The widget producing the data.
 * @param dataType       - The data type being published.
 * @param payload        - The actual data payload.
 */
export function publishData(
  sourceWidgetId: string,
  dataType: PipelineDataType,
  payload: unknown,
): void {
  const links = get(pipelineLinks);

  for (const link of links) {
    if (
      link.source.widgetId === sourceWidgetId &&
      link.source.dataType === dataType
    ) {
      const sinkEntry = sinkRegistry.get(link.sink.widgetId);
      if (sinkEntry && sinkEntry.port.dataType === dataType) {
        try {
          sinkEntry.callback(payload);
        } catch {
          // Sink callback failed - silently continue to other sinks
        }
      }
    }
  }
}

// ===========================================================================
// Query Helpers
// ===========================================================================

/**
 * Get all available sinks that are compatible with a given source.
 */
export function getCompatibleSinks(sourceWidgetId: string): PipelinePort[] {
  const source = sourceRegistry.get(sourceWidgetId);
  if (!source) return [];

  const allSinks = get(registeredSinks);
  return allSinks.filter(
    (s) => s.dataType === source.dataType && s.widgetId !== sourceWidgetId,
  );
}

/**
 * Get all links connected to a specific widget.
 */
export function getLinksForWidget(widgetId: string): PipelineLink[] {
  return get(pipelineLinks).filter(
    (l) => l.source.widgetId === widgetId || l.sink.widgetId === widgetId,
  );
}

/**
 * Check whether a widget has any active pipeline connections.
 */
export function isWidgetLinked(widgetId: string): boolean {
  return getLinksForWidget(widgetId).length > 0;
}
