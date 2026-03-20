import { writable, derived, type Readable, type Writable } from 'svelte/store';
import type {
  PdgParticleRecord,
  PdgDecayChannel,
  PdgQuantumNumbers,
  PdgValue,
  PdgProvenance,
} from '$lib/types/spire';
import {
  searchParticles,
  getParticleProperties,
} from '$lib/api';

/**
 * Catalog mode for PDG particle browser
 * - core_sm: Standard Model particles only
 * - full_catalog: Complete PDG dataset (~1,100 particles)
 */
export const pdgCatalogMode: Writable<'core_sm' | 'full_catalog'> =
  writable('core_sm');

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
export const pdgCatalog: Readable<PdgParticleRecord[]> = derived(
  pdgCatalogMode,
  (mode, set) => {
    if (mode === 'core_sm') {
      set(CORE_SM_PARTICLES);
    } else {
      loadFullCatalog()
        .then((particles) => set(particles))
        .catch((err) => {
          console.error('Failed to load full PDG catalog:', err);
          set(CORE_SM_PARTICLES);
        });
    }
  },
  CORE_SM_PARTICLES
);

/**
 * Async function to load full PDG catalog from backend
 */
async function loadFullCatalog(): Promise<PdgParticleRecord[]> {
  try {
    const allParticles = [...CORE_SM_PARTICLES];
    // TODO: Extend with searchParticles('') call
    return allParticles;
  } catch (err) {
    console.error('loadFullCatalog error:', err);
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

/**
 * Apply facet filters to catalog
 */
function applyFilters(
  particles: PdgParticleRecord[],
  charge: 'all' | number[],
  spin: 'all' | string[],
  mass: { min: number; max: number }
): PdgParticleRecord[] {
  return particles.filter((p) => {
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
}

/**
 * Filtered catalog based on all facet filters
 */
export const filteredCatalog: Readable<PdgParticleRecord[]> = derived(
  [pdgCatalog, chargeFilter, spinFilter, massRange],
  ([$catalog, $charge, $spin, $mass]) => {
    return applyFilters($catalog, $charge, $spin, $mass);
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
}> = derived(pdgCatalog, ($catalog) => {
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
