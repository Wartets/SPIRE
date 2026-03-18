/**
 * SPIRE Default Data Constants
 *
 * Embedded copies of the Standard Model particle and vertex definitions.
 * These TOML strings are pre-loaded into the Model Loader so the user
 * can immediately start running reactions without uploading files.
 */

// ---------------------------------------------------------------------------
// Standard Model Particles (22 particles)
// ---------------------------------------------------------------------------

export const DEFAULT_PARTICLES_TOML = `# SPIRE Particle Data - Standard Model
#
# Quantum Number Conventions (scaled integers to avoid fractions):
#   spin            - 2J  (e.g., 1 for spin-½, 2 for spin-1)
#   electric_charge - 3Q  (e.g., -3 for Q = -1, 2 for Q = +2/3)
#   weak_isospin    - 2T₃ (e.g., -1 for T₃ = -½)
#   hypercharge     - 3Y  (e.g., -3 for Y = -1)
#   baryon_number   - 3B  (e.g., 1 for B = +1/3)
#   lepton_numbers  - [L_e, L_μ, L_τ] (no scaling)

# ============================================================================
# LEPTONS
# ============================================================================

[[particle]]
id              = "e-"
name            = "Electron"
symbol          = "e^-"
mass            = 0.000511
width           = 0.0
spin            = 1
electric_charge = -3
weak_isospin    = -1
hypercharge     = -3
baryon_number   = 0
lepton_numbers  = [1, 0, 0]
parity          = "Even"
charge_conjugation = "Undefined"
color           = "Singlet"
weak_multiplet  = "DoubletDown"
interactions    = ["Electromagnetic", "WeakCC", "WeakNC"]

[[particle]]
id              = "e+"
name            = "Positron"
symbol          = "e^+"
mass            = 0.000511
width           = 0.0
spin            = 1
electric_charge = 3
weak_isospin    = 1
hypercharge     = 3
baryon_number   = 0
lepton_numbers  = [-1, 0, 0]
parity          = "Even"
charge_conjugation = "Undefined"
color           = "Singlet"
weak_multiplet  = "DoubletUp"
interactions    = ["Electromagnetic", "WeakCC", "WeakNC"]

[[particle]]
id              = "mu-"
name            = "Muon"
symbol          = "\\\\mu^-"
mass            = 0.10566
width           = 0.0
spin            = 1
electric_charge = -3
weak_isospin    = -1
hypercharge     = -3
baryon_number   = 0
lepton_numbers  = [0, 1, 0]
parity          = "Even"
charge_conjugation = "Undefined"
color           = "Singlet"
weak_multiplet  = "DoubletDown"
interactions    = ["Electromagnetic", "WeakCC", "WeakNC"]

[[particle]]
id              = "mu+"
name            = "Antimuon"
symbol          = "\\\\mu^+"
mass            = 0.10566
width           = 0.0
spin            = 1
electric_charge = 3
weak_isospin    = 1
hypercharge     = 3
baryon_number   = 0
lepton_numbers  = [0, -1, 0]
parity          = "Even"
charge_conjugation = "Undefined"
color           = "Singlet"
weak_multiplet  = "DoubletUp"
interactions    = ["Electromagnetic", "WeakCC", "WeakNC"]

[[particle]]
id              = "tau-"
name            = "Tau"
symbol          = "\\\\tau^-"
mass            = 1.777
width           = 0.0
spin            = 1
electric_charge = -3
weak_isospin    = -1
hypercharge     = -3
baryon_number   = 0
lepton_numbers  = [0, 0, 1]
parity          = "Even"
charge_conjugation = "Undefined"
color           = "Singlet"
weak_multiplet  = "DoubletDown"
interactions    = ["Electromagnetic", "WeakCC", "WeakNC"]

[[particle]]
id              = "tau+"
name            = "Antitau"
symbol          = "\\\\tau^+"
mass            = 1.777
width           = 0.0
spin            = 1
electric_charge = 3
weak_isospin    = 1
hypercharge     = 3
baryon_number   = 0
lepton_numbers  = [0, 0, -1]
parity          = "Even"
charge_conjugation = "Undefined"
color           = "Singlet"
weak_multiplet  = "DoubletUp"
interactions    = ["Electromagnetic", "WeakCC", "WeakNC"]

[[particle]]
id              = "nu_e"
name            = "Electron Neutrino"
symbol          = "\\\\nu_e"
mass            = 0.0
width           = 0.0
spin            = 1
electric_charge = 0
weak_isospin    = 1
hypercharge     = -3
baryon_number   = 0
lepton_numbers  = [1, 0, 0]
parity          = "Even"
charge_conjugation = "Undefined"
color           = "Singlet"
weak_multiplet  = "DoubletUp"
interactions    = ["WeakCC", "WeakNC"]

[[particle]]
id              = "nu_e_bar"
name            = "Electron Antineutrino"
symbol          = "\\\\bar{\\\\nu}_e"
mass            = 0.0
width           = 0.0
spin            = 1
electric_charge = 0
weak_isospin    = -1
hypercharge     = 3
baryon_number   = 0
lepton_numbers  = [-1, 0, 0]
parity          = "Even"
charge_conjugation = "Undefined"
color           = "Singlet"
weak_multiplet  = "DoubletDown"
interactions    = ["WeakCC", "WeakNC"]

[[particle]]
id              = "nu_mu"
name            = "Muon Neutrino"
symbol          = "\\\\nu_\\\\mu"
mass            = 0.0
width           = 0.0
spin            = 1
electric_charge = 0
weak_isospin    = 1
hypercharge     = -3
baryon_number   = 0
lepton_numbers  = [0, 1, 0]
parity          = "Even"
charge_conjugation = "Undefined"
color           = "Singlet"
weak_multiplet  = "DoubletUp"
interactions    = ["WeakCC", "WeakNC"]

[[particle]]
id              = "nu_mu_bar"
name            = "Muon Antineutrino"
symbol          = "\\\\bar{\\\\nu}_\\\\mu"
mass            = 0.0
width           = 0.0
spin            = 1
electric_charge = 0
weak_isospin    = -1
hypercharge     = 3
baryon_number   = 0
lepton_numbers  = [0, -1, 0]
parity          = "Even"
charge_conjugation = "Undefined"
color           = "Singlet"
weak_multiplet  = "DoubletDown"
interactions    = ["WeakCC", "WeakNC"]

# ============================================================================
# QUARKS (first generation)
# ============================================================================

[[particle]]
id              = "u"
name            = "Up Quark"
symbol          = "u"
mass            = 0.0023
width           = 0.0
spin            = 1
electric_charge = 2
weak_isospin    = 1
hypercharge     = 1
baryon_number   = 1
lepton_numbers  = [0, 0, 0]
parity          = "Even"
charge_conjugation = "Undefined"
color           = "Triplet"
weak_multiplet  = "DoubletUp"
interactions    = ["Electromagnetic", "WeakCC", "WeakNC", "Strong"]

[[particle]]
id              = "u_bar"
name            = "Anti-Up Quark"
symbol          = "\\\\bar{u}"
mass            = 0.0023
width           = 0.0
spin            = 1
electric_charge = -2
weak_isospin    = -1
hypercharge     = -1
baryon_number   = -1
lepton_numbers  = [0, 0, 0]
parity          = "Even"
charge_conjugation = "Undefined"
color           = "AntiTriplet"
weak_multiplet  = "DoubletDown"
interactions    = ["Electromagnetic", "WeakCC", "WeakNC", "Strong"]

[[particle]]
id              = "d"
name            = "Down Quark"
symbol          = "d"
mass            = 0.0048
width           = 0.0
spin            = 1
electric_charge = -1
weak_isospin    = -1
hypercharge     = 1
baryon_number   = 1
lepton_numbers  = [0, 0, 0]
parity          = "Even"
charge_conjugation = "Undefined"
color           = "Triplet"
weak_multiplet  = "DoubletDown"
interactions    = ["Electromagnetic", "WeakCC", "WeakNC", "Strong"]

[[particle]]
id              = "d_bar"
name            = "Anti-Down Quark"
symbol          = "\\\\bar{d}"
mass            = 0.0048
width           = 0.0
spin            = 1
electric_charge = 1
weak_isospin    = 1
hypercharge     = -1
baryon_number   = -1
lepton_numbers  = [0, 0, 0]
parity          = "Even"
charge_conjugation = "Undefined"
color           = "AntiTriplet"
weak_multiplet  = "DoubletUp"
interactions    = ["Electromagnetic", "WeakCC", "WeakNC", "Strong"]

# ============================================================================
# GAUGE BOSONS
# ============================================================================

[[particle]]
id              = "photon"
name            = "Photon"
symbol          = "\\\\gamma"
mass            = 0.0
width           = 0.0
spin            = 2
electric_charge = 0
weak_isospin    = 0
hypercharge     = 0
baryon_number   = 0
lepton_numbers  = [0, 0, 0]
parity          = "Odd"
charge_conjugation = "Odd"
color           = "Singlet"
weak_multiplet  = "Singlet"
interactions    = ["Electromagnetic"]

[[particle]]
id              = "W+"
name            = "W+ Boson"
symbol          = "W^+"
mass            = 80.379
width           = 2.085
spin            = 2
electric_charge = 3
weak_isospin    = 2
hypercharge     = 0
baryon_number   = 0
lepton_numbers  = [0, 0, 0]
parity          = "Odd"
charge_conjugation = "Undefined"
color           = "Singlet"
weak_multiplet  = "Triplet(1)"
interactions    = ["Electromagnetic", "WeakCC"]

[[particle]]
id              = "W-"
name            = "W- Boson"
symbol          = "W^-"
mass            = 80.379
width           = 2.085
spin            = 2
electric_charge = -3
weak_isospin    = -2
hypercharge     = 0
baryon_number   = 0
lepton_numbers  = [0, 0, 0]
parity          = "Odd"
charge_conjugation = "Undefined"
color           = "Singlet"
weak_multiplet  = "Triplet(-1)"
interactions    = ["Electromagnetic", "WeakCC"]

[[particle]]
id              = "Z0"
name            = "Z Boson"
symbol          = "Z^0"
mass            = 91.1876
width           = 2.4952
spin            = 2
electric_charge = 0
weak_isospin    = 0
hypercharge     = 0
baryon_number   = 0
lepton_numbers  = [0, 0, 0]
parity          = "Odd"
charge_conjugation = "Undefined"
color           = "Singlet"
weak_multiplet  = "Singlet"
interactions    = ["WeakNC"]

[[particle]]
id              = "g"
name            = "Gluon"
symbol          = "g"
mass            = 0.0
width           = 0.0
spin            = 2
electric_charge = 0
weak_isospin    = 0
hypercharge     = 0
baryon_number   = 0
lepton_numbers  = [0, 0, 0]
parity          = "Odd"
charge_conjugation = "Odd"
color           = "Octet"
weak_multiplet  = "Singlet"
interactions    = ["Strong"]

# ============================================================================
# SCALAR BOSONS
# ============================================================================

[[particle]]
id              = "H"
name            = "Higgs Boson"
symbol          = "H"
mass            = 125.1
width           = 0.00407
spin            = 0
electric_charge = 0
weak_isospin    = 0
hypercharge     = 0
baryon_number   = 0
lepton_numbers  = [0, 0, 0]
parity          = "Even"
charge_conjugation = "Even"
color           = "Singlet"
weak_multiplet  = "Singlet"
interactions    = ["Yukawa", "ScalarSelf"]
`;

// ---------------------------------------------------------------------------
// Standard Model Vertices (14 vertices)
// ---------------------------------------------------------------------------

export const DEFAULT_VERTICES_TOML = `# SPIRE Standard Model Vertex Definitions

# ============================================================================
# QED VERTICES - Electromagnetic coupling (U(1)_EM)
# ============================================================================

[[vertex]]
id                = "qed_eea"
description       = "QED vertex: electron-photon coupling"
field_ids         = ["e-", "photon", "e-"]
coupling_symbol   = "e"
coupling_value    = 0.30282212
lorentz_structure = "gamma_mu"
interaction_type  = "Electromagnetic"
term_kind         = "Interaction"
expression        = "-i e \\\\gamma^{\\\\mu}"
n_legs            = 3

[[vertex]]
id                = "qed_mma"
description       = "QED vertex: muon-photon coupling"
field_ids         = ["mu-", "photon", "mu-"]
coupling_symbol   = "e"
coupling_value    = 0.30282212
lorentz_structure = "gamma_mu"
interaction_type  = "Electromagnetic"
term_kind         = "Interaction"
expression        = "-i e \\\\gamma^{\\\\mu}"
n_legs            = 3

[[vertex]]
id                = "qed_tta"
description       = "QED vertex: tau-photon coupling"
field_ids         = ["tau-", "photon", "tau-"]
coupling_symbol   = "e"
coupling_value    = 0.30282212
lorentz_structure = "gamma_mu"
interaction_type  = "Electromagnetic"
term_kind         = "Interaction"
expression        = "-i e \\\\gamma^{\\\\mu}"
n_legs            = 3

[[vertex]]
id                = "qed_uua"
description       = "QED vertex: up quark-photon coupling"
field_ids         = ["u", "photon", "u"]
coupling_symbol   = "e_u"
coupling_value    = 0.20188141
lorentz_structure = "gamma_mu"
interaction_type  = "Electromagnetic"
term_kind         = "Interaction"
expression        = "-i (2/3) e \\\\gamma^{\\\\mu}"
n_legs            = 3

[[vertex]]
id                = "qed_dda"
description       = "QED vertex: down quark-photon coupling"
field_ids         = ["d", "photon", "d"]
coupling_symbol   = "e_d"
coupling_value    = 0.10094071
lorentz_structure = "gamma_mu"
interaction_type  = "Electromagnetic"
term_kind         = "Interaction"
expression        = "-i (-1/3) e \\\\gamma^{\\\\mu}"
n_legs            = 3

# ============================================================================
# WEAK CHARGED-CURRENT VERTICES (W± exchange)
# ============================================================================

[[vertex]]
id                = "cc_enw"
description       = "Weak CC vertex: electron-neutrino-W coupling"
field_ids         = ["e-", "nu_e", "W-"]
coupling_symbol   = "g_w"
coupling_value    = 0.6417
lorentz_structure = "gamma_mu"
interaction_type  = "WeakCC"
term_kind         = "Interaction"
expression        = "-i (g_w / \\\\sqrt{2}) \\\\gamma^{\\\\mu} P_L"
n_legs            = 3

[[vertex]]
id                = "cc_mnw"
description       = "Weak CC vertex: muon-neutrino-W coupling"
field_ids         = ["mu-", "nu_mu", "W-"]
coupling_symbol   = "g_w"
coupling_value    = 0.6417
lorentz_structure = "gamma_mu"
interaction_type  = "WeakCC"
term_kind         = "Interaction"
expression        = "-i (g_w / \\\\sqrt{2}) \\\\gamma^{\\\\mu} P_L"
n_legs            = 3

[[vertex]]
id                = "cc_udw"
description       = "Weak CC vertex: up-down quark-W coupling"
field_ids         = ["u", "d", "W+"]
coupling_symbol   = "g_w"
coupling_value    = 0.6417
lorentz_structure = "gamma_mu"
interaction_type  = "WeakCC"
term_kind         = "Interaction"
expression        = "-i (g_w / \\\\sqrt{2}) V_{ud} \\\\gamma^{\\\\mu} P_L"
n_legs            = 3

# ============================================================================
# WEAK NEUTRAL-CURRENT VERTICES (Z exchange)
# ============================================================================

[[vertex]]
id                = "nc_eez"
description       = "Weak NC vertex: electron-Z coupling"
field_ids         = ["e-", "e-", "Z0"]
coupling_symbol   = "g_z"
coupling_value    = 0.7397
lorentz_structure = "gamma_mu"
interaction_type  = "WeakNC"
term_kind         = "Interaction"
expression        = "-i (g_z) \\\\gamma^{\\\\mu} (g_V^e - g_A^e \\\\gamma^5)"
n_legs            = 3

[[vertex]]
id                = "nc_nnz"
description       = "Weak NC vertex: neutrino-Z coupling"
field_ids         = ["nu_e", "nu_e", "Z0"]
coupling_symbol   = "g_z"
coupling_value    = 0.7397
lorentz_structure = "gamma_mu"
interaction_type  = "WeakNC"
term_kind         = "Interaction"
expression        = "-i (g_z / 2) \\\\gamma^{\\\\mu} P_L"
n_legs            = 3

# ============================================================================
# QCD VERTICES - Strong coupling (SU(3)_C)
# ============================================================================

[[vertex]]
id                = "qcd_uug"
description       = "QCD vertex: up quark-gluon coupling"
field_ids         = ["u", "u", "g"]
coupling_symbol   = "g_s"
coupling_value    = 1.218
lorentz_structure = "gamma_mu"
interaction_type  = "Strong"
term_kind         = "Interaction"
expression        = "-i g_s T^a \\\\gamma^{\\\\mu}"
n_legs            = 3

[[vertex]]
id                = "qcd_ddg"
description       = "QCD vertex: down quark-gluon coupling"
field_ids         = ["d", "d", "g"]
coupling_symbol   = "g_s"
coupling_value    = 1.218
lorentz_structure = "gamma_mu"
interaction_type  = "Strong"
term_kind         = "Interaction"
expression        = "-i g_s T^a \\\\gamma^{\\\\mu}"
n_legs            = 3

# ============================================================================
# YUKAWA VERTICES - Higgs-fermion coupling
# ============================================================================

[[vertex]]
id                = "yuk_eeh"
description       = "Yukawa vertex: electron-Higgs coupling"
field_ids         = ["e-", "e-", "H"]
coupling_symbol   = "y_e"
coupling_value    = 0.00000294
lorentz_structure = "scalar"
interaction_type  = "Yukawa"
term_kind         = "Interaction"
expression        = "-i y_e / \\\\sqrt{2}"
n_legs            = 3
`;

// ---------------------------------------------------------------------------
// Commonly Used Particle ID Lists (for quick reaction input)
// ---------------------------------------------------------------------------

/** All available particle IDs in the SM definition. */
export const SM_PARTICLE_IDS: string[] = [
  "e-", "e+", "mu-", "mu+", "tau-", "tau+",
  "nu_e", "nu_e_bar", "nu_mu", "nu_mu_bar",
  "u", "u_bar", "d", "d_bar",
  "photon", "W+", "W-", "Z0", "g", "H",
];

/** Human-readable labels keyed by particle ID. */
export const PARTICLE_LABELS: Record<string, string> = {
  "e-":        "e⁻  Electron",
  "e+":        "e⁺  Positron",
  "mu-":       "μ⁻  Muon",
  "mu+":       "μ⁺  Antimuon",
  "tau-":      "τ⁻  Tau",
  "tau+":      "τ⁺  Antitau",
  "nu_e":      "νₑ  Electron Neutrino",
  "nu_e_bar":  "ν̄ₑ  Electron Antineutrino",
  "nu_mu":     "νμ  Muon Neutrino",
  "nu_mu_bar": "ν̄μ  Muon Antineutrino",
  "u":         "u   Up Quark",
  "u_bar":     "ū   Anti-Up Quark",
  "d":         "d   Down Quark",
  "d_bar":     "d̄   Anti-Down Quark",
  "photon":    "γ   Photon",
  "W+":        "W⁺  W Boson",
  "W-":        "W⁻  W Boson",
  "Z0":        "Z⁰  Z Boson",
  "g":         "g   Gluon",
  "H":         "H   Higgs Boson",
};

/** Default reaction: e⁻e⁺ → μ⁻μ⁺γγZ⁰ at √s = 500 GeV (multi-step richer starter topology). */
export const DEFAULT_INITIAL_IDS = ["e-", "e+"];
export const DEFAULT_FINAL_IDS = ["mu-", "mu+", "photon", "photon", "Z0"];
export const DEFAULT_CMS_ENERGY = 500.0;
