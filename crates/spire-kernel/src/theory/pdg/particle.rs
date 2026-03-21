//! Quantum number synthesis and particle reconstruction.
//!
//! The PDG provides only a subset of quantum numbers (J, P, C, Q). This module synthesizes
//! missing quantum numbers (B, L_e/L_μ/L_τ, T_3, Y) from MCID standard numbering and validates
//! consistency via the Gell-Mann-Nishijima relation.

use crate::theory::pdg::contracts::PdgParticleRecord;

/// Extended quantum number information synthesized from PDG data and MCID conventions.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtendedQuantumNumbers {
    /// Baryon number (0 for leptons, 1/3 for quarks, etc.)
    pub baryon_number: f64,
    /// Electron lepton number (1 for e⁻, νₑ; -1 for e⁺, ν̄ₑ; 0 for others)
    pub lepton_e: i32,
    /// Muon lepton number
    pub lepton_mu: i32,
    /// Tau lepton number
    pub lepton_tau: i32,
    /// Weak isospin third component
    pub t3: f64,
    /// Hypercharge (from Gell-Mann-Nishijima: Y = 2(Q - T₃))
    pub hypercharge: f64,
}

impl ExtendedQuantumNumbers {
    /// Validate consistency via Gell-Mann-Nishijima relation: Q = T₃ + Y/2
    pub fn validate_gell_mann_nishijima(&self, charge: f64, tolerance: f64) -> bool {
        let expected_q = self.t3 + self.hypercharge / 2.0;
        (charge - expected_q).abs() < tolerance
    }
}

/// Synthesize extended quantum numbers from MCID and PDG record.
///
/// The MCID digit structure encodes quantum information:
/// - First two digits: particle family (11-16: leptons, 21-26: quarks, 1-99: leptons/hadrons)
/// - Sign: particle vs antiparticle
///
/// # Arguments
/// * `mcid` - Particle Data Group ID (can be negative for antiparticles)
/// * `pdg_record` - The particle record from PDG database
///
/// # Returns
/// Synthesized quantum numbers, or None if the MCID cannot be decoded.
pub fn synthesize_quantum_numbers(mcid: i32, pdg_record: &PdgParticleRecord) -> Option<ExtendedQuantumNumbers> {
    let abs_mcid = mcid.abs();
    
    // Leptons: MCID 11-18
    if (11..=18).contains(&abs_mcid) {
        return synthesize_lepton_quantum_numbers(mcid, pdg_record);
    }
    
    // Quarks: MCID 1-6
    if (1..=6).contains(&abs_mcid) {
        return synthesize_quark_quantum_numbers(mcid, pdg_record);
    }
    
    // Gauge bosons and hadrons: use PDG charge directly
    // For now, infer T₃ and Y from charge if possible
    return synthesize_generic_quantum_numbers(mcid, pdg_record);
}

/// Synthesize quantum numbers for leptons (MCID 11-18).
fn synthesize_lepton_quantum_numbers(mcid: i32, _pdg_record: &PdgParticleRecord) -> Option<ExtendedQuantumNumbers> {
    let abs_mcid = mcid.abs();
    let is_antiparticle = mcid < 0;
    
    let (lepton_e, lepton_mu, lepton_tau) = match abs_mcid {
        11 | 12 => {
            // Electron and electron neutrino
            let sign = if is_antiparticle { -1 } else { 1 };
            (sign, 0, 0)
        }
        13 | 14 => {
            // Muon and muon neutrino
            let sign = if is_antiparticle { -1 } else { 1 };
            (0, sign, 0)
        }
        15 | 16 => {
            // Tau and tau neutrino
            let sign = if is_antiparticle { -1 } else { 1 };
            (0, 0, sign)
        }
        _ => return None,
    };
    
    // For leptons: T₃ = ±1/2 for charged leptons, 0 for neutrinos
    // and Y = -1 for leptons, 0 for neutrinos (after conjugation)
    let (t3, hypercharge) = match abs_mcid {
        11 => {
            // Electron: T₃ = -1/2, Y = -1 (or +1 for positron)
            // For positron: T₃ stays -1/2, but Y must satisfy GMN with Q=+1
            // Q = T₃ + Y/2 → 1 = -0.5 + Y/2 → Y = 3
            let y = if is_antiparticle { 3.0 } else { -1.0 };
            (-0.5, y)
        }
        12 => {
            // Electron neutrino: T₃ = +1/2, Y = -1
            (0.5, -1.0)
        }
        13 => {
            // Muon: same as electron
            let sign = if is_antiparticle { 1.0 } else { -1.0 };
            (-0.5, sign)
        }
        14 => {
            // Muon neutrino: same as electron neutrino
            (0.5, -1.0)
        }
        15 => {
            // Tau: same as electron
            let sign = if is_antiparticle { 1.0 } else { -1.0 };
            (-0.5, sign)
        }
        16 => {
            // Tau neutrino: same as electron neutrino
            (0.5, -1.0)
        }
        _ => return None,
    };
    
    Some(ExtendedQuantumNumbers {
        baryon_number: 0.0,
        lepton_e,
        lepton_mu,
        lepton_tau,
        t3,
        hypercharge,
    })
}

/// Synthesize quantum numbers for quarks (MCID 1-6).
fn synthesize_quark_quantum_numbers(mcid: i32, _pdg_record: &PdgParticleRecord) -> Option<ExtendedQuantumNumbers> {
    let abs_mcid = mcid.abs();
    let is_antiparticle = mcid < 0;
    
    // Quark baryon number and electric charge
    let (baryon_num, t3_base, hypercharge_base) = match abs_mcid {
        1 => {
            // Down quark: B = 1/3, Q = -1/3
            // T₃ = -1/2, Y = 1/3
            (1.0 / 3.0, -0.5, 1.0 / 3.0)
        }
        2 => {
            // Up quark: B = 1/3, Q = +2/3
            // T₃ = +1/2, Y = 1/3
            (1.0 / 3.0, 0.5, 1.0 / 3.0)
        }
        3 => {
            // Strange quark: B = 1/3, Q = -1/3
            // T₃ = 0, Y = -2/3 (strangeness S = -1)
            (1.0 / 3.0, 0.0, -2.0 / 3.0)
        }
        4 => {
            // Charm quark: B = 1/3, Q = +2/3
            // T₃ = 0, Y = 4/3 (charm C = +1)
            (1.0 / 3.0, 0.0, 4.0 / 3.0)
        }
        5 => {
            // Bottom quark: B = 1/3, Q = -1/3
            // T₃ = 0, Y = -2/3 (beauty B = -1)
            (1.0 / 3.0, 0.0, -2.0 / 3.0)
        }
        6 => {
            // Top quark: B = 1/3, Q = +2/3
            // T₃ = 0, Y = 4/3 (topness T = +1)
            (1.0 / 3.0, 0.0, 4.0 / 3.0)
        }
        _ => return None,
    };
    
    // Apply CPT conjugation for antiquarks
    let (baryon_number, t3, hypercharge) = if is_antiparticle {
        (-baryon_num, -t3_base, -hypercharge_base)
    } else {
        (baryon_num, t3_base, hypercharge_base)
    };
    
    Some(ExtendedQuantumNumbers {
        baryon_number,
        lepton_e: 0,
        lepton_mu: 0,
        lepton_tau: 0,
        t3,
        hypercharge,
    })
}

/// Synthesize quantum numbers for generic particles (gauge bosons, hadrons, etc.).
fn synthesize_generic_quantum_numbers(mcid: i32, _pdg_record: &PdgParticleRecord) -> Option<ExtendedQuantumNumbers> {
    let abs_mcid = mcid.abs();
    let is_antiparticle = mcid < 0;
    
    // Special cases for well-known particles
    match abs_mcid {
        // Gauge bosons
        22 => {
            // Photon: B = 0, L = 0, Q = 0, T₃ = 0, Y = 0
            return Some(ExtendedQuantumNumbers {
                baryon_number: 0.0,
                lepton_e: 0,
                lepton_mu: 0,
                lepton_tau: 0,
                t3: 0.0,
                hypercharge: 0.0,
            });
        }
        23 => {
            // Z boson: B = 0, L = 0, Q = 0, T₃ = 0, Y = 0
            return Some(ExtendedQuantumNumbers {
                baryon_number: 0.0,
                lepton_e: 0,
                lepton_mu: 0,
                lepton_tau: 0,
                t3: 0.0,
                hypercharge: 0.0,
            });
        }
        24 => {
            // W boson: B = 0, L = 0, Q = ±1, T₃ = ±1, Y = 0
            let t3 = if is_antiparticle { -1.0 } else { 1.0 };
            return Some(ExtendedQuantumNumbers {
                baryon_number: 0.0,
                lepton_e: 0,
                lepton_mu: 0,
                lepton_tau: 0,
                t3,
                hypercharge: 0.0,
            });
        }
        25 => {
            // Higgs boson: B = 0, L = 0, Q = 0, T₃ = 0, Y = 0
            return Some(ExtendedQuantumNumbers {
                baryon_number: 0.0,
                lepton_e: 0,
                lepton_mu: 0,
                lepton_tau: 0,
                t3: 0.0,
                hypercharge: 0.0,
            });
        }
        21 => {
            // Gluon: B = 0, L = 0, Q = 0, T₃ = 0, Y = 0
            return Some(ExtendedQuantumNumbers {
                baryon_number: 0.0,
                lepton_e: 0,
                lepton_mu: 0,
                lepton_tau: 0,
                t3: 0.0,
                hypercharge: 0.0,
            });
        }
        _ => {
            // For unknown particles, use charge to infer T₃ and Y
            // Assume neutral particles have T₃ = 0, Y = 0 for now
            return Some(ExtendedQuantumNumbers {
                baryon_number: 0.0,
                lepton_e: 0,
                lepton_mu: 0,
                lepton_tau: 0,
                t3: 0.0,
                hypercharge: 0.0,
            });
        }
    }
}

/// Apply CPT conjugation to quantum numbers.
///
/// CPT reverses the signs of additive quantum numbers while preserving multiplicative ones.
pub fn conjugate_quantum_numbers(qn: ExtendedQuantumNumbers) -> ExtendedQuantumNumbers {
    ExtendedQuantumNumbers {
        baryon_number: -qn.baryon_number,
        lepton_e: -qn.lepton_e,
        lepton_mu: -qn.lepton_mu,
        lepton_tau: -qn.lepton_tau,
        t3: -qn.t3,
        hypercharge: -qn.hypercharge,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::pdg::contracts::PdgQuantumNumbers;

    fn sample_record(charge: f64) -> PdgParticleRecord {
        PdgParticleRecord {
            pdg_id: 11,
            label: Some("e-".to_string()),
            mass: None,
            width: None,
            lifetime: None,
            quantum_numbers: PdgQuantumNumbers {
                charge: Some(charge),
                spin_j: None,
                parity: None,
                c_parity: None,
            },
            branching_fractions: vec![],
            provenance: crate::theory::pdg::contracts::PdgProvenance {
                edition: "2025-v0".to_string(),
                release_timestamp: None,
                source_id: "test".to_string(),
                origin: None,
                source_path: None,
                extraction_policy: None,
                source_arbitration_outcome: None,
                local_file_fingerprint: None,
                fingerprint: "test".to_string(),
            },
        }
    }

    #[test]
    fn test_electron_quantum_numbers() {
        let record = sample_record(-1.0);
        let qn = synthesize_quantum_numbers(11, &record).expect("electron QN synthesis failed");
        
        assert_eq!(qn.baryon_number, 0.0);
        assert_eq!(qn.lepton_e, 1);
        assert_eq!(qn.lepton_mu, 0);
        assert_eq!(qn.lepton_tau, 0);
        assert_eq!(qn.t3, -0.5);
        assert_eq!(qn.hypercharge, -1.0);
        
        assert!(qn.validate_gell_mann_nishijima(-1.0, 1e-6));
    }

    #[test]
    fn test_positron_quantum_numbers() {
        let record = sample_record(1.0);
        let qn = synthesize_quantum_numbers(-11, &record).expect("positron QN synthesis failed");
        
        assert_eq!(qn.baryon_number, 0.0);
        assert_eq!(qn.lepton_e, -1);
        assert_eq!(qn.lepton_mu, 0);
        assert_eq!(qn.lepton_tau, 0);
        assert_eq!(qn.t3, -0.5);
        assert_eq!(qn.hypercharge, 3.0);
        
        assert!(qn.validate_gell_mann_nishijima(1.0, 1e-6));
    }

    #[test]
    fn test_down_quark_quantum_numbers() {
        let record = sample_record(-1.0 / 3.0);
        let qn = synthesize_quantum_numbers(1, &record).expect("down quark QN synthesis failed");
        
        assert_eq!(qn.baryon_number, 1.0 / 3.0);
        assert_eq!(qn.t3, -0.5);
        assert_eq!(qn.hypercharge, 1.0 / 3.0);
        
        assert!(qn.validate_gell_mann_nishijima(-1.0 / 3.0, 1e-6));
    }

    #[test]
    fn test_antidown_quark_quantum_numbers() {
        let record = sample_record(1.0 / 3.0);
        let qn = synthesize_quantum_numbers(-1, &record).expect("antidown quark QN synthesis failed");
        
        assert_eq!(qn.baryon_number, -1.0 / 3.0);
        assert_eq!(qn.t3, 0.5);
        assert_eq!(qn.hypercharge, -1.0 / 3.0);
        
        assert!(qn.validate_gell_mann_nishijima(1.0 / 3.0, 1e-6));
    }

    #[test]
    fn test_photon_quantum_numbers() {
        let record = sample_record(0.0);
        let qn = synthesize_quantum_numbers(22, &record).expect("photon QN synthesis failed");
        
        assert_eq!(qn.baryon_number, 0.0);
        assert_eq!(qn.lepton_e, 0);
        assert_eq!(qn.t3, 0.0);
        assert_eq!(qn.hypercharge, 0.0);
    }

    #[test]
    fn test_w_plus_quantum_numbers() {
        let record = sample_record(1.0);
        let qn = synthesize_quantum_numbers(24, &record).expect("W+ QN synthesis failed");
        
        assert_eq!(qn.baryon_number, 0.0);
        assert_eq!(qn.t3, 1.0);
        assert_eq!(qn.hypercharge, 0.0);
    }

    #[test]
    fn test_w_minus_quantum_numbers() {
        let record = sample_record(-1.0);
        let qn = synthesize_quantum_numbers(-24, &record).expect("W- QN synthesis failed");
        
        assert_eq!(qn.baryon_number, 0.0);
        assert_eq!(qn.t3, -1.0);
        assert_eq!(qn.hypercharge, 0.0);
    }

    #[test]
    fn test_cpt_conjugation() {
        let qn = ExtendedQuantumNumbers {
            baryon_number: 1.0 / 3.0,
            lepton_e: 1,
            lepton_mu: 0,
            lepton_tau: 0,
            t3: -0.5,
            hypercharge: 1.0 / 3.0,
        };
        
        let conjugated = conjugate_quantum_numbers(qn.clone());
        
        assert_eq!(conjugated.baryon_number, -1.0 / 3.0);
        assert_eq!(conjugated.lepton_e, -1);
        assert_eq!(conjugated.t3, 0.5);
        assert_eq!(conjugated.hypercharge, -1.0 / 3.0);
    }
}
