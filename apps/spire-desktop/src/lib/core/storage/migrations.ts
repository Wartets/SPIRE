import type { Workspace, LayoutNode, CanvasItem, CanvasViewport, ViewMode } from "$lib/stores/layoutStore";
import type { NotebookDocument } from "$lib/core/domain/notebook";
import type { TheoreticalFramework } from "$lib/types/spire";

export const CURRENT_SCHEMA_VERSION = 1;

export interface PersistedWorkspaceLayout {
  mode: ViewMode;
  dockingTree: LayoutNode;
  canvasItems: CanvasItem[];
  zoom: number;
  panX: number;
  panY: number;
}

export interface PersistedWorkspaceRecord {
  id: string;
  name: string;
  description: string;
  color: string;
  createdAt: string;
  updatedAt: string;
  autoDescription: boolean;
  layout: PersistedWorkspaceLayout;
  inputs?: Workspace["inputs"];
  physicsState?: Workspace["physics"];
  widgetData: Record<string, Record<string, unknown>>;
}

export interface PersistedWorkspaceStateV1 {
  version: 1;
  activeWorkspaceId: string;
  workspaces: PersistedWorkspaceRecord[];
  physics: {
    framework: TheoreticalFramework;
    particlesToml: string;
    verticesToml: string;
    modelName: string;
    initialIds: string[];
    finalIds: string[];
    cmsEnergy: number;
    maxLoopOrder: number;
  };
  notebookDocument: NotebookDocument;
}

export type PersistedWorkspaceState = PersistedWorkspaceStateV1;

interface MigrationStep {
  version: number;
  up: (oldState: unknown) => unknown;
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function asIsoString(input: unknown): string {
  if (typeof input === "string" && input.trim().length > 0) {
    return input;
  }
  return new Date().toISOString();
}

function parseViewport(input: unknown): CanvasViewport {
  if (!isObject(input)) {
    return { panX: 0, panY: 0, zoom: 1 };
  }
  return {
    panX: typeof input.panX === "number" ? input.panX : 0,
    panY: typeof input.panY === "number" ? input.panY : 0,
    zoom: typeof input.zoom === "number" ? input.zoom : 1,
  };
}

function parseNotebookDocument(input: unknown): NotebookDocument {
  if (!isObject(input)) {
    return {
      title: "Untitled Notebook",
      sessionId: null,
      cells: [],
      dirty: false,
    };
  }

  return {
    title: typeof input.title === "string" ? input.title : "Untitled Notebook",
    sessionId: typeof input.sessionId === "string" ? input.sessionId : null,
    cells: Array.isArray(input.cells) ? input.cells : [],
    dirty: typeof input.dirty === "boolean" ? input.dirty : false,
  };
}

const migrationChain: MigrationStep[] = [
  {
    version: 1,
    up: (oldState: unknown): PersistedWorkspaceStateV1 => {
      if (!isObject(oldState)) {
        throw new Error("Persisted workspace payload must be an object.");
      }

      const wsInput = Array.isArray(oldState.workspaces)
        ? oldState.workspaces.filter(isObject)
        : [];

      const migratedWorkspaces: PersistedWorkspaceRecord[] = wsInput.map((ws, index) => {
        const viewport = parseViewport(ws.viewport);
        const mode = ws.mode === "canvas" ? "canvas" : "docking";
        return {
          id: typeof ws.id === "string" ? ws.id : `ws-migrated-${index}`,
          name: typeof ws.name === "string" ? ws.name : `Workspace ${index + 1}`,
          description: typeof ws.description === "string" ? ws.description : "Untitled workspace",
          color: typeof ws.color === "string" ? ws.color : "#5eb8ff",
          createdAt: asIsoString(ws.createdAt),
          updatedAt: asIsoString(ws.updatedAt),
          autoDescription: typeof ws.autoDescription === "boolean" ? ws.autoDescription : true,
          layout: {
            mode,
            dockingTree: (ws.dockingRoot as LayoutNode) ?? (ws.layoutTree as LayoutNode),
            canvasItems: Array.isArray(ws.canvasItemList)
              ? (ws.canvasItemList as CanvasItem[])
              : Array.isArray(ws.canvasItems)
                ? (ws.canvasItems as CanvasItem[])
                : [],
            zoom: typeof viewport.zoom === "number" ? viewport.zoom : 1,
            panX: typeof viewport.panX === "number" ? viewport.panX : 0,
            panY: typeof viewport.panY === "number" ? viewport.panY : 0,
          },
          inputs: isObject(ws.inputs) ? (ws.inputs as unknown as Workspace["inputs"]) : undefined,
          physicsState: isObject(ws.physicsState) ? (ws.physicsState as unknown as Workspace["physics"]) : undefined,
          widgetData: isObject(ws.widgetData)
            ? (ws.widgetData as Record<string, Record<string, unknown>>)
            : {},
        };
      });

      const physics = isObject(oldState.physics) ? oldState.physics : {};
      const notebookDocument = parseNotebookDocument(oldState.notebookDocument);

      const activeWorkspaceId =
        typeof oldState.activeWorkspaceId === "string" ? oldState.activeWorkspaceId : migratedWorkspaces[0]?.id ?? "";

      return {
        version: 1,
        activeWorkspaceId,
        workspaces: migratedWorkspaces,
        physics: {
          framework: (physics.framework as TheoreticalFramework) ?? "StandardModel",
          particlesToml: typeof physics.particlesToml === "string" ? physics.particlesToml : "",
          verticesToml: typeof physics.verticesToml === "string" ? physics.verticesToml : "",
          modelName: typeof physics.modelName === "string" ? physics.modelName : "Standard Model",
          initialIds: Array.isArray(physics.initialIds) ? (physics.initialIds as string[]) : [],
          finalIds: Array.isArray(physics.finalIds) ? (physics.finalIds as string[]) : [],
          cmsEnergy: typeof physics.cmsEnergy === "number" ? physics.cmsEnergy : 91.2,
          maxLoopOrder: typeof physics.maxLoopOrder === "number" ? physics.maxLoopOrder : 0,
        },
        notebookDocument,
      };
    },
  },
];

export function applyMigrationChain(input: unknown): PersistedWorkspaceState {
  if (!isObject(input)) {
    throw new Error("Persisted state is not an object.");
  }

  const sourceVersion = typeof input.version === "number" ? input.version : 0;
  if (sourceVersion > CURRENT_SCHEMA_VERSION) {
    throw new Error(`Persisted schema version ${sourceVersion} is newer than supported ${CURRENT_SCHEMA_VERSION}.`);
  }

  let current: unknown = input;
  for (const step of migrationChain) {
    if (sourceVersion < step.version) {
      current = step.up(current);
    }
  }

  if (!isObject(current) || current.version !== CURRENT_SCHEMA_VERSION) {
    throw new Error("Migration chain did not produce the current schema version.");
  }

  return current as unknown as PersistedWorkspaceState;
}

/**
 * Internal migration guard for deterministic upgrade behavior.
 */
export function runMigrationSelfCheck(): void {
  const migrated = applyMigrationChain({
    version: 0,
    workspaces: [
      {
        id: "ws-test",
        name: "Workspace Test",
        color: "#5eb8ff",
        mode: "docking",
        dockingRoot: {
          id: "root",
          type: "widget",
          widgetType: "model",
          widgetData: {},
        },
        canvasItemList: [],
        viewport: { panX: 0, panY: 0, zoom: 1 },
      },
    ],
    activeWorkspaceId: "ws-test",
    physics: {
      framework: "StandardModel",
      particlesToml: "",
      verticesToml: "",
      modelName: "Standard Model",
      initialIds: ["e-", "e+"],
      finalIds: ["mu-", "mu+"],
      cmsEnergy: 91.2,
      maxLoopOrder: 0,
    },
    notebookDocument: {
      id: "nb-test",
      title: "Notebook",
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
      sessionId: null,
      cells: [],
      dirty: false,
    },
  });

  if (migrated.version !== 1) {
    throw new Error("Migration self-check failed: version should be upgraded to 1.");
  }
  if (migrated.workspaces.length !== 1 || migrated.workspaces[0].id !== "ws-test") {
    throw new Error("Migration self-check failed: workspace records were not preserved.");
  }
}
