/**
 * SPIRE — Store Synchronisation Service
 *
 * Bridges the Svelte store layer with the Tauri event system so that
 * tear-off windows share the same physics state as the main workspace.
 *
 * ## Architecture
 *
 * - **Main window** is the state authority.  It calls
 *   `initMainWindowSync()` once at mount, which:
 *     1. Subscribes to every physics store; on change, emits
 *        `spire://store-sync` to all tear-off windows (debounced).
 *     2. Listens for `spire://store-action` from tear-offs.
 *        Currently handles `"request-sync"` (send full snapshot) and
 *        `"store-update"` (apply a partial snapshot from a tear-off
 *        that performed a Tauri IPC call locally).
 *
 * - **Tear-off window** calls `initTearOffSync()` once at mount,
 *   which:
 *     1. Listens for `spire://store-sync` and hydrates local stores.
 *     2. Subscribes to local store changes and forwards them back to
 *        the main window via `spire://store-action` → `"store-update"`.
 *     3. Sends an initial `"request-sync"` so it receives state
 *        immediately after opening.
 *
 * A `_hydrating` guard prevents feedback loops: when a window is
 * applying a payload it received from another window, its own store
 * subscriptions are suppressed so they don't re-broadcast the same
 * data back.
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
import { get } from "svelte/store";
import type {
  Reaction,
  ReconstructedFinalState,
  TopologySet,
  AmplitudeResult,
  MandelstamVars as MandelstamVarsType,
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
  mandelstamVars: MandelstamVarsType | null;
  logs: string[];
  observableScripts: ObservableScript[];
  cutScripts: CutScript[];
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** True while we are applying a received payload — suppresses re-broadcasts. */
let _hydrating = false;

const ALL_STORES = [
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
] as const;

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

function hydrateFromPayload(payload: unknown): void {
  if (!payload || typeof payload !== "object") return;
  const snap = payload as Partial<StoreSnapshot>;

  _hydrating = true;
  try {
    if ("theoreticalModel" in snap)    theoreticalModel.set(snap.theoreticalModel ?? null);
    if ("activeFramework" in snap)     activeFramework.set(snap.activeFramework ?? "StandardModel");
    if ("activeReaction" in snap)      activeReaction.set(snap.activeReaction ?? null);
    if ("reconstructedStates" in snap) reconstructedStates.set(snap.reconstructedStates ?? []);
    if ("generatedDiagrams" in snap)   generatedDiagrams.set(snap.generatedDiagrams ?? null);
    if ("amplitudeResults" in snap)    amplitudeResults.set(snap.amplitudeResults ?? []);
    if ("activeAmplitude" in snap)     activeAmplitude.set(snap.activeAmplitude ?? "");
    if ("kinematics" in snap)          kinematics.set(snap.kinematics ?? null);
    if ("mandelstamVars" in snap)      mandelstamVars.set(snap.mandelstamVars ?? null);
    if ("logs" in snap)                logs.set(snap.logs ?? []);
    if ("observableScripts" in snap)   observableScripts.set(snap.observableScripts ?? []);
    if ("cutScripts" in snap)          cutScripts.set(snap.cutScripts ?? []);
  } finally {
    _hydrating = false;
  }
}

/** Whether running inside Tauri. */
function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI__" in window;
}

// ---------------------------------------------------------------------------
// Main Window Sync
// ---------------------------------------------------------------------------

/**
 * Initialise store synchronisation for the **main window**.
 *
 * - Broadcasts store snapshots to tear-off windows on change.
 * - Handles `request-sync` and `store-update` actions from tear-offs.
 *
 * @returns Cleanup function for `onDestroy`.
 */
export async function initMainWindowSync(): Promise<() => void> {
  if (!isTauri()) return () => {};

  const cleanups: (() => void)[] = [];

  try {
    const { emit, listen } = await import("@tauri-apps/api/event");

    // ── Broadcast store changes to tear-offs ──
    let debounceTimer: ReturnType<typeof setTimeout> | null = null;

    function debouncedBroadcast(): void {
      if (_hydrating) return; // don't re-broadcast received data
      if (debounceTimer !== null) clearTimeout(debounceTimer);
      debounceTimer = setTimeout(() => {
        debounceTimer = null;
        emit("spire://store-sync", buildSnapshot());
      }, 50);
    }

    const unsubs = ALL_STORES.map((store) =>
      store.subscribe(() => debouncedBroadcast()),
    );
    cleanups.push(() => {
      unsubs.forEach((u) => u());
      if (debounceTimer !== null) clearTimeout(debounceTimer);
    });

    // ── Listen for actions from tear-off windows ──
    const unlistenActions = await listen<{ action: string; payload: unknown }>(
      "spire://store-action",
      (event) => {
        const { action, payload } = event.payload;
        if (action === "request-sync") {
          // Tear-off just opened — send it the current state immediately
          emit("spire://store-sync", buildSnapshot());
        } else if (action === "store-update") {
          // Tear-off performed a local mutation — apply to main stores
          hydrateFromPayload(payload);
        }
      },
    );
    cleanups.push(unlistenActions);
  } catch {
    // Not in Tauri — no-op
  }

  return () => cleanups.forEach((fn) => fn());
}

// ---------------------------------------------------------------------------
// Tear-Off Window Sync
// ---------------------------------------------------------------------------

/**
 * Initialise store synchronisation for a **tear-off window**.
 *
 * - Listens for `spire://store-sync` and hydrates local stores.
 * - Forwards local store changes back to the main window.
 * - Sends an initial `request-sync` so it gets state immediately.
 *
 * @returns Cleanup function for `onDestroy`.
 */
export async function initTearOffSync(): Promise<() => void> {
  if (!isTauri()) return () => {};

  const cleanups: (() => void)[] = [];

  try {
    const { emit, listen } = await import("@tauri-apps/api/event");

    // ── Receive state from main window ──
    const unlistenSync = await listen<unknown>(
      "spire://store-sync",
      (event) => {
        hydrateFromPayload(event.payload);
      },
    );
    cleanups.push(unlistenSync);

    // ── Forward local store changes back to main ──
    let debounceTimer: ReturnType<typeof setTimeout> | null = null;

    function debouncedForward(): void {
      if (_hydrating) return; // don't echo back what we just received
      if (debounceTimer !== null) clearTimeout(debounceTimer);
      debounceTimer = setTimeout(() => {
        debounceTimer = null;
        emit("spire://store-action", {
          action: "store-update",
          payload: buildSnapshot(),
        });
      }, 80); // slightly longer debounce than main to avoid races
    }

    const unsubs = ALL_STORES.map((store) =>
      store.subscribe(() => debouncedForward()),
    );
    cleanups.push(() => {
      unsubs.forEach((u) => u());
      if (debounceTimer !== null) clearTimeout(debounceTimer);
    });

    // ── Request initial state from main window ──
    await emit("spire://store-action", {
      action: "request-sync",
      payload: null,
    });
  } catch {
    // Not in Tauri — no-op
  }

  return () => cleanups.forEach((fn) => fn());
}
