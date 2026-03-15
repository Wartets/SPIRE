import type { Field, TheoreticalModel, ColorRepresentation } from "$lib/types/spire";

export type ParticleFamily =
  | "quark"
  | "lepton"
  | "neutrino"
  | "gauge_boson"
  | "scalar"
  | "tensor_or_higher"
  | "composite_or_other";

export interface TaxonomyBucket {
  key: ParticleFamily;
  label: string;
  particles: Field[];
}

export interface TaxonomyGroup {
  key: "sm" | "bsm" | "custom";
  label: string;
  buckets: TaxonomyBucket[];
  count: number;
}

export interface TaxonomyResult {
  groups: TaxonomyGroup[];
  total: number;
}

const SM_FIELD_IDS = new Set<string>([
  "e-", "e+", "mu-", "mu+", "tau-", "tau+",
  "nu_e", "nu_e_bar", "nu_mu", "nu_mu_bar", "nu_tau", "nu_tau_bar",
  "u", "u_bar", "d", "d_bar", "c", "c_bar", "s", "s_bar", "t", "t_bar", "b", "b_bar",
  "gamma", "g", "z", "z0", "w+", "w-", "h",
]);

const ORDER: ParticleFamily[] = [
  "quark",
  "lepton",
  "neutrino",
  "gauge_boson",
  "scalar",
  "tensor_or_higher",
  "composite_or_other",
];

const LABELS: Record<ParticleFamily, string> = {
  quark: "Quarks",
  lepton: "Leptons",
  neutrino: "Neutrinos",
  gauge_boson: "Gauge Bosons",
  scalar: "Scalars",
  tensor_or_higher: "Higher-Spin Fields",
  composite_or_other: "Composite / Other",
};

function toSafeMass(v: number): number {
  return Number.isFinite(v) ? Math.abs(v) : Number.POSITIVE_INFINITY;
}

function isColored(color: ColorRepresentation): boolean {
  return color === "Triplet" || color === "AntiTriplet" || color === "Octet";
}

function classifyFamily(field: Field): ParticleFamily {
  const qn = field.quantum_numbers;
  const spin = qn.spin;
  const charge = qn.electric_charge;
  const isColoredField = isColored(qn.color);
  const hasLeptonNumber =
    qn.lepton_numbers.electron !== 0 ||
    qn.lepton_numbers.muon !== 0 ||
    qn.lepton_numbers.tau !== 0;

  if (isColoredField && Math.abs(qn.baryon_number) > 0) {
    return "quark";
  }

  if (spin <= 1 && hasLeptonNumber) {
    return Math.abs(charge) < 1e-12 ? "neutrino" : "lepton";
  }

  if (spin === 2 && (field.id === "gamma" || field.id === "g" || field.id === "z" || field.id === "z0" || field.id === "w+" || field.id === "w-")) {
    return "gauge_boson";
  }

  if (spin === 2 && !isColoredField && !hasLeptonNumber && Math.abs(charge) < 1e-12) {
    return "gauge_boson";
  }

  if (spin === 0) {
    return "scalar";
  }

  if (spin >= 3) {
    return "tensor_or_higher";
  }

  return "composite_or_other";
}

function isCustomParticle(field: Field): boolean {
  const id = field.id.toLowerCase();
  return id.startsWith("custom_") || id.startsWith("x_") || field.name.toLowerCase().includes("custom");
}

function isStandardModelParticle(field: Field): boolean {
  return SM_FIELD_IDS.has(field.id.toLowerCase());
}

function sortFields(fields: Field[]): Field[] {
  return [...fields].sort((a, b) => {
    const massCmp = toSafeMass(a.mass) - toSafeMass(b.mass);
    if (massCmp !== 0) return massCmp;

    const qCmp = Math.abs(a.quantum_numbers.electric_charge) - Math.abs(b.quantum_numbers.electric_charge);
    if (qCmp !== 0) return qCmp;

    const symbolCmp = a.symbol.localeCompare(b.symbol);
    if (symbolCmp !== 0) return symbolCmp;

    const nameCmp = a.name.localeCompare(b.name);
    if (nameCmp !== 0) return nameCmp;

    return a.id.localeCompare(b.id);
  });
}

function toBuckets(fields: Field[]): TaxonomyBucket[] {
  const byFamily = new Map<ParticleFamily, Field[]>();

  for (const field of fields) {
    const family = classifyFamily(field);
    const arr = byFamily.get(family) ?? [];
    arr.push(field);
    byFamily.set(family, arr);
  }

  const buckets: TaxonomyBucket[] = [];
  for (const family of ORDER) {
    const items = byFamily.get(family);
    if (!items || items.length === 0) continue;
    buckets.push({
      key: family,
      label: LABELS[family],
      particles: sortFields(items),
    });
  }

  return buckets;
}

export function categorizeParticles(model: TheoreticalModel | null): TaxonomyResult {
  if (!model) {
    return {
      groups: [],
      total: 0,
    };
  }

  const sm: Field[] = [];
  const bsm: Field[] = [];
  const custom: Field[] = [];

  for (const field of model.fields) {
    if (isCustomParticle(field)) {
      custom.push(field);
    } else if (isStandardModelParticle(field)) {
      sm.push(field);
    } else {
      bsm.push(field);
    }
  }

  const groups: TaxonomyGroup[] = [
    {
      key: "sm",
      label: "Standard Model",
      buckets: toBuckets(sm),
      count: sm.length,
    },
    {
      key: "bsm",
      label: "Beyond the Standard Model",
      buckets: toBuckets(bsm),
      count: bsm.length,
    },
    {
      key: "custom",
      label: "Custom Particles",
      buckets: toBuckets(custom),
      count: custom.length,
    },
  ];

  return {
    groups,
    total: model.fields.length,
  };
}
