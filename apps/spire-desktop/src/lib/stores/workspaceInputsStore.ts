/**
 * SPIRE - Workspace Inputs Store
 *
 * Shared writable stores for user-configurable physics parameters.
 * These stores were promoted from component-local variables in
 * ModelLoader.svelte and ReactionWorkspace.svelte so that the
 * workspace persistence service can read and write them.
 *
 * Components bind directly to these stores (e.g. `$particlesTomlInput`)
 * for seamless two-way reactivity.
 */

import { writable, derived, get } from "svelte/store";
import {
  DEFAULT_PARTICLES_TOML,
  DEFAULT_VERTICES_TOML,
  DEFAULT_INITIAL_IDS,
  DEFAULT_FINAL_IDS,
  DEFAULT_CMS_ENERGY,
} from "$lib/data/defaults";

// ---------------------------------------------------------------------------
// Model Inputs
// ---------------------------------------------------------------------------

/** Raw TOML content defining particle fields. */
export const particlesTomlInput = writable<string>(DEFAULT_PARTICLES_TOML);

/** Raw TOML content defining interaction vertices. */
export const verticesTomlInput = writable<string>(DEFAULT_VERTICES_TOML);

/** Human-readable model name. */
export const modelNameInput = writable<string>("Standard Model");

// ---------------------------------------------------------------------------
// Reaction Inputs
// ---------------------------------------------------------------------------

/** Initial-state particle IDs (e.g. ["e-", "e+"]). */
export const initialIdsInput = writable<string[]>([...DEFAULT_INITIAL_IDS]);

/** Final-state particle IDs (e.g. ["mu-", "mu+"]). */
export const finalIdsInput = writable<string[]>([...DEFAULT_FINAL_IDS]);

/** Centre-of-mass energy in GeV. */
export const cmsEnergyInput = writable<number>(DEFAULT_CMS_ENERGY);

/** Maximum perturbative loop order. */
export const maxLoopOrderInput = writable<number>(0);

// ---------------------------------------------------------------------------
// Aggregate Snapshot (for persistence)
// ---------------------------------------------------------------------------

/** Combined snapshot of all workspace inputs (read-only derived). */
export const workspaceInputsSnapshot = derived(
  [
    particlesTomlInput,
    verticesTomlInput,
    modelNameInput,
    initialIdsInput,
    finalIdsInput,
    cmsEnergyInput,
    maxLoopOrderInput,
  ],
  ([
    $particlesToml,
    $verticesToml,
    $modelName,
    $initialIds,
    $finalIds,
    $cmsEnergy,
    $maxLoopOrder,
  ]) => ({
    particlesToml: $particlesToml,
    verticesToml: $verticesToml,
    modelName: $modelName,
    initialIds: $initialIds,
    finalIds: $finalIds,
    cmsEnergy: $cmsEnergy,
    maxLoopOrder: $maxLoopOrder,
  }),
);

// ---------------------------------------------------------------------------
// Bulk Operations (for workspace import / reset)
// ---------------------------------------------------------------------------

/** Set all input stores from a workspace physics snapshot. */
export function setAllInputs(inputs: {
  particlesToml: string;
  verticesToml: string;
  modelName: string;
  initialIds: string[];
  finalIds: string[];
  cmsEnergy: number;
  maxLoopOrder: number;
}): void {
  particlesTomlInput.set(inputs.particlesToml);
  verticesTomlInput.set(inputs.verticesToml);
  modelNameInput.set(inputs.modelName);
  initialIdsInput.set([...inputs.initialIds]);
  finalIdsInput.set([...inputs.finalIds]);
  cmsEnergyInput.set(inputs.cmsEnergy);
  maxLoopOrderInput.set(inputs.maxLoopOrder);
}

/** Reset all input stores to factory defaults. */
export function resetAllInputs(): void {
  particlesTomlInput.set(DEFAULT_PARTICLES_TOML);
  verticesTomlInput.set(DEFAULT_VERTICES_TOML);
  modelNameInput.set("Standard Model");
  initialIdsInput.set([...DEFAULT_INITIAL_IDS]);
  finalIdsInput.set([...DEFAULT_FINAL_IDS]);
  cmsEnergyInput.set(DEFAULT_CMS_ENERGY);
  maxLoopOrderInput.set(0);
}
