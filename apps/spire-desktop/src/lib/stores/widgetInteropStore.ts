import { derived, get, writable } from "svelte/store";
import type { WidgetType } from "$lib/stores/notebookStore";

export interface WidgetInteropEnvelope<T = Record<string, unknown>> {
  widgetType: WidgetType;
  at: number;
  payload: T;
}

export type WidgetInteropState = Partial<Record<WidgetType, WidgetInteropEnvelope>>;

const _interopState = writable<WidgetInteropState>({});

export const widgetInteropState = {
  subscribe: _interopState.subscribe,
};

export const latestInteropEvent = derived(_interopState, ($state) => {
  let latest: WidgetInteropEnvelope | null = null;
  for (const entry of Object.values($state)) {
    if (!entry) continue;
    if (!latest || entry.at > latest.at) latest = entry;
  }
  return latest;
});

export function publishWidgetInterop<T extends Record<string, unknown>>(
  widgetType: WidgetType,
  payload: T,
): void {
  const envelope: WidgetInteropEnvelope<T> = {
    widgetType,
    at: Date.now(),
    payload,
  };

  _interopState.update((state) => ({
    ...state,
    [widgetType]: envelope,
  }));
}

export function readWidgetInterop<T extends Record<string, unknown>>(
  widgetType: WidgetType,
): WidgetInteropEnvelope<T> | null {
  const state = get(_interopState);
  return (state[widgetType] as WidgetInteropEnvelope<T> | undefined) ?? null;
}
