import { writable } from "svelte/store";
import { getWidgetUiSnapshot, setWidgetUiSnapshot } from "$lib/stores/workspaceStore";
import { activeWorkspaceId } from "$lib/stores/layoutStore";
import {
  getPdgCacheDiagnostics,
  getPdgLiveApiSettings,
  getPdgNetworkDiagnostics,
  setPdgLiveApiSettings,
} from "$lib/api";
import type { PdgParticleRecord } from "$lib/types/spire";

export type PdgMergeMode = "Replace" | "Patch" | "Overlay";
export type PdgBootstrapPreset = "SeedCoreSM" | "SeedQuarkSector" | "SeedLeptonSector" | "FullCatalogImport";

export interface PdgQueryStats {
  queryCount: number;
  cacheHits: number;
  cacheMisses: number;
  averageLatencyMs: number;
  lastLatencyMs: number;
}

export interface PdgCitationEntry {
  key: string;
  title: string;
  year: number;
  doi?: string;
  edition?: string;
}

export interface PdgIntegrationState {
  editionLock: string | null;
  mergeMode: PdgMergeMode;
  bootstrapPreset: PdgBootstrapPreset | null;
  sourceArbitration: "local" | "api" | "mixed" | "unknown";
  authoritativeSource: string | null;
  lastSyncAt: string | null;
  stale: boolean;
  mismatch: boolean;
  queryStats: PdgQueryStats;
  citations: PdgCitationEntry[];
  liveApiEnabled: boolean;
  network: {
    queueDepth: number;
    queueDepthPeak: number;
    responses429: number;
    responses503: number;
    retries: number;
    lastError: string | null;
  };
}

const DEFAULT_STATE: PdgIntegrationState = {
  editionLock: null,
  mergeMode: "Overlay",
  bootstrapPreset: null,
  sourceArbitration: "unknown",
  authoritativeSource: null,
  lastSyncAt: null,
  stale: false,
  mismatch: false,
  queryStats: {
    queryCount: 0,
    cacheHits: 0,
    cacheMisses: 0,
    averageLatencyMs: 0,
    lastLatencyMs: 0,
  },
  citations: [],
  liveApiEnabled: false,
  network: {
    queueDepth: 0,
    queueDepthPeak: 0,
    responses429: 0,
    responses503: 0,
    retries: 0,
    lastError: null,
  },
};

const SNAPSHOT_KEY = "pdg-integration-state";

export const pdgIntegrationState = writable<PdgIntegrationState>({ ...DEFAULT_STATE });

function hydrateFromWorkspace(): void {
  const snapshot = getWidgetUiSnapshot<Partial<PdgIntegrationState>>(SNAPSHOT_KEY);
  if (!snapshot) {
    pdgIntegrationState.set({ ...DEFAULT_STATE });
    return;
  }
  pdgIntegrationState.set({
    ...DEFAULT_STATE,
    ...snapshot,
    queryStats: {
      ...DEFAULT_STATE.queryStats,
      ...(snapshot.queryStats ?? {}),
    },
    network: {
      ...DEFAULT_STATE.network,
      ...(snapshot.network ?? {}),
    },
    citations: Array.isArray(snapshot.citations) ? snapshot.citations : [],
  });
}

activeWorkspaceId.subscribe(() => {
  hydrateFromWorkspace();
});

pdgIntegrationState.subscribe((state) => {
  setWidgetUiSnapshot(SNAPSHOT_KEY, state as unknown as Record<string, unknown>);
});

export function updatePdgIntegration(patch: Partial<PdgIntegrationState>): void {
  pdgIntegrationState.update((state) => ({ ...state, ...patch }));
}

export function recordPdgQuery(latencyMs: number, cacheHit: boolean): void {
  pdgIntegrationState.update((state) => {
    const nextCount = state.queryStats.queryCount + 1;
    const totalLatency = state.queryStats.averageLatencyMs * state.queryStats.queryCount + latencyMs;
    return {
      ...state,
      queryStats: {
        queryCount: nextCount,
        cacheHits: state.queryStats.cacheHits + (cacheHit ? 1 : 0),
        cacheMisses: state.queryStats.cacheMisses + (cacheHit ? 0 : 1),
        averageLatencyMs: totalLatency / nextCount,
        lastLatencyMs: latencyMs,
      },
    };
  });
}

/**
 * Refresh PDG data-plane diagnostics from backend caches/DB timings.
 */
export async function refreshPdgDiagnosticsFromBackend(): Promise<void> {
  try {
    const [diagnostics, networkDiagnostics] = await Promise.all([
      getPdgCacheDiagnostics(),
      getPdgNetworkDiagnostics(),
    ]);
    pdgIntegrationState.update((state) => ({
      ...state,
      queryStats: {
        queryCount: diagnostics.db_queries,
        cacheHits: diagnostics.total_hits,
        cacheMisses: diagnostics.total_misses,
        averageLatencyMs: diagnostics.db_average_latency_us / 1_000,
        lastLatencyMs: diagnostics.db_last_latency_us / 1_000,
      },
      network: {
        queueDepth: networkDiagnostics.queue_depth,
        queueDepthPeak: networkDiagnostics.queue_depth_peak,
        responses429: networkDiagnostics.responses_429,
        responses503: networkDiagnostics.responses_503,
        retries: networkDiagnostics.retries,
        lastError: networkDiagnostics.last_error ?? null,
      },
    }));
  } catch {
    // Best-effort telemetry refresh: keep previous values if backend is unavailable.
  }
}

export async function refreshPdgLiveApiSettings(): Promise<void> {
  try {
    const settings = await getPdgLiveApiSettings();
    pdgIntegrationState.update((state) => ({
      ...state,
      liveApiEnabled: settings.enabled,
      sourceArbitration: settings.enabled ? "mixed" : "local",
    }));
  } catch {
    // Ignore capability gaps in non-tauri environments.
  }
}

export async function setPdgLiveApiEnabled(enabled: boolean): Promise<void> {
  try {
    const settings = await setPdgLiveApiSettings({ enabled });
    pdgIntegrationState.update((state) => ({
      ...state,
      liveApiEnabled: settings.enabled,
      sourceArbitration: settings.enabled ? "mixed" : "local",
      authoritativeSource: settings.enabled ? "local+api" : "local_sqlite",
    }));
  } catch {
    // Ignore capability gaps in non-tauri environments.
  }
}

export function addPdgCitation(entry: PdgCitationEntry): void {
  pdgIntegrationState.update((state) => {
    if (state.citations.some((item) => item.key === entry.key)) {
      return state;
    }
    return {
      ...state,
      citations: [...state.citations, entry],
    };
  });
}

export function addPdgMeasurementCitation(record: PdgParticleRecord): void {
  const label = record.label ?? `PDG ${record.pdg_id}`;
  const edition = record.provenance.edition;
  addPdgCitation({
    key: `pdg-${record.pdg_id}-${edition}`,
    title: `${label} measurements (${edition})`,
    year: 2025,
    edition,
  });
}
