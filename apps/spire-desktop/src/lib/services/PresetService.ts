import { get } from "svelte/store";
import type { CanvasItem, ColNode, LayoutNode, RowNode, StackNode, WidgetLeaf } from "$lib/stores/layoutStore";
import {
  canvasItems,
  layoutRoot,
  makeLayoutId,
  viewMode,
} from "$lib/stores/layoutStore";
import type { WidgetType } from "$lib/stores/notebookStore";

const CUSTOM_PRESET_KEY = "spire.layout.custom_presets";

export type BuiltInPresetId = "theory-studio" | "analysis-focus" | "minimal";
export type LayoutPresetId = BuiltInPresetId | string;

export interface SavedLayoutPreset {
  id: string;
  name: string;
  createdAt: string;
  mode: "canvas" | "docking";
  canvasState: CanvasItem[];
  dockingState: LayoutNode;
}

function leaf(widgetType: WidgetType): WidgetLeaf {
  return {
    id: makeLayoutId(),
    type: "widget",
    widgetType,
    widgetData: {},
  };
}

function stack(widgetType: WidgetType): StackNode {
  return {
    id: makeLayoutId(),
    type: "stack",
    children: [leaf(widgetType)],
    activeIndex: 0,
  };
}

function row(children: LayoutNode[]): RowNode {
  return {
    id: makeLayoutId(),
    type: "row",
    children,
    sizes: children.map(() => 1),
  };
}

function col(children: LayoutNode[]): ColNode {
  return {
    id: makeLayoutId(),
    type: "col",
    children,
    sizes: children.map(() => 1),
  };
}

function theoryStudioPreset(): LayoutNode {
  return row([
    stack("lagrangian_workbench"),
    stack("diagram_editor"),
    col([stack("references"), stack("notebook")]),
  ]);
}

function analysisFocusPreset(): LayoutNode {
  return col([
    row([
      stack("model"),
      col([stack("diagram"), row([stack("analysis"), stack("dalitz")])]),
    ]),
    stack("log"),
  ]);
}

function minimalPreset(): LayoutNode {
  return stack("notebook");
}

export const BUILT_IN_PRESETS: ReadonlyArray<{ id: BuiltInPresetId; name: string }> = [
  { id: "theory-studio", name: "Theory Studio" },
  { id: "analysis-focus", name: "Analysis Focus" },
  { id: "minimal", name: "Minimal" },
];

function dockingForBuiltIn(id: BuiltInPresetId): LayoutNode {
  switch (id) {
    case "theory-studio":
      return theoryStudioPreset();
    case "analysis-focus":
      return analysisFocusPreset();
    case "minimal":
      return minimalPreset();
    default:
      return minimalPreset();
  }
}

function readCustomPresets(): SavedLayoutPreset[] {
  if (typeof localStorage === "undefined") return [];
  try {
    const raw = localStorage.getItem(CUSTOM_PRESET_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as SavedLayoutPreset[];
    if (!Array.isArray(parsed)) return [];
    return parsed;
  } catch {
    return [];
  }
}

function writeCustomPresets(presets: SavedLayoutPreset[]): void {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(CUSTOM_PRESET_KEY, JSON.stringify(presets));
}

export function listCustomPresets(): SavedLayoutPreset[] {
  return readCustomPresets();
}

export function saveCurrentLayoutPreset(name: string): SavedLayoutPreset {
  const trimmedName = name.trim() || `Preset ${new Date().toLocaleTimeString()}`;
  const id = `custom-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
  const preset: SavedLayoutPreset = {
    id,
    name: trimmedName,
    createdAt: new Date().toISOString(),
    mode: get(viewMode),
    canvasState: JSON.parse(JSON.stringify(get(canvasItems))) as CanvasItem[],
    dockingState: JSON.parse(JSON.stringify(get(layoutRoot))) as LayoutNode,
  };

  const current = readCustomPresets();
  writeCustomPresets([preset, ...current].slice(0, 24));
  return preset;
}

export function applyLayoutPreset(presetId: LayoutPresetId): boolean {
  const builtIn = BUILT_IN_PRESETS.find((preset) => preset.id === presetId);
  if (builtIn) {
    layoutRoot.set(dockingForBuiltIn(builtIn.id));
    viewMode.set("docking");
    return true;
  }

  const custom = readCustomPresets().find((preset) => preset.id === presetId);
  if (!custom) return false;

  canvasItems.set(JSON.parse(JSON.stringify(custom.canvasState)) as CanvasItem[]);
  layoutRoot.set(JSON.parse(JSON.stringify(custom.dockingState)) as LayoutNode);
  viewMode.set(custom.mode);
  return true;
}
