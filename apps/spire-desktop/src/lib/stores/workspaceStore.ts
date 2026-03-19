import { derived, get, writable } from "svelte/store";
import {
  activeWorkspaceId,
  canvasItems,
  canvasViewport,
  layoutRoot,
  saveCurrentWorkspaceState,
  viewMode,
  workspaces,
  type Workspace,
} from "$lib/stores/layoutStore";
import { notebookDocument } from "$lib/stores/notebookDocumentStore";
import {
  activeAmplitude,
  activeFramework,
  activeReaction,
  amplitudeResults,
  cutScripts,
  generatedDiagrams,
  kinematics,
  logs,
  observableScripts,
  reconstructedStates,
  theoreticalModel,
} from "$lib/stores/physicsStore";
import { workspaceInputsSnapshot, setAllInputs } from "$lib/stores/workspaceInputsStore";
import {
  DEFAULT_CMS_ENERGY,
  DEFAULT_FINAL_IDS,
  DEFAULT_INITIAL_IDS,
  DEFAULT_PARTICLES_TOML,
  DEFAULT_VERTICES_TOML,
} from "$lib/data/defaults";
import {
  loadWorkspaceState,
  saveWorkspaceState,
  storageSchemaVersion,
} from "$lib/core/storage/StorageManager";
import {
  runMigrationSelfCheck,
  type PersistedWorkspaceState,
  type PersistedWorkspaceRecord,
} from "$lib/core/storage/migrations";

/**
 * Workspace persistence debounce interval in milliseconds.
 *
 * A short debounce prevents excessive write amplification while users type,
 * drag, or resize rapidly, while still persisting quickly enough to reduce
 * data-loss risk on navigation/refresh.
 */
const AUTO_SAVE_DEBOUNCE_MS = 1000;

let persistenceInitialised = false;
let stopPersistenceWatches: Array<() => void> = [];
let saveTimer: ReturnType<typeof setTimeout> | null = null;

/**
 * True once the persisted state has been loaded (or load attempted).
 *
 * UI components can subscribe to gate rendering of hydration-sensitive views.
 */
export const workspacePersistenceHydrated = writable<boolean>(false);

/**
 * Per-workspace, per-widget UI-only snapshots.
 *
 * Shape: workspaceId -> widgetId -> arbitrary serializable snapshot object.
 *
 * This store is intentionally decoupled from physics/domain state so widgets
 * can persist ephemeral UI preferences (expanded sections, toggles, sort mode,
 * etc.) without polluting model-level stores.
 */
export const workspaceWidgetData = writable<
  Record<string, Record<string, Record<string, unknown>>>
>({});

/**
 * Convenience projection of widget snapshot state for the active workspace.
 */
export const activeWorkspaceWidgetData = derived(
  [workspaceWidgetData, activeWorkspaceId],
  ([$data, $activeId]) => $data[$activeId] ?? {},
);

function nowIso(): string {
  return new Date().toISOString();
}

function mapWorkspaceToPersisted(
  ws: Workspace,
  widgetDataByWorkspace: Record<string, Record<string, Record<string, unknown>>>,
): PersistedWorkspaceRecord {
  return {
    id: ws.id,
    name: ws.name,
    description: ws.description,
    color: ws.color,
    createdAt: ws.createdAt,
    updatedAt: ws.updatedAt,
    autoDescription: ws.autoDescription,
    layout: {
      mode: ws.mode,
      dockingTree: ws.dockingRoot,
      canvasItems: ws.canvasItemList,
      zoom: ws.viewport.zoom,
      panX: ws.viewport.panX,
      panY: ws.viewport.panY,
    },
    inputs: ws.inputs,
    physicsState: ws.physics,
    widgetData: widgetDataByWorkspace[ws.id] ?? {},
  };
}

function syncLiveWorkspaceState(): void {
  saveCurrentWorkspaceState();
}

function collectPersistedState(): PersistedWorkspaceState {
  syncLiveWorkspaceState();

  const list = get(workspaces);
  const currentId = get(activeWorkspaceId);
  const widgetState = get(workspaceWidgetData);
  const inputs = get(workspaceInputsSnapshot);

  return {
    version: storageSchemaVersion() as 1,
    activeWorkspaceId: currentId,
    workspaces: list.map((ws) => mapWorkspaceToPersisted(ws, widgetState)),
    physics: {
      framework: get(activeFramework),
      particlesToml: inputs.particlesToml,
      verticesToml: inputs.verticesToml,
      modelName: inputs.modelName,
      initialIds: inputs.initialIds,
      finalIds: inputs.finalIds,
      cmsEnergy: inputs.cmsEnergy,
      maxLoopOrder: inputs.maxLoopOrder,
    },
    notebookDocument: get(notebookDocument),
  };
}

function persistWorkspaceStateNow(): void {
  if (!persistenceInitialised) return;

  if (saveTimer !== null) {
    clearTimeout(saveTimer);
    saveTimer = null;
  }

  const payload = collectPersistedState();
  saveWorkspaceState(payload);
}

function handleWindowPersistenceFlush(): void {
  persistWorkspaceStateNow();
}

/**
 * Debounced save scheduler.
 *
 * Replaces any pending timer so bursty updates collapse into one write.
 */
function schedulePersistedSave(): void {
  if (saveTimer !== null) {
    clearTimeout(saveTimer);
  }

  saveTimer = setTimeout(() => {
    persistWorkspaceStateNow();
  }, AUTO_SAVE_DEBOUNCE_MS);
}

function defaultInputsFromPersisted(state: PersistedWorkspaceState): Workspace["inputs"] {
  return {
    particlesToml: state.physics.particlesToml || DEFAULT_PARTICLES_TOML,
    verticesToml: state.physics.verticesToml || DEFAULT_VERTICES_TOML,
    modelName: state.physics.modelName || "Standard Model",
    initialIds: state.physics.initialIds.length > 0 ? [...state.physics.initialIds] : [...DEFAULT_INITIAL_IDS],
    finalIds: state.physics.finalIds.length > 0 ? [...state.physics.finalIds] : [...DEFAULT_FINAL_IDS],
    cmsEnergy: Number.isFinite(state.physics.cmsEnergy) ? state.physics.cmsEnergy : DEFAULT_CMS_ENERGY,
    maxLoopOrder: Number.isFinite(state.physics.maxLoopOrder) ? state.physics.maxLoopOrder : 0,
  };
}

function defaultPhysicsFromPersisted(state: PersistedWorkspaceState): Workspace["physics"] {
  return {
    framework: state.physics.framework,
    theoreticalModel: null,
    activeReaction: null,
    reconstructedStates: [],
    generatedDiagrams: null,
    amplitudeResults: [],
    activeAmplitude: "",
    kinematics: null,
    observableScripts: [],
    cutScripts: [],
    logs: [],
  };
}

function hydrateFromState(state: PersistedWorkspaceState): void {
  const defaultInputs = defaultInputsFromPersisted(state);
  const defaultPhysics = defaultPhysicsFromPersisted(state);

  const restoredWorkspaces: Workspace[] = state.workspaces.map((ws) => ({
    id: ws.id,
    name: ws.name,
    description: ws.description,
    color: ws.color,
    createdAt: ws.createdAt,
    updatedAt: ws.updatedAt,
    autoDescription: ws.autoDescription,
    dockingRoot: ws.layout.dockingTree,
    canvasItemList: ws.layout.canvasItems,
    viewport: {
      zoom: ws.layout.zoom,
      panX: ws.layout.panX,
      panY: ws.layout.panY,
    },
    mode: ws.layout.mode,
    inputs: ws.inputs ? {
      particlesToml: ws.inputs.particlesToml,
      verticesToml: ws.inputs.verticesToml,
      modelName: ws.inputs.modelName,
      initialIds: [...ws.inputs.initialIds],
      finalIds: [...ws.inputs.finalIds],
      cmsEnergy: ws.inputs.cmsEnergy,
      maxLoopOrder: ws.inputs.maxLoopOrder,
    } : structuredClone(defaultInputs),
    physics: ws.physicsState ? {
      framework: ws.physicsState.framework,
      theoreticalModel: structuredClone(ws.physicsState.theoreticalModel),
      activeReaction: structuredClone(ws.physicsState.activeReaction),
      reconstructedStates: structuredClone(ws.physicsState.reconstructedStates ?? []),
      generatedDiagrams: structuredClone(ws.physicsState.generatedDiagrams),
      amplitudeResults: structuredClone(ws.physicsState.amplitudeResults ?? []),
      activeAmplitude: ws.physicsState.activeAmplitude ?? "",
      kinematics: structuredClone(ws.physicsState.kinematics),
      observableScripts: structuredClone(ws.physicsState.observableScripts ?? []),
      cutScripts: structuredClone(ws.physicsState.cutScripts ?? []),
      logs: structuredClone(ws.physicsState.logs ?? []),
    } : structuredClone(defaultPhysics),
  }));

  if (restoredWorkspaces.length === 0) {
    return;
  }

  workspaces.set(restoredWorkspaces);

  const hasActive = restoredWorkspaces.some((ws) => ws.id === state.activeWorkspaceId);
  const selectedWorkspace = hasActive
    ? restoredWorkspaces.find((ws) => ws.id === state.activeWorkspaceId)!
    : restoredWorkspaces[0];

  activeWorkspaceId.set(selectedWorkspace.id);

  layoutRoot.set(selectedWorkspace.dockingRoot);
  canvasItems.set(selectedWorkspace.canvasItemList);
  canvasViewport.set(selectedWorkspace.viewport);
  viewMode.set(selectedWorkspace.mode);

  const initialInputs = selectedWorkspace.inputs ?? defaultInputs;
  setAllInputs({
    particlesToml: initialInputs.particlesToml,
    verticesToml: initialInputs.verticesToml,
    modelName: initialInputs.modelName,
    initialIds: [...initialInputs.initialIds],
    finalIds: [...initialInputs.finalIds],
    cmsEnergy: initialInputs.cmsEnergy,
    maxLoopOrder: initialInputs.maxLoopOrder,
  });

  const initialPhysics = selectedWorkspace.physics ?? defaultPhysics;
  activeFramework.set(initialPhysics.framework);
  theoreticalModel.set(structuredClone(initialPhysics.theoreticalModel));
  activeReaction.set(structuredClone(initialPhysics.activeReaction));
  reconstructedStates.set(structuredClone(initialPhysics.reconstructedStates));
  generatedDiagrams.set(structuredClone(initialPhysics.generatedDiagrams));
  amplitudeResults.set(structuredClone(initialPhysics.amplitudeResults));
  activeAmplitude.set(initialPhysics.activeAmplitude);
  kinematics.set(structuredClone(initialPhysics.kinematics));
  observableScripts.set(structuredClone(initialPhysics.observableScripts));
  cutScripts.set(structuredClone(initialPhysics.cutScripts));
  logs.set(structuredClone(initialPhysics.logs));

  notebookDocument.set(state.notebookDocument);

  const widgetDataByWorkspace: Record<string, Record<string, Record<string, unknown>>> = {};
  for (const ws of state.workspaces) {
    widgetDataByWorkspace[ws.id] = ws.widgetData ?? {};
  }
  workspaceWidgetData.set(widgetDataByWorkspace);
}

export function setWidgetUiSnapshot(
  widgetId: string,
  patch: Record<string, unknown>,
): void {
  const wsId = get(activeWorkspaceId);
  workspaceWidgetData.update((all) => {
    const currentWs = all[wsId] ?? {};
    const currentWidget = currentWs[widgetId] ?? {};
    return {
      ...all,
      [wsId]: {
        ...currentWs,
        [widgetId]: {
          ...currentWidget,
          ...patch,
          updatedAt: nowIso(),
        },
      },
    };
  });
}

/**
 * Retrieve a typed widget UI snapshot for the active workspace.
 *
 * Returns `null` when no snapshot exists for `widgetId`.
 */
export function getWidgetUiSnapshot<T extends Record<string, unknown>>(
  widgetId: string,
): T | null {
  const wsId = get(activeWorkspaceId);
  const all = get(workspaceWidgetData);
  const found = all[wsId]?.[widgetId];
  return (found as T | undefined) ?? null;
}

/**
 * Initialize persistence watchers and perform one-time hydration.
 *
 * Idempotent: repeated calls after initialization are no-ops.
 *
 * Lifecycle contract:
 * - Call once during app bootstrap.
 * - Pair with {@link destroyWorkspacePersistence} on teardown/tests.
 */
export function initWorkspacePersistence(): void {
  if (persistenceInitialised) return;
  persistenceInitialised = true;

  try {
    runMigrationSelfCheck();
  } catch (error) {
    console.error("[Storage] Migration self-check failed.", error);
  }

  const restored = loadWorkspaceState();
  if (restored) {
    hydrateFromState(restored);
  }

  workspacePersistenceHydrated.set(true);

  stopPersistenceWatches = [
    workspaces.subscribe(() => schedulePersistedSave()),
    activeWorkspaceId.subscribe(() => schedulePersistedSave()),
    layoutRoot.subscribe(() => schedulePersistedSave()),
    canvasItems.subscribe(() => schedulePersistedSave()),
    canvasViewport.subscribe(() => schedulePersistedSave()),
    viewMode.subscribe(() => schedulePersistedSave()),
    workspaceInputsSnapshot.subscribe(() => schedulePersistedSave()),
    activeFramework.subscribe(() => schedulePersistedSave()),
    notebookDocument.subscribe(() => schedulePersistedSave()),
    workspaceWidgetData.subscribe(() => schedulePersistedSave()),
  ];

  if (typeof window !== "undefined") {
    window.addEventListener("pagehide", handleWindowPersistenceFlush);
    window.addEventListener("beforeunload", handleWindowPersistenceFlush);
  }

  schedulePersistedSave();
}

/**
 * Stop persistence watchers, flush pending writes, and release listeners.
 *
 * Safe to call multiple times.
 */
export function destroyWorkspacePersistence(): void {
  for (const stop of stopPersistenceWatches) {
    stop();
  }
  stopPersistenceWatches = [];

  if (saveTimer !== null) {
    clearTimeout(saveTimer);
    saveTimer = null;
  }

  if (typeof window !== "undefined") {
    window.removeEventListener("pagehide", handleWindowPersistenceFlush);
    window.removeEventListener("beforeunload", handleWindowPersistenceFlush);
  }

  persistWorkspaceStateNow();

  persistenceInitialised = false;
}
