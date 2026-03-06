/**
 * Telemetry Service - Performance Profiling Store
 *
 * Provides reactive Svelte stores for tracking kernel computation
 * performance. Receives ComputeProfile payloads from the backend
 * (attached to CrossSectionResult, AnalysisResult, TopologySet,
 * AmplitudeResult) and makes them available for the TelemetryPanel
 * dashboard widget.
 *
 * ## Architecture
 *
 * - `latestProfile` - The most recent ComputeProfile from any computation.
 * - `historicalProfiles` - Rolling buffer of the last 20 profiles.
 * - `convergenceData` - Extracted from the latest profile for MC convergence plots.
 *
 * The service is intentionally decoupled from the backend transport.
 * Any component that receives a computation result with a profile can
 * push it via `pushProfile(profile, label)`.
 */

import { writable, derived, type Readable } from "svelte/store";
import type { ComputeProfile } from "$lib/types/spire";

// ===========================================================================
// Types
// ===========================================================================

/** A historical profile entry with timestamp and source label. */
export interface ProfileEntry {
  /** The performance profile data. */
  profile: ComputeProfile;
  /** Human-readable label for the computation (e.g., "Cross-Section: e⁺e⁻ → μ⁺μ⁻"). */
  label: string;
  /** ISO timestamp when the profile was recorded. */
  timestamp: string;
}

/** A single convergence data point for plotting. */
export interface ConvergencePoint {
  /** Number of events evaluated at this snapshot. */
  events: number;
  /** Relative error at this snapshot. */
  error: number;
}

// ===========================================================================
// Stores
// ===========================================================================

/** Maximum number of historical profiles to retain. */
const MAX_HISTORY = 20;

/** The most recent ComputeProfile from any computation pipeline. */
const _latestEntry = writable<ProfileEntry | null>(null);

/** Rolling buffer of historical profile entries. */
const _history = writable<ProfileEntry[]>([]);

// ===========================================================================
// Derived Stores
// ===========================================================================

/** The most recent ComputeProfile (or null if none recorded). */
export const latestProfile: Readable<ComputeProfile | null> = derived(
  _latestEntry,
  ($entry) => $entry?.profile ?? null,
);

/** Label of the latest computation. */
export const latestLabel: Readable<string> = derived(
  _latestEntry,
  ($entry) => $entry?.label ?? "",
);

/** All historical profile entries (most recent first). */
export const historicalProfiles: Readable<ProfileEntry[]> = derived(
  _history,
  ($h) => $h,
);

/** Number of stored historical profiles. */
export const profileCount: Readable<number> = derived(
  _history,
  ($h) => $h.length,
);

/**
 * Convergence data from the latest profile, formatted for plotting.
 * Empty array if no convergence data is available.
 */
export const convergenceData: Readable<ConvergencePoint[]> = derived(
  _latestEntry,
  ($entry) => {
    if (!$entry?.profile.convergence_data) return [];
    return $entry.profile.convergence_data.map(([events, error]) => ({
      events,
      error,
    }));
  },
);

/**
 * Stage timings from the latest profile as sorted [name, ms] pairs.
 * Sorted by descending time for the bar chart rendering.
 */
export const stageTimings: Readable<[string, number][]> = derived(
  _latestEntry,
  ($entry) => {
    if (!$entry?.profile.stage_timings) return [];
    return Object.entries($entry.profile.stage_timings).sort(
      ([, a], [, b]) => b - a,
    );
  },
);

// ===========================================================================
// Public API
// ===========================================================================

/**
 * Push a new ComputeProfile into the telemetry store.
 *
 * Called by components or API interceptors whenever a computation
 * result includes a profile payload.
 *
 * @param profile - The ComputeProfile from the kernel.
 * @param label - Human-readable description of the computation.
 */
export function pushProfile(profile: ComputeProfile, label: string): void {
  const entry: ProfileEntry = {
    profile,
    label,
    timestamp: new Date().toISOString(),
  };

  _latestEntry.set(entry);

  _history.update((history) => {
    const updated = [entry, ...history];
    if (updated.length > MAX_HISTORY) {
      updated.length = MAX_HISTORY;
    }
    return updated;
  });
}

/**
 * Clear all telemetry data (latest + history).
 */
export function clearTelemetry(): void {
  _latestEntry.set(null);
  _history.set([]);
}

/**
 * Extract and push a profile from a computation result object.
 *
 * Generic helper that checks for a `profile` field on any result
 * object and pushes it if present.
 *
 * @param result - Any computation result that may contain a `profile` field.
 * @param label - Human-readable label for the computation.
 * @returns true if a profile was found and pushed, false otherwise.
 */
export function extractAndPushProfile(
  result: { profile?: ComputeProfile | null },
  label: string,
): boolean {
  if (result.profile) {
    pushProfile(result.profile, label);
    return true;
  }
  return false;
}
