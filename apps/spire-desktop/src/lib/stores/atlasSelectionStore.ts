import { writable } from "svelte/store";

export type AtlasSelectionTarget = "initial" | "final";

export interface AtlasSelectionRequest {
  pending: boolean;
  target: AtlasSelectionTarget | null;
}

export interface AtlasSelectionResult {
  target: AtlasSelectionTarget;
  particleId: string;
  nonce: number;
}

const initialState: AtlasSelectionRequest = {
  pending: false,
  target: null,
};

export const atlasSelectionRequest = writable<AtlasSelectionRequest>(initialState);
export const atlasSelectionResult = writable<AtlasSelectionResult | null>(null);

export function requestAtlasSelection(target: AtlasSelectionTarget): void {
  atlasSelectionRequest.set({ pending: true, target });
}

export function clearAtlasSelectionRequest(): void {
  atlasSelectionRequest.set(initialState);
}

export function submitAtlasSelection(target: AtlasSelectionTarget, particleId: string): void {
  atlasSelectionResult.set({
    target,
    particleId,
    nonce: Date.now(),
  });
  clearAtlasSelectionRequest();
}

export function clearAtlasSelectionResult(): void {
  atlasSelectionResult.set(null);
}
