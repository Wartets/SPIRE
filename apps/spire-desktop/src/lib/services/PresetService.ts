/**
 * SPIRE - Layout Preset Service
 * Provides built-in and custom layout presets for physicist workflows.
 */

import type { LayoutNode } from '../stores/layoutStore';
import { dualLayoutStore } from '../stores/layoutStore';

// Built-in preset definitions
export const BUILTIN_PRESETS: Record<string, LayoutNode> = {
  'theory-studio': {
    id: 'preset-theory',
    type: 'row',
    sizes: [1, 2, 1],
    children: [
      { id: 'preset-left', type: 'widget', widgetType: 'lagrangian', widgetData: {} },
      { id: 'preset-center', type: 'widget', widgetType: 'diagram', widgetData: {} },
      { id: 'preset-right', type: 'widget', widgetType: 'references', widgetData: {} },
    ],
  },
  'analysis-focus': {
    id: 'preset-analysis',
    type: 'col',
    sizes: [1, 2, 1],
    children: [
      { id: 'preset-model', type: 'widget', widgetType: 'model', widgetData: {} },
      {
        id: 'preset-center',
        type: 'row',
        sizes: [2, 1],
        children: [
          { id: 'preset-diagram', type: 'widget', widgetType: 'diagram', widgetData: {} },
          {
            id: 'preset-bottom',
            type: 'col',
            sizes: [1, 1],
            children: [
              { id: 'preset-cross', type: 'widget', widgetType: 'amplitude', widgetData: {} },
              { id: 'preset-dalitz', type: 'widget', widgetType: 'dalitz', widgetData: {} },
            ],
          },
        ],
      },
      { id: 'preset-console', type: 'widget', widgetType: 'log', widgetData: {} },
    ],
  },
  'minimal': {
    id: 'preset-minimal',
    type: 'stack',
    children: [
      { id: 'preset-single', type: 'widget', widgetType: 'diagram', widgetData: {} },
    ],
    activeIndex: 0,
  },
};

export function applyLayoutPreset(presetId: string): void {
  const preset = BUILTIN_PRESETS[presetId];
  if (!preset) return;
  dualLayoutStore.update((mem) => ({
    ...mem,
    mode: 'docking',
    dockingState: preset,
  }));
}

export function saveCustomPreset(name: string): void {
  const mem = get(dualLayoutStore);
  const preset = {
    name,
    dockingState: mem.dockingState,
    canvasState: mem.canvasState,
  };
  localStorage.setItem('spire.preset.' + name, JSON.stringify(preset));
}

export function loadCustomPreset(name: string): void {
  const raw = localStorage.getItem('spire.preset.' + name);
  if (!raw) return;
  const preset = JSON.parse(raw);
  dualLayoutStore.set({
    mode: 'docking',
    dockingState: preset.dockingState,
    canvasState: preset.canvasState,
  });
}
