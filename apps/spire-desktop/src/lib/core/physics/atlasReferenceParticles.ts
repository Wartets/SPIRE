import type { Field, InteractionType, QuantumNumbers } from "$lib/types/spire";

interface ReferenceFieldSeed {
  id: string;
  name: string;
  symbol: string;
  mass: number;
  width?: number;
  interactions: InteractionType[];
  qn: Partial<QuantumNumbers> & Pick<QuantumNumbers, "electric_charge" | "spin" | "color">;
}

function qn(seed: ReferenceFieldSeed["qn"]): QuantumNumbers {
  return {
    electric_charge: seed.electric_charge,
    weak_isospin: seed.weak_isospin ?? 0,
    hypercharge: seed.hypercharge ?? 0,
    baryon_number: seed.baryon_number ?? 0,
    lepton_numbers: seed.lepton_numbers ?? { electron: 0, muon: 0, tau: 0 },
    spin: seed.spin,
    parity: seed.parity ?? "Even",
    charge_conjugation: seed.charge_conjugation ?? "Undefined",
    color: seed.color,
    weak_multiplet: seed.weak_multiplet ?? "Singlet",
  };
}

function mk(seed: ReferenceFieldSeed): Field {
  return {
    id: seed.id,
    name: seed.name,
    symbol: seed.symbol,
    mass: seed.mass,
    width: seed.width ?? 0,
    interactions: seed.interactions,
    quantum_numbers: qn(seed.qn),
  };
}

const REFERENCE_FIELDS: Field[] = [
  // Leptons
  mk({ id: "e-", name: "Electron", symbol: "e⁻", mass: 5.1099895e-4, interactions: ["Electromagnetic", "WeakNC", "WeakCC"], qn: { electric_charge: -1, spin: 1, color: "Singlet", weak_isospin: -1, hypercharge: -1, lepton_numbers: { electron: 1, muon: 0, tau: 0 }, weak_multiplet: "DoubletDown", parity: "Odd" } }),
  mk({ id: "e+", name: "Positron", symbol: "e⁺", mass: 5.1099895e-4, interactions: ["Electromagnetic", "WeakNC", "WeakCC"], qn: { electric_charge: 1, spin: 1, color: "Singlet", weak_isospin: 1, hypercharge: 1, lepton_numbers: { electron: -1, muon: 0, tau: 0 }, weak_multiplet: "DoubletUp", parity: "Odd" } }),
  mk({ id: "mu-", name: "Muon", symbol: "μ⁻", mass: 0.105658, width: 2.996e-19, interactions: ["Electromagnetic", "WeakNC", "WeakCC"], qn: { electric_charge: -1, spin: 1, color: "Singlet", weak_isospin: -1, hypercharge: -1, lepton_numbers: { electron: 0, muon: 1, tau: 0 }, weak_multiplet: "DoubletDown", parity: "Odd" } }),
  mk({ id: "tau-", name: "Tau", symbol: "τ⁻", mass: 1.77686, width: 2.27e-12, interactions: ["Electromagnetic", "WeakNC", "WeakCC"], qn: { electric_charge: -1, spin: 1, color: "Singlet", weak_isospin: -1, hypercharge: -1, lepton_numbers: { electron: 0, muon: 0, tau: 1 }, weak_multiplet: "DoubletDown", parity: "Odd" } }),

  // Neutrinos
  mk({ id: "nu_e", name: "Electron Neutrino", symbol: "νₑ", mass: 1e-9, interactions: ["WeakNC", "WeakCC"], qn: { electric_charge: 0, spin: 1, color: "Singlet", weak_isospin: 1, hypercharge: -1, lepton_numbers: { electron: 1, muon: 0, tau: 0 }, weak_multiplet: "DoubletUp", parity: "Odd" } }),
  mk({ id: "nu_mu", name: "Muon Neutrino", symbol: "ν_μ", mass: 1e-9, interactions: ["WeakNC", "WeakCC"], qn: { electric_charge: 0, spin: 1, color: "Singlet", weak_isospin: 1, hypercharge: -1, lepton_numbers: { electron: 0, muon: 1, tau: 0 }, weak_multiplet: "DoubletUp", parity: "Odd" } }),
  mk({ id: "nu_tau", name: "Tau Neutrino", symbol: "ν_τ", mass: 1e-9, interactions: ["WeakNC", "WeakCC"], qn: { electric_charge: 0, spin: 1, color: "Singlet", weak_isospin: 1, hypercharge: -1, lepton_numbers: { electron: 0, muon: 0, tau: 1 }, weak_multiplet: "DoubletUp", parity: "Odd" } }),

  // Gauge + scalar
  mk({ id: "gamma", name: "Photon", symbol: "γ", mass: 0, interactions: ["Electromagnetic"], qn: { electric_charge: 0, spin: 2, color: "Singlet", parity: "Odd", charge_conjugation: "Odd" } }),
  mk({ id: "g", name: "Gluon", symbol: "g", mass: 0, interactions: ["Strong"], qn: { electric_charge: 0, spin: 2, color: "Octet", parity: "Odd" } }),
  mk({ id: "z", name: "Z Boson", symbol: "Z⁰", mass: 91.1876, width: 2.4952, interactions: ["WeakNC"], qn: { electric_charge: 0, spin: 2, color: "Singlet", weak_multiplet: { Triplet: 0 } } }),
  mk({ id: "w+", name: "W Boson", symbol: "W⁺", mass: 80.379, width: 2.085, interactions: ["WeakCC"], qn: { electric_charge: 1, spin: 2, color: "Singlet", weak_multiplet: { Triplet: 1 } } }),
  mk({ id: "w-", name: "W Boson", symbol: "W⁻", mass: 80.379, width: 2.085, interactions: ["WeakCC"], qn: { electric_charge: -1, spin: 2, color: "Singlet", weak_multiplet: { Triplet: -1 } } }),
  mk({ id: "h", name: "Higgs Boson", symbol: "H", mass: 125.25, width: 4.07e-3, interactions: ["Yukawa", "ScalarSelf", "WeakNC", "WeakCC"], qn: { electric_charge: 0, spin: 0, color: "Singlet", weak_multiplet: "Singlet" } }),

  // Quarks
  mk({ id: "u", name: "Up Quark", symbol: "u", mass: 0.00216, interactions: ["Strong", "Electromagnetic", "WeakCC", "WeakNC"], qn: { electric_charge: 2 / 3, spin: 1, color: "Triplet", baryon_number: 1, weak_isospin: 1, hypercharge: 1, weak_multiplet: "DoubletUp", parity: "Odd" } }),
  mk({ id: "d", name: "Down Quark", symbol: "d", mass: 0.00467, interactions: ["Strong", "Electromagnetic", "WeakCC", "WeakNC"], qn: { electric_charge: -1 / 3, spin: 1, color: "Triplet", baryon_number: 1, weak_isospin: -1, hypercharge: 1, weak_multiplet: "DoubletDown", parity: "Odd" } }),
  mk({ id: "s", name: "Strange Quark", symbol: "s", mass: 0.093, interactions: ["Strong", "Electromagnetic", "WeakCC", "WeakNC"], qn: { electric_charge: -1 / 3, spin: 1, color: "Triplet", baryon_number: 1, weak_isospin: -1, hypercharge: 1, weak_multiplet: "DoubletDown", parity: "Odd" } }),
  mk({ id: "c", name: "Charm Quark", symbol: "c", mass: 1.27, interactions: ["Strong", "Electromagnetic", "WeakCC", "WeakNC"], qn: { electric_charge: 2 / 3, spin: 1, color: "Triplet", baryon_number: 1, weak_isospin: 1, hypercharge: 1, weak_multiplet: "DoubletUp", parity: "Odd" } }),
  mk({ id: "b", name: "Bottom Quark", symbol: "b", mass: 4.18, interactions: ["Strong", "Electromagnetic", "WeakCC", "WeakNC"], qn: { electric_charge: -1 / 3, spin: 1, color: "Triplet", baryon_number: 1, weak_isospin: -1, hypercharge: 1, weak_multiplet: "DoubletDown", parity: "Odd" } }),
  mk({ id: "t", name: "Top Quark", symbol: "t", mass: 172.76, width: 1.41, interactions: ["Strong", "Electromagnetic", "WeakCC", "WeakNC", "Yukawa"], qn: { electric_charge: 2 / 3, spin: 1, color: "Triplet", baryon_number: 1, weak_isospin: 1, hypercharge: 1, weak_multiplet: "DoubletUp", parity: "Odd" } }),

  // Light hadrons
  mk({ id: "p", name: "Proton", symbol: "p", mass: 0.938272, interactions: ["Strong", "Electromagnetic", "WeakNC"], qn: { electric_charge: 1, spin: 1, color: "Singlet", baryon_number: 3, parity: "Even" } }),
  mk({ id: "n", name: "Neutron", symbol: "n", mass: 0.939565, width: 7.5e-28, interactions: ["Strong", "WeakCC", "WeakNC"], qn: { electric_charge: 0, spin: 1, color: "Singlet", baryon_number: 3, parity: "Even" } }),
  mk({ id: "pi+", name: "Charged Pion", symbol: "π⁺", mass: 0.13957, width: 2.53e-17, interactions: ["Strong", "WeakCC", "Electromagnetic"], qn: { electric_charge: 1, spin: 0, color: "Singlet", parity: "Odd" } }),
  mk({ id: "pi0", name: "Neutral Pion", symbol: "π⁰", mass: 0.134977, width: 7.81e-9, interactions: ["Strong", "Electromagnetic"], qn: { electric_charge: 0, spin: 0, color: "Singlet", parity: "Odd", charge_conjugation: "Even" } }),
  mk({ id: "k+", name: "Charged Kaon", symbol: "K⁺", mass: 0.493677, width: 5.32e-17, interactions: ["Strong", "WeakCC", "Electromagnetic"], qn: { electric_charge: 1, spin: 0, color: "Singlet", parity: "Odd" } }),
  mk({ id: "k0", name: "Neutral Kaon", symbol: "K⁰", mass: 0.497611, width: 1.3e-14, interactions: ["Strong", "WeakNC"], qn: { electric_charge: 0, spin: 0, color: "Singlet", parity: "Odd" } }),
  mk({ id: "lambda0", name: "Lambda Baryon", symbol: "Λ⁰", mass: 1.115683, width: 2.5e-15, interactions: ["Strong", "WeakNC"], qn: { electric_charge: 0, spin: 1, color: "Singlet", baryon_number: 3, parity: "Even" } }),
  mk({ id: "sigma+", name: "Sigma Plus", symbol: "Σ⁺", mass: 1.18937, width: 8.2e-15, interactions: ["Strong", "WeakNC"], qn: { electric_charge: 1, spin: 1, color: "Singlet", baryon_number: 3, parity: "Even" } }),
  mk({ id: "sigma0", name: "Sigma Zero", symbol: "Σ⁰", mass: 1.192642, width: 8.9e-6, interactions: ["Strong", "Electromagnetic"], qn: { electric_charge: 0, spin: 1, color: "Singlet", baryon_number: 3, parity: "Even" } }),
  mk({ id: "sigma-", name: "Sigma Minus", symbol: "Σ⁻", mass: 1.197449, width: 4.4e-15, interactions: ["Strong", "WeakNC"], qn: { electric_charge: -1, spin: 1, color: "Singlet", baryon_number: 3, parity: "Even" } }),
  mk({ id: "xi0", name: "Xi Zero", symbol: "Ξ⁰", mass: 1.31486, width: 2.27e-15, interactions: ["Strong", "WeakNC"], qn: { electric_charge: 0, spin: 1, color: "Singlet", baryon_number: 3, parity: "Even" } }),
  mk({ id: "xi-", name: "Xi Minus", symbol: "Ξ⁻", mass: 1.32171, width: 4.02e-15, interactions: ["Strong", "WeakNC"], qn: { electric_charge: -1, spin: 1, color: "Singlet", baryon_number: 3, parity: "Even" } }),
  mk({ id: "omega-", name: "Omega Minus", symbol: "Ω⁻", mass: 1.67245, width: 5.0e-15, interactions: ["Strong", "WeakNC"], qn: { electric_charge: -1, spin: 3, color: "Singlet", baryon_number: 3, parity: "Even" } }),

  // Resonances/exotics for atlas breadth
  mk({ id: "rho0", name: "Rho Meson", symbol: "ρ⁰", mass: 0.77526, width: 0.1491, interactions: ["Strong", "Electromagnetic"], qn: { electric_charge: 0, spin: 2, color: "Singlet", parity: "Odd", charge_conjugation: "Odd" } }),
  mk({ id: "phi_meson", name: "Phi Meson", symbol: "φ", mass: 1.019461, width: 0.004249, interactions: ["Strong", "Electromagnetic"], qn: { electric_charge: 0, spin: 2, color: "Singlet", parity: "Odd", charge_conjugation: "Odd" } }),
  mk({ id: "j_psi", name: "J/Psi", symbol: "J/ψ", mass: 3.0969, width: 9.29e-5, interactions: ["Strong", "Electromagnetic"], qn: { electric_charge: 0, spin: 2, color: "Singlet", parity: "Odd", charge_conjugation: "Odd" } }),
  mk({ id: "upsilon", name: "Upsilon", symbol: "ϒ", mass: 9.4603, width: 5.4e-5, interactions: ["Strong", "Electromagnetic"], qn: { electric_charge: 0, spin: 2, color: "Singlet", parity: "Odd", charge_conjugation: "Odd" } }),
  mk({ id: "x3872", name: "X(3872)", symbol: "X", mass: 3.8717, width: 0.0012, interactions: ["Strong"], qn: { electric_charge: 0, spin: 2, color: "Singlet", parity: "Even" } }),
  mk({ id: "z_c3900", name: "Zc(3900)", symbol: "Zc", mass: 3.899, width: 0.028, interactions: ["Strong"], qn: { electric_charge: 1, spin: 2, color: "Singlet", parity: "Even" } }),
  mk({ id: "theta_plus", name: "Θ⁺ Pentaquark", symbol: "Θ⁺", mass: 1.54, width: 0.015, interactions: ["Strong"], qn: { electric_charge: 1, spin: 1, color: "Singlet", baryon_number: 3, parity: "Even" } }),

  // BSM-style templates
  mk({ id: "a_prime", name: "Dark Photon", symbol: "A'", mass: 0.35, width: 1.2e-8, interactions: ["Electromagnetic", "WeakNC"], qn: { electric_charge: 0, spin: 2, color: "Singlet" } }),
  mk({ id: "z_prime", name: "Z Prime", symbol: "Z'", mass: 2000, width: 48, interactions: ["WeakNC"], qn: { electric_charge: 0, spin: 2, color: "Singlet" } }),
  mk({ id: "w_prime", name: "W Prime", symbol: "W'", mass: 2300, width: 62, interactions: ["WeakCC"], qn: { electric_charge: 1, spin: 2, color: "Singlet" } }),
  mk({ id: "h2", name: "Heavy Scalar H2", symbol: "H₂", mass: 620, width: 7.6, interactions: ["ScalarSelf", "Yukawa", "WeakNC"], qn: { electric_charge: 0, spin: 0, color: "Singlet" } }),
  mk({ id: "axion_like", name: "Axion-like Particle", symbol: "a", mass: 0.001, width: 2.1e-12, interactions: ["Electromagnetic"], qn: { electric_charge: 0, spin: 0, color: "Singlet", parity: "Odd" } }),
  mk({ id: "n_sterile", name: "Sterile Neutrino", symbol: "N", mass: 50, width: 0.004, interactions: ["WeakNC"], qn: { electric_charge: 0, spin: 1, color: "Singlet", lepton_numbers: { electron: 0, muon: 0, tau: 1 }, parity: "Odd" } }),
  mk({ id: "chi0", name: "Neutralino-like χ0", symbol: "χ⁰", mass: 120, width: 0, interactions: ["WeakNC"], qn: { electric_charge: 0, spin: 1, color: "Singlet" } }),
  mk({ id: "gluino", name: "Gluino", symbol: "g̃", mass: 1800, width: 7.5, interactions: ["Strong"], qn: { electric_charge: 0, spin: 1, color: "Octet" } }),
  mk({ id: "graviton", name: "Kaluza-Klein Graviton", symbol: "G*", mass: 3000, width: 110, interactions: ["WeakNC", "Electromagnetic", "Strong"], qn: { electric_charge: 0, spin: 4, color: "Singlet" } }),
  mk({ id: "leptoquark_u1", name: "Vector Leptoquark U1", symbol: "U₁", mass: 1600, width: 32, interactions: ["Strong", "WeakNC", "WeakCC"], qn: { electric_charge: 2 / 3, spin: 2, color: "Triplet", baryon_number: 1, lepton_numbers: { electron: -1, muon: 0, tau: 0 } } }),
];

export function listAtlasReferenceParticles(): Field[] {
  return [...REFERENCE_FIELDS].sort((a, b) => a.mass - b.mass || a.name.localeCompare(b.name));
}
