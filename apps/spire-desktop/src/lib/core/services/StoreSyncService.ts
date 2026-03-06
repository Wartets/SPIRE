/**
 * SPIRE — Store Synchronisation Service
 *
 * Bridges the Svelte store layer with the Tauri event system so that
 * tear-off windows share the same physics state as the main workspace.
 *
 * ## Main Window
 *
 * Call `startBroadcasting()` once at mount.  It subscribes to every
 * physics store and, on change, emits a `spire://store-sync` event
 * carrying a JSON snapshot of the full state.  The broadcast is
 * debounced (50 ms) so rapid store mutations don't flood the IPC bus.
 *
 * ## Tear-Off Window
 *
 * Call `hydrateFromPayload(payload)` inside the `listenForStoreSync`
 * callback.  It parses the snapshot and writes each field back into
 * the corresponding Svelte store.
 */

import {
  theoreticalModel,
  activeFramework,
  activeReaction,
  reconstructedStates,
  generatedDiagrams,
  amplitudeResults,
  activeAmplitude,
  kinematics,
  mandelstamVars,
  logs,
  observableScripts,
  cutScripts,
} from "$lib/stores/physicsStore";
import { broadcastStoreSync } from "$lib/core/services/WindowManager";
import { get } from "svelte/store";
import type {
  Reaction,
  ReconstructedFinalState,
  TopologySet,
  AmplitudeResult,
  MandelstamVars,
  KinematicsReport,
  TheoreticalFramework,
  TheoreticalModel,
  ObservableScript,
  CutScript,
} from "$lib/types/spire";

// ---------------------------------------------------------------------------
// Snapshot Shape
// ---------------------------------------------------------------------------

export interface StoreSnapshot {
  theoreticalModel: TheoreticalModel | null;
  activeFramework: TheoreticalFramework;
  activeReaction: Reaction | null;
  reconstructedStates: ReconstructedFinalState[];
  generatedDiagrams: TopologySet | null;
  amplitudeResults: AmplitudeResult[];
  activeAmplitude: string;
  kinematics: KinematicsReport | null;
  mandelstamVars: MandelstamVars | null;
  logs: string[];
  observableScripts: ObservableScript[];
  cutScripts: CutScript[];
}

// ---------------------------------------------------------------------------
// Main Window — Broadcast
// ---------------------------------------------------------------------------

/** Unsubscribe functions returned by Svelte store `.subscribe()`. */
let _unsubscribers: (() => void)[] = [];
let _debounceTimer: ReturnType<typeof setTimeout> | null = null;

export function buildSnapshot(): StoreSnapshot {
  return {
    theoreticalModel: get(theoreticalModel),
    activeFramework: get(activeFramework),
    activeReaction: get(activeReaction),
    reconstructedStates: get(reconstructedStates),
    generatedDiagrams: get(generatedDiagrams),
    amplitudeResults: get(amplitudeResults),
    activeAmplitude: get(activeAmplitude),
    kinematics: get(kinematics),
    mandelstamVars: get(mandelstamVars),
    logs: get(logs),
    observableScripts: get(observableScripts),
    cutScripts: get(cutScripts),
  };
}

function debouncedBroadcast(): void {
  if (_debounceTimer !== null) clearTimeout(_debounceTimer);
  _debounceTimer = setTimeout(() => {
    _debounceTimer = null;
    broadcastStoreSync(buildSnapshot());
  }, 50);
}

/**
 * Start broadcasting physics-store changes to all tear-off windows.
 * Call once from the main window's root layout `onMount`.
 *
 * @returns A cleanup function to call in `onDestroy`.
 */
export function startBroadcasting(): () => void {
  // Subscribe to every physics store.
  const stores = [
    theoreticalModel,
    activeFramework,
    activeReaction,
    reconstructedStates,
    generatedDiagrams,
    amplitudeResults,
    activeAmplitude,
    kinematics,
    mandelstamVars,
    logs,
    observableScripts,
    cutScripts,
  ];

  _unsubscribers = stores.map((store) => store.subscribe(() => debouncedBroadcast()));

  return stopBroadcasting;
}

/** Stop broadcasting and release all subscriptions. */
export function stopBroadcasting(): void {
  for (const unsub of _unsubscribers) unsub();
  _unsubscribers = [];
  if (_debounceTimer !== null) {
    clearTimeout(_debounceTimer);
    _debounceTimer = null;
  }
}

// ---------------------------------------------------------------------------
// Tear-Off Window — Hydrate
// ---------------------------------------------------------------------------

/**
 * Apply a state snapshot received from the main window to local stores.
 * Call this from the `listenForStoreSync` callback in the tear-off page.
 */
export function hydrateFromPayload(payload: unknown): void {
  if (!payload || typeof payload !== "object") return;

  const snap = payload as Partial<StoreSnapshot>;

  if ("theoreticalModel" in snap)  theoreticalModel.set(snap.theoreticalModel ?? null);
  if ("activeFramework" in snap)   activeFramework.set(snap.activeFramework ?? "StandardModel");
  if ("activeReaction" in snap)    activeReaction.set(snap.activeReaction ?? null);
  if ("reconstructedStates" in snap) reconstructedStates.set(snap.reconstructedStates ?? []);
  if ("generatedDiagrams" in snap) generatedDiagrams.set(snap.generatedDiagrams ?? null);
  if ("amplitudeResults" in snap)  amplitudeResults.set(snap.amplitudeResults ?? []);
  if ("activeAmplitude" in snap)   activeAmplitude.set(snap.activeAmplitude ?? "");
  if ("kinematics" in snap)        kinematics.set(snap.kinematics ?? null);
  if ("mandelstamVars" in snap)    mandelstamVars.set(snap.mandelstamVars ?? null);
  if ("logs" in snap)              logs.set(snap.logs ?? []);
  if ("observableScripts" in snap) observableScripts.set(snap.observableScripts ?? []);
  if ("cutScripts" in snap)        cutScripts.set(snap.cutScripts ?? []);
}
