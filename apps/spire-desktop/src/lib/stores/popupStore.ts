import { get, writable } from "svelte/store";

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
  input?: PopupInput;
  actions: PopupAction[];
  closeActionId?: string;
  maxWidth?: number;
}

export interface PopupInput {
  label: string;
  placeholder?: string;
  value?: string;
  multiline?: boolean;
  rows?: number;
}

export interface PopupTextInputOptions {
  title: string;
  message?: string;
  label: string;
  placeholder?: string;
  value?: string;
  multiline?: boolean;
  rows?: number;
  tone?: PopupTone;
  confirmLabel?: string;
  cancelLabel?: string;
  confirmVariant?: PopupActionVariant;
  requireNonEmpty?: boolean;
  maxWidth?: number;
}

export interface PopupState {
  visible: boolean;
  title: string;
  message: string;
  details: string[];
  meta: PopupMetaRow[];
  tone: PopupTone;
  input: PopupInput | null;
  inputValue: string;
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
  input: null,
  inputValue: "",
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
    input: options.input ?? null,
    inputValue: options.input?.value ?? "",
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

export function setPopupInputValue(value: string): void {
  popupState.update((current) => ({ ...current, inputValue: value }));
}

export async function openTextInputPopup(options: PopupTextInputOptions): Promise<string | null> {
  const confirmId = "confirm";
  const cancelId = "cancel";
  const action = await openPopup({
    title: options.title,
    message: options.message,
    tone: options.tone ?? "info",
    input: {
      label: options.label,
      placeholder: options.placeholder,
      value: options.value ?? "",
      multiline: options.multiline ?? false,
      rows: options.rows ?? 5,
    },
    actions: [
      {
        id: confirmId,
        label: options.confirmLabel ?? "Confirm",
        variant: options.confirmVariant ?? "primary",
        autofocus: true,
      },
      {
        id: cancelId,
        label: options.cancelLabel ?? "Cancel",
        variant: "ghost",
      },
    ],
    closeActionId: cancelId,
    maxWidth: options.maxWidth ?? 640,
  });

  if (action !== confirmId) {
    return null;
  }

  const value = get(popupState).inputValue;
  const normalized = value;
  if (options.requireNonEmpty && normalized.trim().length === 0) {
    return null;
  }
  return normalized;
}
