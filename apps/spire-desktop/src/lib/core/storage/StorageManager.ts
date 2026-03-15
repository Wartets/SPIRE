import {
  applyMigrationChain,
  CURRENT_SCHEMA_VERSION,
  type PersistedWorkspaceState,
} from "./migrations";

export const STORAGE_KEY = "spire_workspace_state";
export const CORRUPTED_BACKUP_KEY = "spire_corrupted_backup";

function canUseStorage(): boolean {
  return typeof localStorage !== "undefined";
}

export function saveWorkspaceState(state: PersistedWorkspaceState): void {
  if (!canUseStorage()) return;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
}

export function readWorkspaceStateRaw(): unknown {
  if (!canUseStorage()) return null;
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) return null;
  return JSON.parse(raw) as unknown;
}

export function loadWorkspaceState(): PersistedWorkspaceState | null {
  if (!canUseStorage()) return null;
  const rawString = localStorage.getItem(STORAGE_KEY);
  if (!rawString) return null;

  try {
    const parsed = JSON.parse(rawString) as unknown;
    return applyMigrationChain(parsed);
  } catch (error) {
    localStorage.setItem(CORRUPTED_BACKUP_KEY, rawString);
    localStorage.removeItem(STORAGE_KEY);
    console.warn("[Storage] Persisted workspace payload was corrupted and moved to backup.", error);
    return null;
  }
}

export function clearWorkspaceState(): void {
  if (!canUseStorage()) return;
  localStorage.removeItem(STORAGE_KEY);
}

export function storageSchemaVersion(): number {
  return CURRENT_SCHEMA_VERSION;
}
