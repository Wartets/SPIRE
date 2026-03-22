import { writable, derived, type Readable, type Writable } from 'svelte/store';
import type {
  PdgMetadata,
  PdgParticleRecord,
  PdgDecayChannel,
} from '$lib/types/spire';
import {
  getPdgMetadata,
  beginPdgCatalogStream,
  cancelPdgCatalogStream,
  searchParticles,
  searchParticlesChunked,
} from '$lib/api';

/**
 * Catalog mode for PDG particle browser
 * - core_sm: Standard Model particles only
 * - full_catalog: Complete PDG dataset (~1,100 particles)
 */
export const pdgCatalogMode: Writable<'core_sm' | 'full_catalog'> =
  writable('core_sm');

export const pdgMetadata: Writable<PdgMetadata | null> = writable(null);
export const pdgMetadataError: Writable<string | null> = writable(null);

export type PdgSortMode = 'pdg_asc' | 'pdg_desc' | 'alpha' | 'mass_asc' | 'mass_desc' | 'charge_abs';

export const pdgSortMode: Writable<PdgSortMode> = writable('pdg_asc');
export const pdgEditionFilter: Writable<'latest' | string> = writable('latest');
export const pdgOnlyWithMeasurements: Writable<boolean> = writable(false);
export const pdgAvailableEditions: Writable<string[]> = writable([]);
export const pdgCatalogHistory: Writable<Record<number, PdgParticleRecord[]>> = writable({});
export const pdgCatalogLoadProgress: Writable<{
  active: boolean;
  loaded: number;
  total: number;
  cancelled: boolean;
  requestId: string | null;
}> = writable({
  active: false,
  loaded: 0,
  total: 0,
  cancelled: false,
  requestId: null,
});

function editionRank(edition: string): number {
  const yearMatch = edition.match(/(19|20)\d{2}/);
  if (yearMatch) return Number(yearMatch[0]);
  return Number.MIN_SAFE_INTEGER;
}

function rankHistory(history: PdgParticleRecord[]): PdgParticleRecord[] {
  return [...history].sort((a, b) => {
    const yearDelta = editionRank(b.provenance.edition) - editionRank(a.provenance.edition);
    if (yearDelta !== 0) return yearDelta;
    return a.provenance.source_id.localeCompare(b.provenance.source_id);
  });
}

function hasMeasurements(record: PdgParticleRecord): boolean {
  return Boolean(record.mass || record.width || record.lifetime);
}

export async function refreshPdgMetadata(): Promise<void> {
  try {
    const metadata = await getPdgMetadata();
    pdgMetadata.set(metadata);
    pdgMetadataError.set(null);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    pdgMetadataError.set(message);
    pdgMetadata.set(null);
  }
}

/**
 * Core Standard Model particles (bootstrap set)
 * 13 particles: photon, Z, W, Higgs, 6 leptons, 3 neutrinos, 6 quarks
 */
const CORE_SM_PARTICLES: PdgParticleRecord[] = [
  // Gauge bosons
  {
    pdg_id: 22,
    label: 'photon',
    quantum_numbers: {
      charge: 0,
      spin_j: '1',
      parity: '-1',
    },
    mass: undefined,
    width: undefined,
    lifetime: undefined,
    branching_fractions: [],
    provenance: { edition: 'PDG2024', source_id: 'core_sm', fingerprint: 'photon' },
  },
  {
    pdg_id: 23,
    label: 'Z boson',
    quantum_numbers: {
      charge: 0,
      spin_j: '1',
      parity: '-1',
    },
    mass: { kind: 'symmetric', value: 91.188, error: 0.002 },
    width: { kind: 'symmetric', value: 2.495, error: 0.003 },
    lifetime: undefined,
    branching_fractions: [],
    provenance: { edition: 'PDG2024', source_id: 'core_sm', fingerprint: 'z_boson' },
  },
  {
    pdg_id: 24,
    label: 'W boson',
    quantum_numbers: {
      charge: 1,
      spin_j: '1',
      parity: '-1',
    },
    mass: { kind: 'symmetric', value: 80.379, error: 0.012 },
    width: { kind: 'symmetric', value: 2.085, error: 0.042 },
    lifetime: undefined,
    branching_fractions: [],
    provenance: { edition: 'PDG2024', source_id: 'core_sm', fingerprint: 'w_boson' },
  },
  {
    pdg_id: 25,
    label: 'Higgs boson',
    quantum_numbers: {
      charge: 0,
      spin_j: '0',
      parity: '1',
    },
    mass: { kind: 'symmetric', value: 125.10, error: 0.14 },
    width: { kind: 'symmetric', value: 0.004, error: 0.0003 },
    lifetime: undefined,
    branching_fractions: [],
    provenance: { edition: 'PDG2024', source_id: 'core_sm', fingerprint: 'higgs' },
  },

  // Leptons
  {
    pdg_id: 11,
    label: 'electron',
    quantum_numbers: {
      charge: -1,
      spin_j: '1/2',
      parity: '-1',
    },
    mass: { kind: 'exact', value: 0.5109989461 },
    width: undefined,
    lifetime: undefined,
    branching_fractions: [],
    provenance: { edition: 'PDG2024', source_id: 'core_sm', fingerprint: 'electron' },
  },
  {
    pdg_id: 13,
    label: 'muon',
    quantum_numbers: {
      charge: -1,
      spin_j: '1/2',
      parity: '-1',
    },
    mass: { kind: 'symmetric', value: 105.6583745, error: 0.0000024 },
    width: undefined,
    lifetime: { kind: 'symmetric', value: 2.1969811e-6, error: 0.0000030e-6 },
    branching_fractions: [],
    provenance: { edition: 'PDG2024', source_id: 'core_sm', fingerprint: 'muon' },
  },
  {
    pdg_id: 15,
    label: 'tau',
    quantum_numbers: {
      charge: -1,
      spin_j: '1/2',
      parity: '-1',
    },
    mass: { kind: 'symmetric', value: 1776.86, error: 0.12 },
    width: undefined,
    lifetime: { kind: 'symmetric', value: 2.903e-13, error: 0.004e-13 },
    branching_fractions: [],
    provenance: { edition: 'PDG2024', source_id: 'core_sm', fingerprint: 'tau' },
  },

  // Neutrinos
  {
    pdg_id: 12,
    label: 'electron neutrino',
    quantum_numbers: {
      charge: 0,
      spin_j: '1/2',
      parity: '-1',
    },
    mass: { kind: 'exact', value: 0.0 },
    width: undefined,
    lifetime: undefined,
    branching_fractions: [],
    provenance: { edition: 'PDG2024', source_id: 'core_sm', fingerprint: 'nu_e' },
  },
  {
    pdg_id: 14,
    label: 'muon neutrino',
    quantum_numbers: {
      charge: 0,
      spin_j: '1/2',
      parity: '-1',
    },
    mass: { kind: 'exact', value: 0.0 },
    width: undefined,
    lifetime: undefined,
    branching_fractions: [],
    provenance: { edition: 'PDG2024', source_id: 'core_sm', fingerprint: 'nu_mu' },
  },
  {
    pdg_id: 16,
    label: 'tau neutrino',
    quantum_numbers: {
      charge: 0,
      spin_j: '1/2',
      parity: '-1',
    },
    mass: { kind: 'exact', value: 0.0 },
    width: undefined,
    lifetime: undefined,
    branching_fractions: [],
    provenance: { edition: 'PDG2024', source_id: 'core_sm', fingerprint: 'nu_tau' },
  },

  // Quarks
  {
    pdg_id: 1,
    label: 'down quark',
    quantum_numbers: {
      charge: -1 / 3,
      spin_j: '1/2',
      parity: '-1',
    },
    mass: { kind: 'symmetric', value: 4.67, error: 0.48 },
    width: undefined,
    lifetime: undefined,
    branching_fractions: [],
    provenance: { edition: 'PDG2024', source_id: 'core_sm', fingerprint: 'd_quark' },
  },
  {
    pdg_id: 2,
    label: 'up quark',
    quantum_numbers: {
      charge: 2 / 3,
      spin_j: '1/2',
      parity: '-1',
    },
    mass: { kind: 'symmetric', value: 2.16, error: 0.49 },
    width: undefined,
    lifetime: undefined,
    branching_fractions: [],
    provenance: { edition: 'PDG2024', source_id: 'core_sm', fingerprint: 'u_quark' },
  },
  {
    pdg_id: 3,
    label: 'strange quark',
    quantum_numbers: {
      charge: -1 / 3,
      spin_j: '1/2',
      parity: '-1',
    },
    mass: { kind: 'symmetric', value: 93.1, error: 5.8 },
    width: undefined,
    lifetime: undefined,
    branching_fractions: [],
    provenance: { edition: 'PDG2024', source_id: 'core_sm', fingerprint: 's_quark' },
  },
  {
    pdg_id: 4,
    label: 'charm quark',
    quantum_numbers: {
      charge: 2 / 3,
      spin_j: '1/2',
      parity: '-1',
    },
    mass: { kind: 'symmetric', value: 1275, error: 25 },
    width: undefined,
    lifetime: undefined,
    branching_fractions: [],
    provenance: { edition: 'PDG2024', source_id: 'core_sm', fingerprint: 'c_quark' },
  },
  {
    pdg_id: 5,
    label: 'bottom quark',
    quantum_numbers: {
      charge: -1 / 3,
      spin_j: '1/2',
      parity: '-1',
    },
    mass: { kind: 'symmetric', value: 4180, error: 30 },
    width: undefined,
    lifetime: undefined,
    branching_fractions: [],
    provenance: { edition: 'PDG2024', source_id: 'core_sm', fingerprint: 'b_quark' },
  },
  {
    pdg_id: 6,
    label: 'top quark',
    quantum_numbers: {
      charge: 2 / 3,
      spin_j: '1/2',
      parity: '-1',
    },
    mass: { kind: 'symmetric', value: 173210, error: 540 },
    width: { kind: 'symmetric', value: 1.42, error: 0.19 },
    lifetime: undefined,
    branching_fractions: [],
    provenance: { edition: 'PDG2024', source_id: 'core_sm', fingerprint: 't_quark' },
  },
];

/**
 * Current PDG catalog based on mode
 */
let activeCatalogRequestId = 0;

export const pdgCatalog: Readable<PdgParticleRecord[]> = derived(
  pdgCatalogMode,
  (mode, set) => {
    const requestId = ++activeCatalogRequestId;
    let disposed = false;
    const signal = { cancelled: false, requestId: null as string | null };

    if (mode === 'core_sm') {
      const history = CORE_SM_PARTICLES.reduce<Record<number, PdgParticleRecord[]>>((acc, particle) => {
        acc[particle.pdg_id] = [particle];
        return acc;
      }, {});
      pdgCatalogHistory.set(history);
      pdgAvailableEditions.set(['PDG2024']);
      pdgCatalogLoadProgress.set({
        active: false,
        loaded: CORE_SM_PARTICLES.length,
        total: CORE_SM_PARTICLES.length,
        cancelled: false,
        requestId: null,
      });
      set(CORE_SM_PARTICLES);
    } else {
      loadFullCatalogProgressive(signal)
        .then(({ particles, historyByPdgId, editions }) => {
          if (disposed || requestId !== activeCatalogRequestId) return;
          pdgCatalogHistory.set(historyByPdgId);
          pdgAvailableEditions.set(editions);
          set(particles);
        })
        .catch((err) => {
          if (disposed || requestId !== activeCatalogRequestId) return;
          console.error('Failed to load full PDG catalog:', err);
          pdgCatalogHistory.set({});
          pdgAvailableEditions.set(['PDG2024']);
          set(CORE_SM_PARTICLES);
        });
    }

    return () => {
      disposed = true;
      signal.cancelled = true;
      if (signal.requestId) {
        void cancelPdgCatalogStream(signal.requestId);
      }
    };
  },
  CORE_SM_PARTICLES
);

function buildCatalogFromRecords(fetched: PdgParticleRecord[]): {
  particles: PdgParticleRecord[];
  historyByPdgId: Record<number, PdgParticleRecord[]>;
  editions: string[];
} {
  if (!Array.isArray(fetched) || fetched.length === 0) {
    return {
      particles: [...CORE_SM_PARTICLES],
      historyByPdgId: CORE_SM_PARTICLES.reduce<Record<number, PdgParticleRecord[]>>((acc, particle) => {
        acc[particle.pdg_id] = [particle];
        return acc;
      }, {}),
      editions: ['PDG2024'],
    };
  }

  const historyByPdgId = new Map<number, PdgParticleRecord[]>();
  const editions = new Set<string>();

  for (const particle of [...CORE_SM_PARTICLES, ...fetched]) {
    editions.add(particle.provenance.edition);
    const current = historyByPdgId.get(particle.pdg_id) ?? [];
    current.push(particle);
    historyByPdgId.set(particle.pdg_id, current);
  }

  const reducedHistory: Record<number, PdgParticleRecord[]> = {};
  const latestCatalog = Array.from(historyByPdgId.entries()).map(([pdgId, history]) => {
    const ranked = rankHistory(history);
    reducedHistory[pdgId] = ranked;
    return ranked[0];
  });

  return {
    particles: latestCatalog.sort((a, b) => a.pdg_id - b.pdg_id),
    historyByPdgId: reducedHistory,
    editions: [...editions].sort((a, b) => editionRank(b) - editionRank(a)),
  };
}

/**
 * Async function to load full PDG catalog progressively from backend.
 */
async function loadFullCatalogProgressive(signal: { cancelled: boolean; requestId: string | null }): Promise<{
  particles: PdgParticleRecord[];
  historyByPdgId: Record<number, PdgParticleRecord[]>;
  editions: string[];
}> {
  const chunkSize = 128;
  let requestId: string | null = null;

  try {
    requestId = await beginPdgCatalogStream();
    signal.requestId = requestId;
    pdgCatalogLoadProgress.set({
      active: true,
      loaded: 0,
      total: 0,
      cancelled: false,
      requestId,
    });

    const fetched: PdgParticleRecord[] = [];
    let offset = 0;
    let total = 0;

    while (!signal.cancelled) {
      const chunk = await searchParticlesChunked('', offset, chunkSize, requestId);
      if (chunk.cancelled) {
        pdgCatalogLoadProgress.set({
          active: false,
          loaded: fetched.length,
          total: total || chunk.total,
          cancelled: true,
          requestId,
        });
        throw new Error('PDG catalog load cancelled');
      }

      fetched.push(...chunk.records);
      offset += chunk.records.length;
      total = chunk.total;

      pdgCatalogLoadProgress.set({
        active: !chunk.done,
        loaded: fetched.length,
        total,
        cancelled: false,
        requestId,
      });

      if (chunk.done || chunk.records.length === 0) {
        break;
      }

      await Promise.resolve();
    }

    if (signal.cancelled) {
      if (requestId) {
        await cancelPdgCatalogStream(requestId);
      }
      pdgCatalogLoadProgress.set({
        active: false,
        loaded: 0,
        total: 0,
        cancelled: true,
        requestId,
      });
      throw new Error('PDG catalog load cancelled');
    }

    const catalog = buildCatalogFromRecords(fetched);
    pdgCatalogLoadProgress.set({
      active: false,
      loaded: catalog.particles.length,
      total: total || catalog.particles.length,
      cancelled: false,
      requestId,
    });
    return catalog;
  } catch (err) {
    if (!signal.cancelled) {
      try {
        const fallback = await searchParticles('');
        const catalog = buildCatalogFromRecords(fallback);
        pdgCatalogLoadProgress.set({
          active: false,
          loaded: catalog.particles.length,
          total: catalog.particles.length,
          cancelled: false,
          requestId,
        });
        return catalog;
      } catch (fallbackErr) {
        console.error('loadFullCatalogProgressive fallback error:', fallbackErr);
      }
    }

    console.error('loadFullCatalogProgressive error:', err);
    throw err;
  }
}

/**
 * Facet filters for PDG catalog
 */
export const chargeFilter: Writable<'all' | number[]> = writable('all');
export const spinFilter: Writable<'all' | string[]> = writable('all');
export const massRange: Writable<{ min: number; max: number }> = writable({
  min: 0,
  max: 1000000,
});

function materializeEditionCatalog(
  latestCatalog: PdgParticleRecord[],
  historyByPdgId: Record<number, PdgParticleRecord[]>,
  editionFilter: 'latest' | string,
): PdgParticleRecord[] {
  if (editionFilter === 'latest') {
    return latestCatalog;
  }

  return latestCatalog
    .map((entry) => {
      const history = historyByPdgId[entry.pdg_id] ?? [entry];
      return history.find((candidate) => candidate.provenance.edition === editionFilter) ?? entry;
    });
}

export const pdgEffectiveCatalog: Readable<PdgParticleRecord[]> = derived(
  [pdgCatalog, pdgCatalogHistory, pdgEditionFilter],
  ([$catalog, $history, $editionFilter]) => materializeEditionCatalog($catalog, $history, $editionFilter),
  CORE_SM_PARTICLES,
);

/**
 * Apply facet filters to catalog
 */
function applyFilters(
  particles: PdgParticleRecord[],
  charge: 'all' | number[],
  spin: 'all' | string[],
  mass: { min: number; max: number },
  onlyWithMeasurements: boolean,
  sortMode: PdgSortMode,
): PdgParticleRecord[] {
  const filtered = particles.filter((p) => {
    if (onlyWithMeasurements && !hasMeasurements(p)) {
      return false;
    }

    if (charge !== 'all' && p.quantum_numbers.charge !== undefined) {
      if (!charge.includes(p.quantum_numbers.charge)) {
        return false;
      }
    }

    if (spin !== 'all' && p.quantum_numbers.spin_j !== undefined) {
      if (!spin.includes(p.quantum_numbers.spin_j)) {
        return false;
      }
    }

    const mass_val = p.mass?.kind === 'exact' || p.mass?.kind === 'symmetric' || p.mass?.kind === 'asymmetric' 
      ? p.mass.value 
      : 0;
    if (mass_val < mass.min || mass_val > mass.max) {
      return false;
    }

    return true;
  });

  return filtered.sort((a, b) => {
    const massA = a.mass?.value ?? Number.POSITIVE_INFINITY;
    const massB = b.mass?.value ?? Number.POSITIVE_INFINITY;
    const chargeA = Math.abs(a.quantum_numbers.charge ?? 0);
    const chargeB = Math.abs(b.quantum_numbers.charge ?? 0);

    switch (sortMode) {
      case 'pdg_desc':
        return b.pdg_id - a.pdg_id;
      case 'alpha':
        return (a.label ?? '').localeCompare(b.label ?? '') || a.pdg_id - b.pdg_id;
      case 'mass_asc':
        return massA - massB || a.pdg_id - b.pdg_id;
      case 'mass_desc':
        return massB - massA || a.pdg_id - b.pdg_id;
      case 'charge_abs':
        return chargeB - chargeA || a.pdg_id - b.pdg_id;
      default:
        return a.pdg_id - b.pdg_id;
    }
  });
}

/**
 * Filtered catalog based on all facet filters
 */
export const filteredCatalog: Readable<PdgParticleRecord[]> = derived(
  [pdgEffectiveCatalog, chargeFilter, spinFilter, massRange, pdgOnlyWithMeasurements, pdgSortMode],
  ([$catalog, $charge, $spin, $mass, $onlyWithMeasurements, $sortMode]) => {
    return applyFilters($catalog, $charge, $spin, $mass, $onlyWithMeasurements, $sortMode);
  }
);

/**
 * Search query input
 */
export const pdgSearchQuery: Writable<string> = writable('');

/**
 * Fuzzy search function for particle properties
 */
function fuzzySearch(query: string, particles: PdgParticleRecord[]): PdgParticleRecord[] {
  if (!query.trim()) {
    return particles;
  }

  const q = query.trim().toLowerCase();
  return particles.filter((p) => {
    const label = p.label || '';
    return (
      label.toLowerCase().includes(q) ||
      p.pdg_id.toString().includes(q)
    );
  });
}

/**
 * Debounced search results (300ms)
 */
export const pdgSearchResults: Readable<PdgParticleRecord[]> = derived(
  [pdgSearchQuery, filteredCatalog],
  ([$query, $filtered], set) => {
    const debounceTimer = setTimeout(() => {
      const results = fuzzySearch($query, $filtered);
      set(results);
    }, 300);

    return () => clearTimeout(debounceTimer);
  },
  [] as PdgParticleRecord[]
);

/**
 * Selected particle for drill-down detail panel
 */
export const selectedPdgParticle: Writable<PdgParticleRecord | null> = writable(null);

let lastSelectedPdgId: number | null = null;

selectedPdgParticle.subscribe((value) => {
  lastSelectedPdgId = value?.pdg_id ?? null;
});

pdgSearchResults.subscribe((results) => {
  if (results.length === 0) {
    selectedPdgParticle.set(null);
    return;
  }

  const current = lastSelectedPdgId;
  if (current === null) return;
  const match = results.find((particle) => particle.pdg_id === current);
  if (match) {
    selectedPdgParticle.set(match);
  } else {
    selectedPdgParticle.set(results[0]);
  }
});

/**
 * Selected decay channel for detail inspection
 */
export const selectedPdgDecayChannel: Writable<PdgDecayChannel | null> = writable(null);

/**
 * Compute facet statistics from current catalog
 */
export const pdgFacetStats: Readable<{
  chargeDistribution: Record<number, number>;
  spinDistribution: Record<string, number>;
  massRange: { min: number; max: number };
}> = derived(pdgEffectiveCatalog, ($catalog) => {
  const chargeMap: Record<number, number> = {};
  const spinMap: Record<string, number> = {};
  let minMass = Infinity;
  let maxMass = 0;

  for (const p of $catalog) {
    if (p.quantum_numbers.charge !== undefined) {
      const charge = p.quantum_numbers.charge;
      chargeMap[charge] = (chargeMap[charge] ?? 0) + 1;
    }

    if (p.quantum_numbers.spin_j !== undefined) {
      const spin = p.quantum_numbers.spin_j;
      spinMap[spin] = (spinMap[spin] ?? 0) + 1;
    }

    const mass = p.mass?.kind === 'exact' || p.mass?.kind === 'symmetric' || p.mass?.kind === 'asymmetric'
      ? p.mass.value
      : 0;
    minMass = Math.min(minMass, mass);
    maxMass = Math.max(maxMass, mass);
  }

  return {
    chargeDistribution: chargeMap,
    spinDistribution: spinMap,
    massRange: { min: minMass === Infinity ? 0 : minMass, max: maxMass },
  };
});
