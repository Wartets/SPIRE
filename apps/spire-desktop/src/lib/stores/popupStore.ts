import { writable } from "svelte/store";

export type PopupTone = "neutral" | "info" | "warn" | "danger" | "success";
export type PopupActionVariant = "default" | "primary" | "danger" | "ghost";

export interface PopupMetaRow {
  label: string;
  value: string;
}

export interface PopupAction {
  id: string;
  label: string;
  variant?: PopupActionVariant;
  autofocus?: boolean;
}

export interface PopupOptions {
  title: string;
  message?: string;
  details?: string[];
  meta?: PopupMetaRow[];
  tone?: PopupTone;
  actions: PopupAction[];
  closeActionId?: string;
  maxWidth?: number;
}

export interface PopupState {
  visible: boolean;
  title: string;
  message: string;
  details: string[];
  meta: PopupMetaRow[];
  tone: PopupTone;
  actions: PopupAction[];
  closeActionId: string;
  maxWidth: number;
}

const defaultState: PopupState = {
  visible: false,
  title: "",
  message: "",
  details: [],
  meta: [],
  tone: "neutral",
  actions: [],
  closeActionId: "cancel",
  maxWidth: 520,
};

export const popupState = writable<PopupState>(defaultState);

let resolver: ((actionId: string) => void) | null = null;

export function openPopup(options: PopupOptions): Promise<string> {
  if (resolver) {
    resolver(defaultState.closeActionId);
    resolver = null;
  }

  popupState.set({
    visible: true,
    title: options.title,
    message: options.message ?? "",
    details: options.details ?? [],
    meta: options.meta ?? [],
    tone: options.tone ?? "neutral",
    actions: options.actions,
    closeActionId: options.closeActionId ?? options.actions[options.actions.length - 1]?.id ?? "cancel",
    maxWidth: options.maxWidth ?? 520,
  });

  return new Promise<string>((resolve) => {
    resolver = resolve;
  });
}

export function resolvePopup(actionId: string): void {
  popupState.update((current) => ({ ...current, visible: false }));
  if (resolver) {
    resolver(actionId);
    resolver = null;
  }
}

export function closePopup(): void {
  let closeId = defaultState.closeActionId;
  popupState.update((current) => {
    closeId = current.closeActionId;
    return { ...current, visible: false };
  });
  if (resolver) {
    resolver(closeId);
    resolver = null;
  }
}
