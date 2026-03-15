/**
 * SPIRE – Selection Bus
 *
 * A global reactive store that broadcasts item-selection events originating
 * from any atlas-type component (Particle Atlas, Periodic Table, Isotope
 * Drawer) to any listening consumer (Reaction Builder, Lagrangian Workbench,
 * Kinematics Explorer, etc.).
 *
 * Consumers subscribe to `selectionBus` and react to payloads by type:
 *
 *   import { selectionBus } from "$lib/stores/selectionBus";
 *   $: if ($selectionBus?.type === "PARTICLE_SELECTED") { ... }
 *
 * The bus is write-once-then-consume by convention: after processing a
 * payload, consumers call `clearSelectionBus()` to signal completion.
 */

import { writable } from "svelte/store";
import type { Field } from "$lib/types/spire";
import type { ElementData, IsotopeData } from "$lib/core/physics/nuclearDataLoader";

// ---------------------------------------------------------------------------
// Payload Types
// ---------------------------------------------------------------------------

export interface ParticleSelectedPayload {
  type: "PARTICLE_SELECTED";
  data: Field;
}

export interface ElementSelectedPayload {
  type: "ELEMENT_SELECTED";
  data: ElementData;
}

export interface IsotopeSelectedPayload {
  type: "ISOTOPE_SELECTED";
  data: {
    Z: number;
    A: number;
    symbol: string;
    name: string;
    isotope: IsotopeData;
  };
}

export type SelectionBusPayload =
  | ParticleSelectedPayload
  | ElementSelectedPayload
  | IsotopeSelectedPayload;

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

/** The active selection broadcast.  Null when no selection is in flight. */
export const selectionBus = writable<SelectionBusPayload | null>(null);

/**
 * Broadcast a selection payload to all subscribers.
 *
 * @param payload – The typed selection event to dispatch.
 */
export function broadcastSelection(payload: SelectionBusPayload): void {
  selectionBus.set(payload);
}

/**
 * Clear the current broadcast.  Consumers should call this after they have
 * processed the payload to prevent re-processing on re-render.
 */
export function clearSelectionBus(): void {
  selectionBus.set(null);
}
