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
import { activeFramework } from "$lib/stores/physicsStore";
import { workspaceInputsSnapshot, setAllInputs } from "$lib/stores/workspaceInputsStore";
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

const AUTO_SAVE_DEBOUNCE_MS = 1000;

let persistenceInitialised = false;
let stopPersistenceWatches: Array<() => void> = [];
let saveTimer: ReturnType<typeof setTimeout> | null = null;

export const workspacePersistenceHydrated = writable<boolean>(false);

export const workspaceWidgetData = writable<
  Record<string, Record<string, Record<string, unknown>>>
>({});

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
  const encoded = JSON.stringify(payload);
  saveWorkspaceState(payload);
  const kb = (encoded.length / 1024).toFixed(1);
  console.info(`[Storage] Workspace state synced (${kb}kb)`);
}

function handleWindowPersistenceFlush(): void {
  persistWorkspaceStateNow();
}

function schedulePersistedSave(): void {
  if (saveTimer !== null) {
    clearTimeout(saveTimer);
  }

  saveTimer = setTimeout(() => {
    persistWorkspaceStateNow();
  }, AUTO_SAVE_DEBOUNCE_MS);
}

function hydrateFromState(state: PersistedWorkspaceState): void {
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

  setAllInputs({
    particlesToml: state.physics.particlesToml,
    verticesToml: state.physics.verticesToml,
    modelName: state.physics.modelName,
    initialIds: state.physics.initialIds,
    finalIds: state.physics.finalIds,
    cmsEnergy: state.physics.cmsEnergy,
    maxLoopOrder: state.physics.maxLoopOrder,
  });
  activeFramework.set(state.physics.framework);

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

export function getWidgetUiSnapshot<T extends Record<string, unknown>>(
  widgetId: string,
): T | null {
  const wsId = get(activeWorkspaceId);
  const all = get(workspaceWidgetData);
  const found = all[wsId]?.[widgetId];
  return (found as T | undefined) ?? null;
}

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
