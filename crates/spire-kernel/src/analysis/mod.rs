//! # Analysis : Integrated Histogramming & Observable Pipeline
//!
//! This module provides high-performance histogramming structures and an
//! analysis pipeline that connects the Monte Carlo integration engine
//! with the Rhai scripting engine for real-time
//! statistical visualization of kinematic distributions.
//!
//! ## Architecture
//!
//! The analysis pipeline follows a three-stage design:
//!
//! 1. **Definition**: Users specify plots via [`PlotDefinition`], each
//!    containing a Rhai observable script and histogram binning parameters.
//! 2. **Accumulation**: During the Monte Carlo integration loop, each event
//!    that passes kinematic cuts is evaluated against all observable scripts.
//!    The resulting values are filled into [`Histogram1D`] accumulators.
//! 3. **Serialization**: Completed histograms are converted to
//!    [`HistogramData`] DTOs for transmission to the frontend.
//!
//! ## Concurrency Strategy
//!
//! For parallel integration, each thread maintains a **thread-local**
//! histogram set. After the parallel loop completes, all thread-local
//! histograms are merged via [`Histogram1D::merge`]. This avoids atomic
//! operations in the hot loop and incurs only an $O(N_\text{bins})$ merge
//! cost at the end of the run.
//!
//! ## Performance
//!
//! The [`Histogram1D::fill`] method is optimised for the hot loop:
//! - Precomputed inverse bin width replaces division with multiplication.
//! - No heap allocations per event.
//! - No string operations in the filling path.
//! - Branch-free bin index computation for in-range values.

pub mod likelihood;

use serde::{Deserialize, Serialize};

use crate::kinematics::{PhaseSpaceGenerator, PhaseSpacePoint};
use crate::reco::detector::{DetectorModel, ParticleKind};
use crate::scripting::{Observable, RhaiObservable, RhaiRecoObservable, SpireScriptEngine};
use crate::{SpireError, SpireResult};

// ===========================================================================
// Histogram1D
// ===========================================================================

/// A one-dimensional histogram with fixed-width bins.
///
/// Designed for high-throughput filling in Monte Carlo integration loops.
/// Supports weighted entries, under/overflow tracking, and bin-level
/// variance estimation via sum-of-weights-squared accumulation.
///
/// # Bin Layout
///
/// For $N$ bins over the interval $[x_\min, x_\max)$:
///
/// $$\text{bin width} = \frac{x_\max - x_\min}{N}$$
///
/// Bin $i$ covers $[x_\min + i \cdot w, \, x_\min + (i+1) \cdot w)$
/// for $i \in \{0, 1, \ldots, N-1\}$.
///
/// Values below $x_\min$ go to underflow; values $\geq x_\max$ go to overflow.
#[derive(Debug, Clone)]
pub struct Histogram1D {
    /// Weighted counts per bin.
    bins: Vec<f64>,
    /// Sum of weightsÃ‚Â² per bin (for variance estimation).
    bin_sq: Vec<f64>,
    /// Lower edge of the histogram range.
    min: f64,
    /// Upper edge of the histogram range.
    max: f64,
    /// Number of bins.
    n_bins: usize,
    /// Precomputed bin width: $(x_\max - x_\min) / N$.
    bin_width: f64,
    /// Precomputed inverse bin width: $N / (x_\max - x_\min)$.
    /// Replaces division with multiplication in the hot loop.
    inv_bin_width: f64,
    /// Accumulated weight below the lower edge.
    underflow: f64,
    /// Accumulated weight above the upper edge.
    overflow: f64,
    /// Total accumulated weight across all bins (including under/overflow).
    total_weight: f64,
    /// Total number of fill calls.
    entries: u64,
}

impl Histogram1D {
    /// Create a new histogram with the specified binning.
    ///
    /// # Arguments
    ///
    /// * `n_bins` : Number of equally-spaced bins.
    /// * `min` : Lower edge of the first bin.
    /// * `max` : Upper edge of the last bin.
    ///
    /// # Panics
    ///
    /// Panics if `n_bins == 0` or `min >= max`.
    pub fn new(n_bins: usize, min: f64, max: f64) -> Self {
        assert!(n_bins > 0, "Histogram must have at least one bin");
        assert!(
            min < max,
            "Histogram min ({}) must be less than max ({})",
            min,
            max
        );

        let bin_width = (max - min) / n_bins as f64;
        let inv_bin_width = 1.0 / bin_width;

        Self {
            bins: vec![0.0; n_bins],
            bin_sq: vec![0.0; n_bins],
            min,
            max,
            n_bins,
            bin_width,
            inv_bin_width,
            underflow: 0.0,
            overflow: 0.0,
            total_weight: 0.0,
            entries: 0,
        }
    }

    /// Fill the histogram with a single value and weight.
    ///
    /// This is the hot-loop method: no allocations, no string operations,
    /// and the bin lookup uses precomputed `inv_bin_width` to avoid division.
    ///
    /// # Arguments
    ///
    /// * `value` : The observable value to bin.
    /// * `weight` : The event weight (typically the phase-space weight
    ///   times the squared matrix element).
    #[inline]
    pub fn fill(&mut self, value: f64, weight: f64) {
        self.entries += 1;
        self.total_weight += weight;

        if value < self.min {
            self.underflow += weight;
        } else if value >= self.max {
            self.overflow += weight;
        } else {
            // Compute bin index via multiplication (no division in hot path).
            let idx = ((value - self.min) * self.inv_bin_width) as usize;
            // Guard against floating-point edge case where value Ã¢â€°Ë† max.
            let idx = idx.min(self.n_bins - 1);
            self.bins[idx] += weight;
            self.bin_sq[idx] += weight * weight;
        }
    }

    /// Merge another histogram into this one (parallel reduction step).
    ///
    /// Both histograms must have identical binning (same `n_bins`, `min`, `max`).
    /// This is an $O(N_\text{bins})$ operation called once after the parallel
    /// integration loop completes.
    ///
    /// # Errors
    ///
    /// Returns an error if the binning parameters do not match.
    pub fn merge(&mut self, other: &Histogram1D) -> SpireResult<()> {
        if self.n_bins != other.n_bins
            || (self.min - other.min).abs() > 1e-12
            || (self.max - other.max).abs() > 1e-12
        {
            return Err(SpireError::InternalError(
                "Cannot merge histograms with different binning".into(),
            ));
        }

        for i in 0..self.n_bins {
            self.bins[i] += other.bins[i];
            self.bin_sq[i] += other.bin_sq[i];
        }
        self.underflow += other.underflow;
        self.overflow += other.overflow;
        self.total_weight += other.total_weight;
        self.entries += other.entries;

        Ok(())
    }

    /// Reset all bin contents to zero.
    pub fn reset(&mut self) {
        self.bins.iter_mut().for_each(|b| *b = 0.0);
        self.bin_sq.iter_mut().for_each(|b| *b = 0.0);
        self.underflow = 0.0;
        self.overflow = 0.0;
        self.total_weight = 0.0;
        self.entries = 0;
    }

    /// Number of bins.
    pub fn n_bins(&self) -> usize {
        self.n_bins
    }

    /// Lower edge of the histogram range.
    pub fn min(&self) -> f64 {
        self.min
    }

    /// Upper edge of the histogram range.
    pub fn max(&self) -> f64 {
        self.max
    }

    /// Bin width.
    pub fn bin_width(&self) -> f64 {
        self.bin_width
    }

    /// Underflow count.
    pub fn underflow(&self) -> f64 {
        self.underflow
    }

    /// Overflow count.
    pub fn overflow(&self) -> f64 {
        self.overflow
    }

    /// Total accumulated weight.
    pub fn total_weight(&self) -> f64 {
        self.total_weight
    }

    /// Total number of fill operations.
    pub fn entries(&self) -> u64 {
        self.entries
    }

    /// Read-only access to the bin contents (weighted counts).
    pub fn bin_contents(&self) -> &[f64] {
        &self.bins
    }

    /// Read-only access to the sum-of-weights-squared per bin.
    pub fn bin_sum_w2(&self) -> &[f64] {
        &self.bin_sq
    }

    /// Compute the statistical error per bin: $\sqrt{\sum w_i^2}$.
    pub fn bin_errors(&self) -> Vec<f64> {
        self.bin_sq.iter().map(|s| s.sqrt()).collect()
    }

    /// Return the centre of each bin.
    pub fn bin_centres(&self) -> Vec<f64> {
        (0..self.n_bins)
            .map(|i| self.min + (i as f64 + 0.5) * self.bin_width)
            .collect()
    }

    /// Return the lower edge of each bin.
    pub fn bin_edges(&self) -> Vec<f64> {
        (0..=self.n_bins)
            .map(|i| self.min + i as f64 * self.bin_width)
            .collect()
    }

    /// Find the bin index with the maximum content.
    pub fn max_bin(&self) -> usize {
        self.bins
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Mean of the distribution: $\bar{x} = \sum_i x_i w_i / \sum_i w_i$.
    pub fn mean(&self) -> f64 {
        let in_range_weight = self.total_weight - self.underflow - self.overflow;
        if in_range_weight.abs() < 1e-300 {
            return 0.0;
        }
        let sum_xw: f64 = self
            .bins
            .iter()
            .enumerate()
            .map(|(i, &w)| {
                let x = self.min + (i as f64 + 0.5) * self.bin_width;
                x * w
            })
            .sum();
        sum_xw / in_range_weight
    }

    /// Convert to a serializable DTO for frontend transmission.
    pub fn to_data(&self, name: &str) -> HistogramData {
        HistogramData {
            name: name.to_string(),
            bin_edges: self.bin_edges(),
            bin_contents: self.bins.clone(),
            bin_errors: self.bin_errors(),
            underflow: self.underflow,
            overflow: self.overflow,
            entries: self.entries,
            mean: self.mean(),
        }
    }
}

// ===========================================================================
// Histogram2D
// ===========================================================================

/// A two-dimensional histogram with fixed-width bins on both axes.
///
/// Useful for correlation plots (e.g., $p_T$ vs $\eta$, invariant mass
/// vs rapidity). The bin layout is row-major: bin `(ix, iy)` is stored
/// at index `iy * nx + ix`.
#[derive(Debug, Clone)]
pub struct Histogram2D {
    /// Weighted counts per bin (row-major: bins[iy * nx + ix]).
    bins: Vec<f64>,
    /// X-axis parameters.
    x_min: f64,
    x_max: f64,
    nx: usize,
    _x_bin_width: f64,
    inv_x_bin_width: f64,
    /// Y-axis parameters.
    y_min: f64,
    y_max: f64,
    ny: usize,
    _y_bin_width: f64,
    inv_y_bin_width: f64,
    /// Total accumulated weight.
    total_weight: f64,
    /// Total number of fill operations.
    entries: u64,
}

impl Histogram2D {
    /// Create a new 2D histogram.
    ///
    /// # Panics
    ///
    /// Panics if either axis has zero bins or `min >= max`.
    pub fn new(nx: usize, x_min: f64, x_max: f64, ny: usize, y_min: f64, y_max: f64) -> Self {
        assert!(
            nx > 0 && ny > 0,
            "2D histogram must have at least one bin per axis"
        );
        assert!(x_min < x_max, "X-axis min must be less than max");
        assert!(y_min < y_max, "Y-axis min must be less than max");

        let x_bin_width = (x_max - x_min) / nx as f64;
        let y_bin_width = (y_max - y_min) / ny as f64;

        Self {
            bins: vec![0.0; nx * ny],
            x_min,
            x_max,
            nx,
            _x_bin_width: x_bin_width,
            inv_x_bin_width: 1.0 / x_bin_width,
            y_min,
            y_max,
            ny,
            _y_bin_width: y_bin_width,
            inv_y_bin_width: 1.0 / y_bin_width,
            total_weight: 0.0,
            entries: 0,
        }
    }

    /// Fill the 2D histogram. Values outside either axis range are ignored.
    #[inline]
    pub fn fill(&mut self, x: f64, y: f64, weight: f64) {
        self.entries += 1;
        self.total_weight += weight;

        if x < self.x_min || x >= self.x_max || y < self.y_min || y >= self.y_max {
            return;
        }

        let ix = ((x - self.x_min) * self.inv_x_bin_width) as usize;
        let iy = ((y - self.y_min) * self.inv_y_bin_width) as usize;
        let ix = ix.min(self.nx - 1);
        let iy = iy.min(self.ny - 1);

        self.bins[iy * self.nx + ix] += weight;
    }

    /// Merge another 2D histogram (parallel reduction).
    pub fn merge(&mut self, other: &Histogram2D) -> SpireResult<()> {
        if self.nx != other.nx || self.ny != other.ny {
            return Err(SpireError::InternalError(
                "Cannot merge 2D histograms with different binning".into(),
            ));
        }
        for i in 0..self.bins.len() {
            self.bins[i] += other.bins[i];
        }
        self.total_weight += other.total_weight;
        self.entries += other.entries;
        Ok(())
    }

    /// Read-only access to the bin contents.
    pub fn bin_contents(&self) -> &[f64] {
        &self.bins
    }

    /// Number of bins on the X axis.
    pub fn nx(&self) -> usize {
        self.nx
    }

    /// Number of bins on the Y axis.
    pub fn ny(&self) -> usize {
        self.ny
    }

    /// Total entries.
    pub fn entries(&self) -> u64 {
        self.entries
    }

    /// X-axis bin edges (length = nx + 1).
    pub fn x_bin_edges(&self) -> Vec<f64> {
        let x_bw = (self.x_max - self.x_min) / self.nx as f64;
        (0..=self.nx)
            .map(|i| self.x_min + i as f64 * x_bw)
            .collect()
    }

    /// Y-axis bin edges (length = ny + 1).
    pub fn y_bin_edges(&self) -> Vec<f64> {
        let y_bw = (self.y_max - self.y_min) / self.ny as f64;
        (0..=self.ny)
            .map(|i| self.y_min + i as f64 * y_bw)
            .collect()
    }

    /// Convert to a serializable DTO for frontend transmission.
    pub fn to_data_2d(&self, name: &str) -> Histogram2DData {
        Histogram2DData {
            name: name.to_string(),
            x_bin_edges: self.x_bin_edges(),
            y_bin_edges: self.y_bin_edges(),
            bin_contents: self.bins.clone(),
            nx: self.nx,
            ny: self.ny,
            entries: self.entries,
            total_weight: self.total_weight,
        }
    }
}

// ===========================================================================
// Serializable DTOs
// ===========================================================================

/// Serializable histogram data for frontend transmission.
///
/// Contains all information needed to render a histogram bar chart:
/// bin edges (N+1 values), bin contents (N values), and statistical errors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramData {
    /// Human-readable name of this histogram.
    pub name: String,
    /// Bin edges (length = n_bins + 1).
    pub bin_edges: Vec<f64>,
    /// Bin contents (weighted counts, length = n_bins).
    pub bin_contents: Vec<f64>,
    /// Statistical errors per bin: $\sqrt{\sum w_i^2}$.
    pub bin_errors: Vec<f64>,
    /// Underflow count.
    pub underflow: f64,
    /// Overflow count.
    pub overflow: f64,
    /// Total number of entries.
    pub entries: u64,
    /// Distribution mean.
    pub mean: f64,
}

/// Serializable 2D histogram data for frontend heatmap rendering.
///
/// Contains all information needed to render a 2D heatmap:
/// bin edges on both axes and a flat row-major array of bin contents.
///
/// # Layout
///
/// The `bin_contents` array is stored in row-major order:
/// `bin_contents[iy * nx + ix]` gives the content of the bin at
/// x-index `ix`, y-index `iy`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Histogram2DData {
    /// Human-readable name of this 2D histogram.
    pub name: String,
    /// X-axis bin edges (length = nx + 1).
    pub x_bin_edges: Vec<f64>,
    /// Y-axis bin edges (length = ny + 1).
    pub y_bin_edges: Vec<f64>,
    /// Bin contents in row-major order (length = nx * ny).
    pub bin_contents: Vec<f64>,
    /// Number of bins on the X axis.
    pub nx: usize,
    /// Number of bins on the Y axis.
    pub ny: usize,
    /// Total number of fill operations.
    pub entries: u64,
    /// Total accumulated weight.
    pub total_weight: f64,
}

// ===========================================================================
// Event Display DTO
// ===========================================================================

/// Serializable 3D vector for frontend rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vec3 {
    /// Cartesian x-component.
    pub x: f64,
    /// Cartesian y-component.
    pub y: f64,
    /// Cartesian z-component.
    pub z: f64,
}

/// Serializable jet representation for 3D event display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayJet {
    /// Jet momentum direction.
    pub direction: Vec3,
    /// Jet energy (GeV) â€” used to scale the cone size.
    pub energy: f64,
    /// Jet transverse momentum (GeV).
    pub pt: f64,
    /// Pseudorapidity.
    pub eta: f64,
    /// Azimuthal angle.
    pub phi: f64,
    /// Number of constituents.
    pub n_constituents: usize,
}

/// Serializable track for 3D event display (leptons, photons).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayTrack {
    /// Momentum direction (unit-normalised for rendering).
    pub direction: Vec3,
    /// Track energy (GeV).
    pub energy: f64,
    /// Transverse momentum (GeV).
    pub pt: f64,
    /// Pseudorapidity.
    pub eta: f64,
    /// Particle type label.
    pub particle_type: String,
}

/// Serializable missing transverse energy for 3D event display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayMET {
    /// MET direction in the transverse plane.
    pub direction: Vec3,
    /// MET magnitude (GeV).
    pub magnitude: f64,
}

/// Complete event display data for the 3D visualiser.
///
/// Contains all physics objects needed to render an interactive
/// event display: jets as cones, lepton/photon tracks as lines,
/// and missing transverse energy as a dashed arrow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDisplayData {
    /// Reconstructed jets (rendered as cones).
    pub jets: Vec<DisplayJet>,
    /// Electron tracks (rendered as green lines).
    pub electrons: Vec<DisplayTrack>,
    /// Muon tracks (rendered as red lines).
    pub muons: Vec<DisplayTrack>,
    /// Photon tracks (rendered as yellow dashed lines).
    pub photons: Vec<DisplayTrack>,
    /// Missing transverse energy (rendered as a dashed arrow).
    pub met: DisplayMET,
    /// Centre-of-mass energy used for this event (GeV).
    pub cms_energy: f64,
}

/// Generate a single display event by running RAMBO + detector simulation.
///
/// This produces a serializable `EventDisplayData` suitable for 3D rendering.
///
/// # Arguments
///
/// * `cms_energy` â€” Centre-of-mass energy (GeV).
/// * `final_masses` â€” Final-state particle masses.
/// * `detector_preset` â€” Detector preset name (e.g., `"lhc_like"`).
/// * `particle_kinds_str` â€” Optional particle kind labels per final-state leg.
pub fn generate_display_event(
    cms_energy: f64,
    final_masses: &[f64],
    detector_preset: &str,
    particle_kinds_str: Option<&[String]>,
) -> SpireResult<EventDisplayData> {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let detector = DetectorModel::from_preset(detector_preset).ok_or_else(|| {
        SpireError::InternalError(format!("unknown detector preset '{detector_preset}'"))
    })?;

    let n_final = final_masses.len();
    let particle_kinds: Vec<ParticleKind> = match particle_kinds_str {
        Some(kinds) => {
            if kinds.len() != n_final {
                return Err(SpireError::InternalError(format!(
                    "particle_kinds length ({}) must match final_masses ({n_final})",
                    kinds.len()
                )));
            }
            kinds
                .iter()
                .map(|s| parse_particle_kind(s))
                .collect::<SpireResult<Vec<_>>>()?
        }
        None => vec![ParticleKind::Hadron; n_final],
    };

    // Generate one event.
    use crate::kinematics::RamboGenerator;
    let mut gen = RamboGenerator::new();
    let event = gen.generate_event(cms_energy, final_masses)?;

    // Use a random seed that changes each call for variety.
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(42);
    let mut rng = StdRng::seed_from_u64(seed);

    let reco =
        crate::reco::detector::reconstruct_event(&event, &particle_kinds, &detector, &mut rng);

    // Convert to display format.
    let jets: Vec<DisplayJet> = reco
        .jets
        .iter()
        .map(|jet| {
            let px = jet.momentum[1];
            let py = jet.momentum[2];
            let pz = jet.momentum[3];
            let p_mag = (px * px + py * py + pz * pz).sqrt().max(1e-30);
            DisplayJet {
                direction: Vec3 {
                    x: px / p_mag,
                    y: py / p_mag,
                    z: pz / p_mag,
                },
                energy: jet.energy(),
                pt: jet.pt(),
                eta: jet.eta(),
                phi: jet.phi(),
                n_constituents: jet.n_constituents(),
            }
        })
        .collect();

    let make_track = |v: &crate::algebra::SpacetimeVector, ptype: &str| -> DisplayTrack {
        let px = v[1];
        let py = v[2];
        let pz = v[3];
        let p_mag = (px * px + py * py + pz * pz).sqrt().max(1e-30);
        let pt = (px * px + py * py).sqrt();
        let eta = if p_mag > 1e-30 {
            (pz / p_mag).atanh()
        } else {
            0.0
        };
        DisplayTrack {
            direction: Vec3 {
                x: px / p_mag,
                y: py / p_mag,
                z: pz / p_mag,
            },
            energy: v[0],
            pt,
            eta,
            particle_type: ptype.to_string(),
        }
    };

    let electrons: Vec<DisplayTrack> = reco
        .electrons
        .iter()
        .map(|v| make_track(v, "electron"))
        .collect();
    let muons: Vec<DisplayTrack> = reco.muons.iter().map(|v| make_track(v, "muon")).collect();
    let photons: Vec<DisplayTrack> = reco
        .photons
        .iter()
        .map(|v| make_track(v, "photon"))
        .collect();

    let met_px = reco.met[1];
    let met_py = reco.met[2];
    let met_mag = (met_px * met_px + met_py * met_py).sqrt();
    let met = DisplayMET {
        direction: Vec3 {
            x: if met_mag > 1e-30 {
                met_px / met_mag
            } else {
                0.0
            },
            y: if met_mag > 1e-30 {
                met_py / met_mag
            } else {
                0.0
            },
            z: 0.0,
        },
        magnitude: met_mag,
    };

    Ok(EventDisplayData {
        jets,
        electrons,
        muons,
        photons,
        met,
        cms_energy,
    })
}

/// Generate a batch of event display data for animation playback.
///
/// Produces `batch_size` independent Monte-Carlo events, each processed
/// through the detector simulation, and returns them as a vector of
/// [`EventDisplayData`] suitable for sequential playback in the 3D viewer.
///
/// # Arguments
///
/// * `cms_energy` â€” Centre-of-mass energy (GeV).
/// * `final_masses` â€” Final-state particle masses.
/// * `detector_preset` â€” Detector preset name (e.g., `"lhc_like"`).
/// * `particle_kinds_str` â€” Optional particle kind labels per final-state leg.
/// * `batch_size` â€” Number of events to generate (clamped to 1..=100).
pub fn generate_display_batch(
    cms_energy: f64,
    final_masses: &[f64],
    detector_preset: &str,
    particle_kinds_str: Option<&[String]>,
    batch_size: usize,
) -> SpireResult<Vec<EventDisplayData>> {
    let clamped = batch_size.clamp(1, 100);
    let mut results = Vec::with_capacity(clamped);
    for _ in 0..clamped {
        results.push(generate_display_event(
            cms_energy,
            final_masses,
            detector_preset,
            particle_kinds_str,
        )?);
    }
    Ok(results)
}

// ===========================================================================
// Analysis Configuration & Result
// ===========================================================================

/// Definition of a single plot to be filled during analysis.
///
/// Each plot specifies a Rhai observable script (which extracts a scalar
/// from each event) and the histogram binning parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotDefinition {
    /// Human-readable name (e.g., "Muon $p_T$").
    pub name: String,
    /// Rhai script returning a numeric observable value.
    /// The variable `event` (a `PhaseSpacePoint`) is in scope.
    pub observable_script: String,
    /// Number of histogram bins.
    pub n_bins: usize,
    /// Lower edge of the histogram range.
    pub min: f64,
    /// Upper edge of the histogram range.
    pub max: f64,
}

/// Definition of a 2D correlation plot to be filled during analysis.
///
/// Each 2D plot specifies two Rhai observable scripts (one per axis)
/// and the binning parameters for both axes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotDefinition2D {
    /// Human-readable name (e.g., "pT vs ÃŽÂ·").
    pub name: String,
    /// Rhai script returning the X-axis observable value.
    pub x_observable_script: String,
    /// Rhai script returning the Y-axis observable value.
    pub y_observable_script: String,
    /// Number of histogram bins on the X axis.
    pub nx: usize,
    /// Lower edge of the X-axis range.
    pub x_min: f64,
    /// Upper edge of the X-axis range.
    pub x_max: f64,
    /// Number of histogram bins on the Y axis.
    pub ny: usize,
    /// Lower edge of the Y-axis range.
    pub y_min: f64,
    /// Upper edge of the Y-axis range.
    pub y_max: f64,
}

/// Complete analysis configuration sent from the frontend.
///
/// Bundles the process definition, Monte Carlo parameters, observable
/// scripts, and optional kinematic cuts into a single request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    /// Plot definitions (each with its observable script and binning).
    pub plots: Vec<PlotDefinition>,
    /// Optional kinematic cut scripts (events failing any cut are discarded).
    pub cut_scripts: Vec<String>,
    /// Number of Monte Carlo events to generate.
    pub num_events: usize,
    /// Centre-of-mass energy in GeV.
    pub cms_energy: f64,
    /// Final-state particle masses in GeV.
    pub final_masses: Vec<f64>,

    // --- Detector simulation (optional) ---
    /// Detector preset name. When `Some`, the analysis pipeline will
    /// reconstruct events through a phenomenological detector model
    /// before evaluating observable scripts that reference `reco`.
    ///
    /// Supported values: `"perfect"`, `"lhc_like"`, `"ilc_like"`.
    /// When `None`, no detector simulation is applied and only truth-level
    /// observables (accessing `event`) are available.
    #[serde(default)]
    pub detector_preset: Option<String>,

    /// Classification of each final-state particle by detector subsystem.
    ///
    /// Must have the same length as `final_masses` when `detector_preset`
    /// is active. Each string maps to a [`ParticleKind`]:
    /// `"electron"`, `"muon"`, `"photon"`, `"hadron"`, `"invisible"`.
    ///
    /// When `None` and a detector preset is active, all final-state
    /// particles are assumed to be hadrons.
    #[serde(default)]
    pub particle_kinds: Option<Vec<String>>,

    /// Optional 2D correlation plot definitions.
    ///
    /// Each entry specifies two observable scripts (X and Y axes) and
    /// binning parameters for a 2D heatmap. These are filled alongside
    /// the standard 1D histograms.
    #[serde(default)]
    pub plots_2d: Option<Vec<PlotDefinition2D>>,
}

/// Complete analysis result returned to the frontend.
///
/// Contains the filled histograms and integration diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Filled histogram data for each requested plot.
    pub histograms: Vec<HistogramData>,
    /// Filled 2D histogram data for each requested 2D correlation plot.
    #[serde(default)]
    pub histograms_2d: Vec<Histogram2DData>,
    /// Estimated total cross-section (GeVÃ¢ÂÂ»Ã‚Â²).
    pub cross_section: f64,
    /// Statistical uncertainty on the cross-section.
    pub cross_section_error: f64,
    /// Total events generated.
    pub events_generated: usize,
    /// Events passing all kinematic cuts.
    pub events_passed: usize,
    /// Optional performance profile from the analysis pipeline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<crate::telemetry::ComputeProfile>,
}

// ===========================================================================
// Analysis Runner
// ===========================================================================

/// Execute a complete analysis pipeline.
///
/// This is the main orchestration function that:
/// 1. Compiles all observable and cut scripts via the Rhai engine.
/// 2. Initialises histograms for each plot definition.
/// 3. Runs the Monte Carlo event loop, applying cuts and filling histograms.
/// 4. Returns the filled histogram data alongside cross-section estimates.
///
/// # Arguments
///
/// * `config` : The complete analysis configuration.
/// * `integrand` : The squared matrix element $|\mathcal{M}|^2$.
/// * `generator` : Phase-space generator (e.g., RAMBO).
///
/// # Performance
///
/// All scripts are compiled once before the event loop begins.
/// The hot loop evaluates only pre-compiled ASTs and performs
/// zero-allocation histogram fills.
pub fn run_analysis<F>(
    config: &AnalysisConfig,
    integrand: F,
    generator: &mut dyn PhaseSpaceGenerator,
) -> SpireResult<AnalysisResult>
where
    F: Fn(&PhaseSpacePoint) -> f64,
{
    let engine = SpireScriptEngine::new();

    // --- Compile all observable scripts ---
    let observables: Vec<RhaiObservable> = config
        .plots
        .iter()
        .map(|plot| {
            engine
                .compile_observable(&plot.observable_script)
                .map(|obs| obs.with_name(&plot.name))
        })
        .collect::<SpireResult<Vec<_>>>()?;

    // --- Compile all cut scripts ---
    let cuts: Vec<crate::scripting::RhaiCut> = config
        .cut_scripts
        .iter()
        .map(|script| engine.compile_cut(script))
        .collect::<SpireResult<Vec<_>>>()?;

    // --- Initialise histograms ---
    let mut histograms: Vec<Histogram1D> = config
        .plots
        .iter()
        .map(|plot| Histogram1D::new(plot.n_bins, plot.min, plot.max))
        .collect();

    // --- Compile and initialise 2D plots ---
    let plots_2d = config.plots_2d.as_deref().unwrap_or(&[]);
    let observables_2d: Vec<(RhaiObservable, RhaiObservable)> = plots_2d
        .iter()
        .map(|p| {
            let x = engine.compile_observable(&p.x_observable_script)?;
            let y = engine.compile_observable(&p.y_observable_script)?;
            Ok((x, y))
        })
        .collect::<SpireResult<Vec<_>>>()?;

    let mut histograms_2d: Vec<Histogram2D> = plots_2d
        .iter()
        .map(|p| Histogram2D::new(p.nx, p.x_min, p.x_max, p.ny, p.y_min, p.y_max))
        .collect();

    // --- Monte Carlo event loop ---
    let mut sum_fw = 0.0_f64;
    let mut sum_fw2 = 0.0_f64;
    let mut n_generated = 0_usize;
    let mut n_passed = 0_usize;

    for _ in 0..config.num_events {
        let event = generator.generate_event(config.cms_energy, &config.final_masses)?;
        n_generated += 1;

        // Apply kinematic cuts.
        let passed = cuts
            .iter()
            .all(|cut| crate::scripting::KinematicCut::is_passed(cut, &event));
        if !passed {
            continue;
        }
        n_passed += 1;

        // Evaluate the integrand (|M|Ã‚Â²).
        let f_val = integrand(&event);
        if !f_val.is_finite() || !event.weight.is_finite() {
            continue;
        }

        let fw = f_val * event.weight;
        sum_fw += fw;
        sum_fw2 += fw * fw;

        // Fill each 1D histogram with the corresponding observable value.
        for (obs, hist) in observables.iter().zip(histograms.iter_mut()) {
            let value = obs.evaluate(&event);
            if value.is_finite() {
                hist.fill(value, fw);
            }
        }

        // Fill each 2D histogram with (x, y) observable values.
        for ((x_obs, y_obs), hist) in observables_2d.iter().zip(histograms_2d.iter_mut()) {
            let x_val = x_obs.evaluate(&event);
            let y_val = y_obs.evaluate(&event);
            if x_val.is_finite() && y_val.is_finite() {
                hist.fill(x_val, y_val, fw);
            }
        }
    }

    // --- Compute cross-section statistics ---
    let s = config.cms_energy * config.cms_energy;
    let flux_factor = 2.0 * s;
    let n_f = config.num_events as f64;

    let (cross_section, cross_section_error) = if n_f > 0.0 && flux_factor > 1e-300 {
        let mean = sum_fw / n_f;
        let variance = (sum_fw2 / n_f - mean * mean).max(0.0);
        let std_error = (variance / n_f).sqrt();
        (mean / flux_factor, std_error / flux_factor)
    } else {
        (0.0, 0.0)
    };

    // --- Serialize histograms ---
    let histogram_data: Vec<HistogramData> = config
        .plots
        .iter()
        .zip(histograms.iter())
        .map(|(plot, hist)| hist.to_data(&plot.name))
        .collect();

    let histogram_2d_data: Vec<Histogram2DData> = plots_2d
        .iter()
        .zip(histograms_2d.iter())
        .map(|(plot, hist)| hist.to_data_2d(&plot.name))
        .collect();

    Ok(AnalysisResult {
        histograms: histogram_data,
        histograms_2d: histogram_2d_data,
        cross_section,
        cross_section_error,
        events_generated: n_generated,
        events_passed: n_passed,
        profile: None,
    })
}

/// Execute the analysis pipeline with parallel event evaluation.
///
/// Events are generated sequentially (the generator is not `Send`),
/// then the cuts, observables, and histogram filling are evaluated in
/// parallel using rayon. Thread-local histograms are merged at the end.
pub fn run_analysis_parallel<F>(
    config: &AnalysisConfig,
    integrand: F,
    generator: &mut dyn PhaseSpaceGenerator,
) -> SpireResult<AnalysisResult>
where
    F: Fn(&PhaseSpacePoint) -> f64 + Send + Sync,
{
    use rayon::prelude::*;

    let engine = SpireScriptEngine::new();

    // --- Compile all scripts ---
    let observables: Vec<RhaiObservable> = config
        .plots
        .iter()
        .map(|plot| {
            engine
                .compile_observable(&plot.observable_script)
                .map(|obs| obs.with_name(&plot.name))
        })
        .collect::<SpireResult<Vec<_>>>()?;

    let cuts: Vec<crate::scripting::RhaiCut> = config
        .cut_scripts
        .iter()
        .map(|script| engine.compile_cut(script))
        .collect::<SpireResult<Vec<_>>>()?;

    // --- Generate events sequentially ---
    let mut events = Vec::with_capacity(config.num_events);
    for _ in 0..config.num_events {
        events.push(generator.generate_event(config.cms_energy, &config.final_masses)?);
    }

    let n_generated = events.len();

    // --- Parallel evaluation with thread-local histograms ---
    // Each rayon task returns (sum_fw, sum_fw2, n_passed, thread_local_histograms).
    let initial_hists: Vec<Histogram1D> = config
        .plots
        .iter()
        .map(|plot| Histogram1D::new(plot.n_bins, plot.min, plot.max))
        .collect();

    let chunk_size = {
        if let Ok(value) = std::env::var("SPIRE_ANALYSIS_CHUNK_SIZE") {
            if let Ok(parsed) = value.parse::<usize>() {
                if parsed > 0 {
                    parsed
                } else {
                    let threads = rayon::current_num_threads().max(1);
                    let target = events.len() / (threads * 8);
                    target.max(64).min(2048)
                }
            } else {
                let threads = rayon::current_num_threads().max(1);
                let target = events.len() / (threads * 8);
                target.max(64).min(2048)
            }
        } else {
            let threads = rayon::current_num_threads().max(1);
            let target = events.len() / (threads * 8);
            target.max(64).min(2048)
        }
    };

    let (sum_fw, sum_fw2, n_passed, merged_hists) = events
        .par_chunks(chunk_size)
        .map(|chunk| {
            let mut s_fw = 0.0_f64;
            let mut s_fw2 = 0.0_f64;
            let mut n_p = 0_usize;
            let mut hists = initial_hists.clone();

            for event in chunk {
                // Apply cuts.
                let passed = cuts
                    .iter()
                    .all(|cut| crate::scripting::KinematicCut::is_passed(cut, event));
                if !passed {
                    continue;
                }
                n_p += 1;

                let f_val = integrand(event);
                if !f_val.is_finite() || !event.weight.is_finite() {
                    continue;
                }

                let fw = f_val * event.weight;
                s_fw += fw;
                s_fw2 += fw * fw;

                // Fill chunk-local histograms.
                for (obs, hist) in observables.iter().zip(hists.iter_mut()) {
                    let value = obs.evaluate(event);
                    if value.is_finite() {
                        hist.fill(value, fw);
                    }
                }
            }

            (s_fw, s_fw2, n_p, hists)
        })
        .reduce(
            || (0.0_f64, 0.0_f64, 0_usize, initial_hists.clone()),
            |(a_fw, a_fw2, a_n, mut a_hists), (b_fw, b_fw2, b_n, b_hists)| {
                // Merge thread-local histograms.
                for (a, b) in a_hists.iter_mut().zip(b_hists.iter()) {
                    let _ = a.merge(b);
                }
                (a_fw + b_fw, a_fw2 + b_fw2, a_n + b_n, a_hists)
            },
        );

    // --- Compute cross-section statistics ---
    let s = config.cms_energy * config.cms_energy;
    let flux_factor = 2.0 * s;
    let n_f = config.num_events as f64;

    let (cross_section, cross_section_error) = if n_f > 0.0 && flux_factor > 1e-300 {
        let mean = sum_fw / n_f;
        let variance = (sum_fw2 / n_f - mean * mean).max(0.0);
        let std_error = (variance / n_f).sqrt();
        (mean / flux_factor, std_error / flux_factor)
    } else {
        (0.0, 0.0)
    };

    // --- Serialize histograms ---
    let histogram_data: Vec<HistogramData> = config
        .plots
        .iter()
        .zip(merged_hists.iter())
        .map(|(plot, hist)| hist.to_data(&plot.name))
        .collect();

    Ok(AnalysisResult {
        histograms: histogram_data,
        histograms_2d: vec![],
        cross_section,
        cross_section_error,
        events_generated: n_generated,
        events_passed: n_passed,
        profile: None,
    })
}

// ===========================================================================
// Reconstruction-Aware Analysis Pipeline
// ===========================================================================

/// Parse a particle-kind string into the corresponding enum.
fn parse_particle_kind(s: &str) -> SpireResult<ParticleKind> {
    match s.to_lowercase().as_str() {
        "electron" | "e" => Ok(ParticleKind::Electron),
        "muon" | "mu" => Ok(ParticleKind::Muon),
        "photon" | "gamma" | "a" => Ok(ParticleKind::Photon),
        "hadron" | "jet" | "q" | "g" => Ok(ParticleKind::Hadron),
        "invisible" | "neutrino" | "nu" => Ok(ParticleKind::Invisible),
        _ => Err(SpireError::InternalError(format!(
            "unknown particle kind '{s}'; expected one of: \
             electron, muon, photon, hadron, invisible"
        ))),
    }
}

/// Execute a reconstruction-aware analysis pipeline.
///
/// This function extends the standard analysis pipeline with an optional
/// phenomenological detector simulation step. When `config.detector_preset`
/// is `Some`, each truth-level event is passed through the [`DetectorModel`]
/// to produce a [`ReconstructedEvent`](crate::reco::detector::ReconstructedEvent) before observable evaluation.
///
/// Observable scripts may access both:
/// - `event` : the truth-level [`PhaseSpacePoint`]
/// - `reco` : the reconstructed [`ReconstructedEvent`](crate::reco::detector::ReconstructedEvent) (jets, leptons, MET)
///
/// When no detector preset is specified, this function delegates to
/// [`run_analysis`] for backward compatibility.
///
/// # Arguments
///
/// * `config` : Analysis configuration including optional detector settings.
/// * `integrand` : The squared matrix element $|\mathcal{M}|^2$.
/// * `generator` : Phase-space generator (e.g., RAMBO).
pub fn run_reco_analysis<F>(
    config: &AnalysisConfig,
    integrand: F,
    generator: &mut dyn PhaseSpaceGenerator,
) -> SpireResult<AnalysisResult>
where
    F: Fn(&PhaseSpacePoint) -> f64,
{
    // If no detector is requested, fall back to the standard pipeline.
    let detector_preset = match &config.detector_preset {
        Some(name) if !name.is_empty() => name.clone(),
        _ => return run_analysis(config, integrand, generator),
    };

    // Build the detector model.
    let detector = DetectorModel::from_preset(&detector_preset).ok_or_else(|| {
        SpireError::InternalError(format!(
            "unknown detector preset '{detector_preset}'; \
             expected one of: perfect, lhc_like, ilc_like"
        ))
    })?;

    // Parse particle kinds for each final-state leg.
    let n_final = config.final_masses.len();
    let particle_kinds: Vec<ParticleKind> = match &config.particle_kinds {
        Some(kinds) => {
            if kinds.len() != n_final {
                return Err(SpireError::InternalError(format!(
                    "particle_kinds length ({}) must match final_masses length ({n_final})",
                    kinds.len()
                )));
            }
            kinds
                .iter()
                .map(|s| parse_particle_kind(s))
                .collect::<SpireResult<Vec<_>>>()?
        }
        // Default: treat all final-state particles as hadrons.
        None => vec![ParticleKind::Hadron; n_final],
    };

    let engine = SpireScriptEngine::new();

    // --- Compile all observable scripts (reco-aware) ---
    let observables: Vec<RhaiRecoObservable> = config
        .plots
        .iter()
        .map(|plot| {
            engine
                .compile_reco_observable(&plot.observable_script)
                .map(|obs| obs.with_name(&plot.name))
        })
        .collect::<SpireResult<Vec<_>>>()?;

    // --- Compile all cut scripts (truth-level : applied before reconstruction) ---
    let cuts: Vec<crate::scripting::RhaiCut> = config
        .cut_scripts
        .iter()
        .map(|script| engine.compile_cut(script))
        .collect::<SpireResult<Vec<_>>>()?;

    // --- Initialise histograms ---
    let mut histograms: Vec<Histogram1D> = config
        .plots
        .iter()
        .map(|plot| Histogram1D::new(plot.n_bins, plot.min, plot.max))
        .collect();

    // --- Compile and initialise 2D plots (reco-aware) ---
    let plots_2d = config.plots_2d.as_deref().unwrap_or(&[]);
    let observables_2d: Vec<(RhaiRecoObservable, RhaiRecoObservable)> = plots_2d
        .iter()
        .map(|p| {
            let x = engine.compile_reco_observable(&p.x_observable_script)?;
            let y = engine.compile_reco_observable(&p.y_observable_script)?;
            Ok((x, y))
        })
        .collect::<SpireResult<Vec<_>>>()?;

    let mut histograms_2d: Vec<Histogram2D> = plots_2d
        .iter()
        .map(|p| Histogram2D::new(p.nx, p.x_min, p.x_max, p.ny, p.y_min, p.y_max))
        .collect();

    // --- Seeded RNG for reproducible detector smearing ---
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    let mut rng = StdRng::seed_from_u64(42);

    // --- Monte Carlo event loop ---
    let mut sum_fw = 0.0_f64;
    let mut sum_fw2 = 0.0_f64;
    let mut n_generated = 0_usize;
    let mut n_passed = 0_usize;

    for _ in 0..config.num_events {
        let event = generator.generate_event(config.cms_energy, &config.final_masses)?;
        n_generated += 1;

        // Apply truth-level kinematic cuts.
        let passed = cuts
            .iter()
            .all(|cut| crate::scripting::KinematicCut::is_passed(cut, &event));
        if !passed {
            continue;
        }
        n_passed += 1;

        // Evaluate the integrand (|M|Ã‚Â²).
        let f_val = integrand(&event);
        if !f_val.is_finite() || !event.weight.is_finite() {
            continue;
        }

        let fw = f_val * event.weight;
        sum_fw += fw;
        sum_fw2 += fw * fw;

        // Reconstruct the event through the detector model.
        let reco =
            crate::reco::detector::reconstruct_event(&event, &particle_kinds, &detector, &mut rng);

        // Fill each 1D histogram with the corresponding reco observable value.
        for (obs, hist) in observables.iter().zip(histograms.iter_mut()) {
            let value = obs.evaluate(&reco, &event);
            if value.is_finite() {
                hist.fill(value, fw);
            }
        }

        // Fill each 2D histogram with (x, y) reco observable values.
        for ((x_obs, y_obs), hist) in observables_2d.iter().zip(histograms_2d.iter_mut()) {
            let x_val = x_obs.evaluate(&reco, &event);
            let y_val = y_obs.evaluate(&reco, &event);
            if x_val.is_finite() && y_val.is_finite() {
                hist.fill(x_val, y_val, fw);
            }
        }
    }

    // --- Compute cross-section statistics ---
    let s = config.cms_energy * config.cms_energy;
    let flux_factor = 2.0 * s;
    let n_f = config.num_events as f64;

    let (cross_section, cross_section_error) = if n_f > 0.0 && flux_factor > 1e-300 {
        let mean = sum_fw / n_f;
        let variance = (sum_fw2 / n_f - mean * mean).max(0.0);
        let std_error = (variance / n_f).sqrt();
        (mean / flux_factor, std_error / flux_factor)
    } else {
        (0.0, 0.0)
    };

    // --- Serialize histograms ---
    let histogram_data: Vec<HistogramData> = config
        .plots
        .iter()
        .zip(histograms.iter())
        .map(|(plot, hist)| hist.to_data(&plot.name))
        .collect();

    let histogram_2d_data: Vec<Histogram2DData> = plots_2d
        .iter()
        .zip(histograms_2d.iter())
        .map(|(plot, hist)| hist.to_data_2d(&plot.name))
        .collect();

    Ok(AnalysisResult {
        histograms: histogram_data,
        histograms_2d: histogram_2d_data,
        cross_section,
        cross_section_error,
        events_generated: n_generated,
        events_passed: n_passed,
        profile: None,
    })
}

// ===========================================================================
// Unit Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Histogram1D tests
    // -----------------------------------------------------------------------

    #[test]
    fn histogram1d_new() {
        let h = Histogram1D::new(10, 0.0, 100.0);
        assert_eq!(h.n_bins(), 10);
        assert_eq!(h.min(), 0.0);
        assert_eq!(h.max(), 100.0);
        assert!((h.bin_width() - 10.0).abs() < 1e-12);
        assert_eq!(h.entries(), 0);
        assert_eq!(h.total_weight(), 0.0);
    }

    #[test]
    #[should_panic(expected = "at least one bin")]
    fn histogram1d_zero_bins_panics() {
        let _h = Histogram1D::new(0, 0.0, 100.0);
    }

    #[test]
    #[should_panic(expected = "less than max")]
    fn histogram1d_invalid_range_panics() {
        let _h = Histogram1D::new(10, 100.0, 0.0);
    }

    #[test]
    fn histogram1d_fill_in_range() {
        let mut h = Histogram1D::new(10, 0.0, 100.0);
        // Fill value 15.0 Ã¢â€ â€™ bin 1 (covers [10, 20))
        h.fill(15.0, 1.0);
        assert_eq!(h.entries(), 1);
        assert!((h.total_weight() - 1.0).abs() < 1e-12);
        assert!((h.bin_contents()[1] - 1.0).abs() < 1e-12);
        assert!(h.underflow().abs() < 1e-12);
        assert!(h.overflow().abs() < 1e-12);
    }

    #[test]
    fn histogram1d_fill_underflow() {
        let mut h = Histogram1D::new(10, 0.0, 100.0);
        h.fill(-5.0, 2.0);
        assert!((h.underflow() - 2.0).abs() < 1e-12);
        assert_eq!(h.entries(), 1);
    }

    #[test]
    fn histogram1d_fill_overflow() {
        let mut h = Histogram1D::new(10, 0.0, 100.0);
        h.fill(100.0, 3.0); // value == max Ã¢â€ â€™ overflow
        h.fill(150.0, 1.0);
        assert!((h.overflow() - 4.0).abs() < 1e-12);
        assert_eq!(h.entries(), 2);
    }

    #[test]
    fn histogram1d_fill_weighted() {
        let mut h = Histogram1D::new(5, 0.0, 50.0);
        // Bin width = 10. Value 5.0 Ã¢â€ â€™ bin 0.
        h.fill(5.0, 3.5);
        h.fill(5.0, 1.5);
        assert!((h.bin_contents()[0] - 5.0).abs() < 1e-12);
        assert!((h.total_weight() - 5.0).abs() < 1e-12);
    }

    #[test]
    fn histogram1d_bin_edges() {
        let h = Histogram1D::new(4, 0.0, 40.0);
        let edges = h.bin_edges();
        assert_eq!(edges.len(), 5);
        assert!((edges[0] - 0.0).abs() < 1e-12);
        assert!((edges[1] - 10.0).abs() < 1e-12);
        assert!((edges[4] - 40.0).abs() < 1e-12);
    }

    #[test]
    fn histogram1d_bin_centres() {
        let h = Histogram1D::new(4, 0.0, 40.0);
        let centres = h.bin_centres();
        assert_eq!(centres.len(), 4);
        assert!((centres[0] - 5.0).abs() < 1e-12);
        assert!((centres[1] - 15.0).abs() < 1e-12);
    }

    #[test]
    fn histogram1d_merge() {
        let mut h1 = Histogram1D::new(5, 0.0, 50.0);
        let mut h2 = Histogram1D::new(5, 0.0, 50.0);

        h1.fill(5.0, 1.0);
        h1.fill(15.0, 2.0);
        h2.fill(5.0, 3.0);
        h2.fill(-1.0, 0.5);

        h1.merge(&h2).unwrap();

        assert!((h1.bin_contents()[0] - 4.0).abs() < 1e-12); // 1.0 + 3.0
        assert!((h1.bin_contents()[1] - 2.0).abs() < 1e-12);
        assert!((h1.underflow() - 0.5).abs() < 1e-12);
        assert_eq!(h1.entries(), 4);
    }

    #[test]
    fn histogram1d_merge_different_binning_fails() {
        let mut h1 = Histogram1D::new(5, 0.0, 50.0);
        let h2 = Histogram1D::new(10, 0.0, 50.0);
        assert!(h1.merge(&h2).is_err());
    }

    #[test]
    fn histogram1d_reset() {
        let mut h = Histogram1D::new(5, 0.0, 50.0);
        h.fill(5.0, 1.0);
        h.fill(-1.0, 0.5);
        h.reset();
        assert_eq!(h.entries(), 0);
        assert!(h.total_weight().abs() < 1e-12);
        assert!(h.bin_contents().iter().all(|&b| b == 0.0));
    }

    #[test]
    fn histogram1d_max_bin() {
        let mut h = Histogram1D::new(10, 0.0, 100.0);
        h.fill(55.0, 10.0); // bin 5
        h.fill(15.0, 3.0); // bin 1
        h.fill(55.0, 5.0); // bin 5 again
        assert_eq!(h.max_bin(), 5);
    }

    #[test]
    fn histogram1d_mean() {
        let mut h = Histogram1D::new(10, 0.0, 100.0);
        // Fill 100 entries centred around 50 (all weight 1.0).
        for _ in 0..100 {
            h.fill(50.0, 1.0);
        }
        assert!((h.mean() - 55.0).abs() < 1e-8); // bin centre at 55 since 50 Ã¢â€ â€™ bin 5 Ã¢â€ â€™ centre 55
    }

    #[test]
    fn histogram1d_bin_errors() {
        let mut h = Histogram1D::new(5, 0.0, 50.0);
        h.fill(5.0, 2.0);
        h.fill(5.0, 3.0);
        let errors = h.bin_errors();
        // bin_sq[0] = 4 + 9 = 13, error = sqrt(13)
        assert!((errors[0] - 13.0_f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn histogram1d_to_data() {
        let mut h = Histogram1D::new(5, 0.0, 50.0);
        h.fill(5.0, 1.0);
        let data = h.to_data("Test Plot");
        assert_eq!(data.name, "Test Plot");
        assert_eq!(data.bin_edges.len(), 6);
        assert_eq!(data.bin_contents.len(), 5);
        assert_eq!(data.bin_errors.len(), 5);
        assert_eq!(data.entries, 1);
    }

    #[test]
    fn histogram1d_edge_case_value_at_boundary() {
        let mut h = Histogram1D::new(10, 0.0, 100.0);
        // Value exactly at a bin boundary.
        h.fill(10.0, 1.0); // Should go to bin 1 (covers [10, 20))
        assert!((h.bin_contents()[1] - 1.0).abs() < 1e-12);
        // Value exactly at min.
        h.fill(0.0, 1.0); // Should go to bin 0
        assert!((h.bin_contents()[0] - 1.0).abs() < 1e-12);
    }

    // -----------------------------------------------------------------------
    // Histogram2D tests
    // -----------------------------------------------------------------------

    #[test]
    fn histogram2d_new() {
        let h = Histogram2D::new(10, 0.0, 100.0, 5, -2.5, 2.5);
        assert_eq!(h.nx(), 10);
        assert_eq!(h.ny(), 5);
        assert_eq!(h.bin_contents().len(), 50);
        assert_eq!(h.entries(), 0);
    }

    #[test]
    fn histogram2d_fill() {
        let mut h = Histogram2D::new(10, 0.0, 100.0, 10, 0.0, 100.0);
        h.fill(15.0, 15.0, 1.0); // ix=1, iy=1 Ã¢â€ â€™ index = 11
        assert!((h.bin_contents()[11] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn histogram2d_fill_out_of_range() {
        let mut h = Histogram2D::new(10, 0.0, 100.0, 10, 0.0, 100.0);
        h.fill(-5.0, 50.0, 1.0); // X out of range : ignored
        h.fill(50.0, 150.0, 1.0); // Y out of range : ignored
        assert!(h.bin_contents().iter().all(|&b| b == 0.0));
        assert_eq!(h.entries(), 2);
    }

    #[test]
    fn histogram2d_merge() {
        let mut h1 = Histogram2D::new(5, 0.0, 50.0, 5, 0.0, 50.0);
        let mut h2 = Histogram2D::new(5, 0.0, 50.0, 5, 0.0, 50.0);
        h1.fill(5.0, 5.0, 2.0);
        h2.fill(5.0, 5.0, 3.0);
        h1.merge(&h2).unwrap();
        assert!((h1.bin_contents()[0] - 5.0).abs() < 1e-12);
    }

    // -----------------------------------------------------------------------
    // Analysis pipeline tests
    // -----------------------------------------------------------------------

    #[test]
    fn plot_definition_serde() {
        let plot = PlotDefinition {
            name: "Muon pT".into(),
            observable_script: "event.momenta[2].pt()".into(),
            n_bins: 50,
            min: 0.0,
            max: 100.0,
        };
        let json = serde_json::to_string(&plot).unwrap();
        let back: PlotDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "Muon pT");
        assert_eq!(back.n_bins, 50);
    }

    #[test]
    fn analysis_config_serde() {
        let config = AnalysisConfig {
            plots: vec![PlotDefinition {
                name: "pT".into(),
                observable_script: "event.momenta[0].pt()".into(),
                n_bins: 20,
                min: 0.0,
                max: 100.0,
            }],
            cut_scripts: vec![],
            num_events: 1000,
            cms_energy: 100.0,
            final_masses: vec![0.0, 0.0],
            detector_preset: None,
            particle_kinds: None,
            plots_2d: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: AnalysisConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.num_events, 1000);
        assert_eq!(back.plots.len(), 1);
    }

    #[test]
    fn analysis_result_serde() {
        let result = AnalysisResult {
            histograms: vec![],
            cross_section: 1.23e-6,
            cross_section_error: 4.56e-8,
            events_generated: 1000,
            events_passed: 800,
            histograms_2d: vec![],
            profile: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: AnalysisResult = serde_json::from_str(&json).unwrap();
        assert!((back.cross_section - 1.23e-6).abs() < 1e-15);
    }

    #[test]
    fn run_analysis_basic() {
        use crate::kinematics::RamboGenerator;

        let config = AnalysisConfig {
            plots: vec![PlotDefinition {
                name: "Leading pT".into(),
                observable_script: "event.momenta[0].pt()".into(),
                n_bins: 20,
                min: 0.0,
                max: 60.0,
            }],
            cut_scripts: vec![],
            num_events: 500,
            cms_energy: 100.0,
            final_masses: vec![0.0, 0.0],
            detector_preset: None,
            particle_kinds: None,
            plots_2d: None,
        };

        let mut gen = RamboGenerator::new();
        let result = run_analysis(&config, |_| 1.0, &mut gen).unwrap();

        assert_eq!(result.events_generated, 500);
        assert_eq!(result.events_passed, 500); // no cuts
        assert_eq!(result.histograms.len(), 1);

        let h = &result.histograms[0];
        assert_eq!(h.name, "Leading pT");
        assert_eq!(h.bin_contents.len(), 20);
        assert!(h.entries > 0);
    }

    #[test]
    fn run_analysis_with_cuts() {
        use crate::kinematics::RamboGenerator;

        let config = AnalysisConfig {
            plots: vec![PlotDefinition {
                name: "pT after cut".into(),
                observable_script: "event.momenta[0].pt()".into(),
                n_bins: 10,
                min: 0.0,
                max: 60.0,
            }],
            cut_scripts: vec!["event.momenta[0].pt() > 40.0".into()],
            num_events: 2000,
            cms_energy: 100.0,
            final_masses: vec![0.0, 0.0],
            detector_preset: None,
            particle_kinds: None,
            plots_2d: None,
        };

        let mut gen = RamboGenerator::new();
        let result = run_analysis(&config, |_| 1.0, &mut gen).unwrap();

        // The cut should reject some events.
        assert!(result.events_passed < result.events_generated);
        assert!(result.events_passed > 0);
    }

    #[test]
    fn run_analysis_multiple_plots() {
        use crate::kinematics::RamboGenerator;

        let config = AnalysisConfig {
            plots: vec![
                PlotDefinition {
                    name: "pT particle 0".into(),
                    observable_script: "event.momenta[0].pt()".into(),
                    n_bins: 10,
                    min: 0.0,
                    max: 60.0,
                },
                PlotDefinition {
                    name: "Energy particle 0".into(),
                    observable_script: "event.momenta[0].e()".into(),
                    n_bins: 10,
                    min: 0.0,
                    max: 60.0,
                },
            ],
            cut_scripts: vec![],
            num_events: 300,
            cms_energy: 100.0,
            final_masses: vec![0.0, 0.0],
            detector_preset: None,
            particle_kinds: None,
            plots_2d: None,
        };

        let mut gen = RamboGenerator::new();
        let result = run_analysis(&config, |_| 1.0, &mut gen).unwrap();

        assert_eq!(result.histograms.len(), 2);
        assert_eq!(result.histograms[0].name, "pT particle 0");
        assert_eq!(result.histograms[1].name, "Energy particle 0");
    }

    #[test]
    fn run_analysis_parallel_matches_sequential() {
        use crate::kinematics::RamboGenerator;

        let config = AnalysisConfig {
            plots: vec![PlotDefinition {
                name: "pT".into(),
                observable_script: "event.momenta[0].pt()".into(),
                n_bins: 10,
                min: 0.0,
                max: 60.0,
            }],
            cut_scripts: vec![],
            num_events: 1000,
            cms_energy: 100.0,
            final_masses: vec![0.0, 0.0],
            detector_preset: None,
            particle_kinds: None,
            plots_2d: None,
        };

        let mut gen1 = RamboGenerator::new();
        let seq = run_analysis(&config, |_| 1.0, &mut gen1).unwrap();

        let mut gen2 = RamboGenerator::new();
        let par = run_analysis_parallel(&config, |_| 1.0, &mut gen2).unwrap();

        // Both should process the same number of events.
        assert_eq!(seq.events_generated, par.events_generated);
        assert_eq!(seq.events_passed, par.events_passed);
        // Cross-sections should be very close (same events, same seed).
        assert!(
            (seq.cross_section - par.cross_section).abs() / seq.cross_section.abs().max(1e-300)
                < 0.01,
            "Sequential and parallel cross-sections should agree"
        );
    }

    #[test]
    fn run_analysis_cross_section_positive() {
        use crate::kinematics::RamboGenerator;

        let config = AnalysisConfig {
            plots: vec![PlotDefinition {
                name: "pT".into(),
                observable_script: "event.momenta[0].pt()".into(),
                n_bins: 10,
                min: 0.0,
                max: 60.0,
            }],
            cut_scripts: vec![],
            num_events: 5000,
            cms_energy: 100.0,
            final_masses: vec![0.0, 0.0],
            detector_preset: None,
            particle_kinds: None,
            plots_2d: None,
        };

        let mut gen = RamboGenerator::new();
        let result = run_analysis(&config, |_| 1.0, &mut gen).unwrap();

        assert!(
            result.cross_section > 0.0,
            "Cross-section should be positive"
        );
        assert!(result.cross_section_error >= 0.0);
    }

    #[test]
    fn run_analysis_invariant_mass_distribution() {
        use crate::kinematics::RamboGenerator;

        // For 2Ã¢â€ â€™2 massless at Ã¢Ë†Å¡s = 200 GeV, the invariant mass of the
        // pair should always be exactly 200 GeV (total 4-momentum conservation).
        let config = AnalysisConfig {
            plots: vec![PlotDefinition {
                name: "Invariant Mass".into(),
                observable_script: r#"
                    let p1 = event.momenta[0];
                    let p2 = event.momenta[1];
                    let total = p1 + p2;
                    total.m()
                "#
                .into(),
                n_bins: 20,
                min: 150.0,
                max: 250.0,
            }],
            cut_scripts: vec![],
            num_events: 500,
            cms_energy: 200.0,
            final_masses: vec![0.0, 0.0],
            detector_preset: None,
            particle_kinds: None,
            plots_2d: None,
        };

        let mut gen = RamboGenerator::new();
        let result = run_analysis(&config, |_| 1.0, &mut gen).unwrap();

        let h = &result.histograms[0];
        // The invariant mass should peak around 200 GeV (bin containing 200).
        // Bin 10 covers [200, 205) Ã¢â€ â€™ the peak should be in bin 10.
        let peak_bin = h
            .bin_contents
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap();
        // The peak bin centre should be close to 200 GeV.
        let bin_width = (250.0 - 150.0) / 20.0;
        let peak_centre = 150.0 + (peak_bin as f64 + 0.5) * bin_width;
        assert!(
            (peak_centre - 200.0).abs() < bin_width,
            "Invariant mass peak at {} should be near 200 GeV",
            peak_centre
        );
    }

    #[test]
    fn run_analysis_pt_distribution_shape() {
        use crate::kinematics::RamboGenerator;

        // For 2-body massless at Ã¢Ë†Å¡s = 100 GeV, pT of each particle is
        // (Ã¢Ë†Å¡s / 2) * sin(ÃŽÂ¸) = 50 * sin(ÃŽÂ¸). Since ÃŽÂ¸ is isotropic in CM,
        // the pT distribution should peak near pT Ã¢â€°Ë† 50 GeV (ÃŽÂ¸ Ã¢â€°Ë† Ãâ‚¬/2).
        let config = AnalysisConfig {
            plots: vec![PlotDefinition {
                name: "pT distribution".into(),
                observable_script: "event.momenta[0].pt()".into(),
                n_bins: 25,
                min: 0.0,
                max: 55.0,
            }],
            cut_scripts: vec![],
            num_events: 10000,
            cms_energy: 100.0,
            final_masses: vec![0.0, 0.0],
            detector_preset: None,
            particle_kinds: None,
            plots_2d: None,
        };

        let mut gen = RamboGenerator::new();
        let result = run_analysis(&config, |_| 1.0, &mut gen).unwrap();

        let h = &result.histograms[0];
        // The peak should be in the last few bins (near pT = 50 GeV).
        let peak_bin = h
            .bin_contents
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap();
        // Peak bin should be in the upper half of the range.
        assert!(
            peak_bin >= 10,
            "pT peak bin {} should be in the upper half (near 50 GeV)",
            peak_bin
        );
    }

    #[test]
    fn run_analysis_invalid_script_returns_error() {
        use crate::kinematics::RamboGenerator;

        let config = AnalysisConfig {
            plots: vec![PlotDefinition {
                name: "Bad script".into(),
                observable_script: "let = ;; bad".into(),
                n_bins: 10,
                min: 0.0,
                max: 100.0,
            }],
            cut_scripts: vec![],
            num_events: 100,
            cms_energy: 100.0,
            final_masses: vec![0.0, 0.0],
            detector_preset: None,
            particle_kinds: None,
            plots_2d: None,
        };

        let mut gen = RamboGenerator::new();
        let result = run_analysis(&config, |_| 1.0, &mut gen);
        assert!(result.is_err());
    }

    #[test]
    fn histogram_data_serde_roundtrip() {
        let data = HistogramData {
            name: "Test".into(),
            bin_edges: vec![0.0, 10.0, 20.0, 30.0],
            bin_contents: vec![5.0, 10.0, 3.0],
            bin_errors: vec![2.236, 3.162, 1.732],
            underflow: 0.5,
            overflow: 1.2,
            entries: 100,
            mean: 15.0,
        };
        let json = serde_json::to_string(&data).unwrap();
        let back: HistogramData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "Test");
        assert_eq!(back.bin_contents.len(), 3);
        assert_eq!(back.entries, 100);
    }

    // -----------------------------------------------------------------------
    // Reconstruction-Aware Analysis Tests
    // -----------------------------------------------------------------------

    #[test]
    fn run_reco_analysis_no_detector_delegates_to_standard() {
        use crate::kinematics::RamboGenerator;
        // With detector_preset = None, run_reco_analysis should behave
        // identically to run_analysis.
        let config = AnalysisConfig {
            plots: vec![PlotDefinition {
                name: "Leading pT".into(),
                observable_script: "let p = event.momenta[0]; p.pt()".into(),
                n_bins: 20,
                min: 0.0,
                max: 100.0,
            }],
            cut_scripts: vec![],
            num_events: 200,
            cms_energy: 200.0,
            final_masses: vec![0.0, 0.0],
            detector_preset: None,
            particle_kinds: None,
            plots_2d: None,
        };

        let mut gen = RamboGenerator::new();
        let result = run_reco_analysis(&config, |_| 1.0, &mut gen).unwrap();
        assert_eq!(result.events_generated, 200);
        assert_eq!(result.events_passed, 200);
        assert_eq!(result.histograms.len(), 1);
        assert!(result.histograms[0].entries > 0);
    }

    #[test]
    fn run_reco_analysis_perfect_detector() {
        use crate::kinematics::RamboGenerator;
        // Perfect detector: no smearing, 100% efficiency.
        // A reco-level observable should yield values close to truth.
        let config = AnalysisConfig {
            plots: vec![PlotDefinition {
                name: "N jets".into(),
                observable_script: "reco.n_jets().to_float()".into(),
                n_bins: 10,
                min: 0.0,
                max: 10.0,
            }],
            cut_scripts: vec![],
            num_events: 100,
            cms_energy: 200.0,
            final_masses: vec![0.0, 0.0],
            detector_preset: Some("perfect".into()),
            particle_kinds: Some(vec!["hadron".into(), "hadron".into()]),
            plots_2d: None,
        };

        let mut gen = RamboGenerator::new();
        let result = run_reco_analysis(&config, |_| 1.0, &mut gen).unwrap();
        assert_eq!(result.events_generated, 100);
        assert!(result.events_passed > 0);
        assert_eq!(result.histograms.len(), 1);
        assert!(result.histograms[0].entries > 0);
    }

    #[test]
    fn run_reco_analysis_lhc_like_jet_pt() {
        use crate::kinematics::RamboGenerator;
        // LHC-like detector with 4-body hadronic final state.
        let config = AnalysisConfig {
            plots: vec![PlotDefinition {
                name: "Leading jet pT".into(),
                observable_script: r#"
                    let jets = reco.jets;
                    if jets.len() > 0 {
                        jets[0].pt()
                    } else {
                        -1.0
                    }
                "#
                .into(),
                n_bins: 20,
                min: 0.0,
                max: 300.0,
            }],
            cut_scripts: vec![],
            num_events: 200,
            cms_energy: 500.0,
            final_masses: vec![0.0, 0.0, 0.0, 0.0],
            detector_preset: Some("lhc_like".into()),
            particle_kinds: None, // default all-hadronic
            plots_2d: None,
        };

        let mut gen = RamboGenerator::new();
        let result = run_reco_analysis(&config, |_| 1.0, &mut gen).unwrap();
        assert_eq!(result.events_generated, 200);
        assert!(result.histograms[0].entries > 0);
    }

    #[test]
    fn run_reco_analysis_met_observable() {
        use crate::kinematics::RamboGenerator;
        // One visible + one invisible particle Ã¢â€ â€™ MET from invisible.
        let config = AnalysisConfig {
            plots: vec![PlotDefinition {
                name: "MET".into(),
                observable_script: "reco.met_pt()".into(),
                n_bins: 20,
                min: 0.0,
                max: 300.0,
            }],
            cut_scripts: vec![],
            num_events: 100,
            cms_energy: 200.0,
            final_masses: vec![0.0, 0.0],
            detector_preset: Some("perfect".into()),
            particle_kinds: Some(vec!["electron".into(), "invisible".into()]),
            plots_2d: None,
        };

        let mut gen = RamboGenerator::new();
        let result = run_reco_analysis(&config, |_| 1.0, &mut gen).unwrap();
        assert_eq!(result.events_generated, 100);
        // MET should be non-zero for most events (invisible carries momentum).
        assert!(result.histograms[0].entries > 0);
    }

    #[test]
    fn run_reco_analysis_invalid_preset_returns_error() {
        use crate::kinematics::RamboGenerator;
        let config = AnalysisConfig {
            plots: vec![],
            cut_scripts: vec![],
            num_events: 10,
            cms_energy: 100.0,
            final_masses: vec![0.0, 0.0],
            detector_preset: Some("nonexistent_detector".into()),
            particle_kinds: None,
            plots_2d: None,
        };

        let mut gen = RamboGenerator::new();
        let result = run_reco_analysis(&config, |_| 1.0, &mut gen);
        assert!(result.is_err());
    }

    #[test]
    fn run_reco_analysis_mismatched_particle_kinds_returns_error() {
        use crate::kinematics::RamboGenerator;
        let config = AnalysisConfig {
            plots: vec![],
            cut_scripts: vec![],
            num_events: 10,
            cms_energy: 100.0,
            final_masses: vec![0.0, 0.0],
            detector_preset: Some("lhc_like".into()),
            // 3 particle kinds but only 2 final masses.
            particle_kinds: Some(vec!["electron".into(), "muon".into(), "hadron".into()]),
            plots_2d: None,
        };

        let mut gen = RamboGenerator::new();
        let result = run_reco_analysis(&config, |_| 1.0, &mut gen);
        assert!(result.is_err());
    }

    #[test]
    fn run_reco_analysis_truth_and_reco_in_same_script() {
        use crate::kinematics::RamboGenerator;
        // Script accesses both `event` (truth) and `reco` (detector-level).
        // It adds the number of truth-level particles and the number of jets.
        let config = AnalysisConfig {
            plots: vec![PlotDefinition {
                name: "sum".into(),
                observable_script: "reco.n_jets().to_float() + event.momenta.len().to_float()"
                    .into(),
                n_bins: 20,
                min: 0.0,
                max: 20.0,
            }],
            cut_scripts: vec![],
            num_events: 100,
            cms_energy: 200.0,
            final_masses: vec![0.0, 0.0],
            detector_preset: Some("perfect".into()),
            particle_kinds: Some(vec!["hadron".into(), "hadron".into()]),
            plots_2d: None,
        };

        let mut gen = RamboGenerator::new();
        let result = run_reco_analysis(&config, |_| 1.0, &mut gen).unwrap();
        assert!(result.histograms[0].entries > 0);
    }

    #[test]
    fn run_reco_analysis_ilc_preset() {
        use crate::kinematics::RamboGenerator;
        let config = AnalysisConfig {
            plots: vec![PlotDefinition {
                name: "N jets".into(),
                observable_script: "reco.n_jets().to_float()".into(),
                n_bins: 10,
                min: 0.0,
                max: 10.0,
            }],
            cut_scripts: vec![],
            num_events: 50,
            cms_energy: 500.0,
            final_masses: vec![0.0, 0.0, 0.0, 0.0],
            detector_preset: Some("ilc_like".into()),
            particle_kinds: None,
            plots_2d: None,
        };

        let mut gen = RamboGenerator::new();
        let result = run_reco_analysis(&config, |_| 1.0, &mut gen).unwrap();
        assert!(result.events_passed > 0);
        assert!(result.histograms[0].entries > 0);
    }

    #[test]
    fn parse_particle_kind_variants() {
        use super::parse_particle_kind;
        use crate::reco::detector::ParticleKind;

        assert!(matches!(
            parse_particle_kind("electron"),
            Ok(ParticleKind::Electron)
        ));
        assert!(matches!(
            parse_particle_kind("e"),
            Ok(ParticleKind::Electron)
        ));
        assert!(matches!(
            parse_particle_kind("muon"),
            Ok(ParticleKind::Muon)
        ));
        assert!(matches!(parse_particle_kind("mu"), Ok(ParticleKind::Muon)));
        assert!(matches!(
            parse_particle_kind("photon"),
            Ok(ParticleKind::Photon)
        ));
        assert!(matches!(
            parse_particle_kind("gamma"),
            Ok(ParticleKind::Photon)
        ));
        assert!(matches!(
            parse_particle_kind("hadron"),
            Ok(ParticleKind::Hadron)
        ));
        assert!(matches!(
            parse_particle_kind("jet"),
            Ok(ParticleKind::Hadron)
        ));
        assert!(matches!(parse_particle_kind("q"), Ok(ParticleKind::Hadron)));
        assert!(matches!(parse_particle_kind("g"), Ok(ParticleKind::Hadron)));
        assert!(matches!(
            parse_particle_kind("invisible"),
            Ok(ParticleKind::Invisible)
        ));
        assert!(matches!(
            parse_particle_kind("neutrino"),
            Ok(ParticleKind::Invisible)
        ));
        assert!(matches!(
            parse_particle_kind("nu"),
            Ok(ParticleKind::Invisible)
        ));
        assert!(parse_particle_kind("unknown_type").is_err());
    }

    #[test]
    fn analysis_config_with_detector_serde() {
        let config = AnalysisConfig {
            plots: vec![PlotDefinition {
                name: "test".into(),
                observable_script: "reco.n_jets().to_float()".into(),
                n_bins: 10,
                min: 0.0,
                max: 100.0,
            }],
            cut_scripts: vec![],
            num_events: 100,
            cms_energy: 200.0,
            final_masses: vec![0.0, 0.0],
            detector_preset: Some("lhc_like".into()),
            particle_kinds: Some(vec!["hadron".into(), "hadron".into()]),
            plots_2d: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        let back: AnalysisConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.detector_preset, Some("lhc_like".into()));
        assert_eq!(back.particle_kinds.unwrap().len(), 2);
    }

    #[test]
    fn analysis_config_without_detector_deserializes() {
        // Legacy JSON without detector fields should deserialize cleanly.
        let json = r#"{
            "plots": [],
            "cut_scripts": [],
            "num_events": 10,
            "cms_energy": 100.0,
            "final_masses": [0.0, 0.0]
        }"#;
        let config: AnalysisConfig = serde_json::from_str(json).unwrap();
        assert!(config.detector_preset.is_none());
        assert!(config.particle_kinds.is_none());
        assert!(config.plots_2d.is_none());
    }

    // -----------------------------------------------------------------------
    // Histogram2D DTO tests
    // -----------------------------------------------------------------------

    #[test]
    fn histogram2d_to_data_2d() {
        let mut h = Histogram2D::new(5, 0.0, 50.0, 4, -2.0, 2.0);
        h.fill(10.0, 0.5, 1.0);
        h.fill(10.0, 0.5, 2.0);
        let data = h.to_data_2d("pT vs Î·");
        assert_eq!(data.name, "pT vs Î·");
        assert_eq!(data.nx, 5);
        assert_eq!(data.ny, 4);
        assert_eq!(data.x_bin_edges.len(), 6); // nx + 1
        assert_eq!(data.y_bin_edges.len(), 5); // ny + 1
        assert_eq!(data.bin_contents.len(), 20); // nx * ny
        assert_eq!(data.entries, 2);
        assert!((data.total_weight - 3.0).abs() < 1e-12);
    }

    #[test]
    fn histogram2d_x_bin_edges() {
        let h = Histogram2D::new(4, 0.0, 40.0, 2, 0.0, 2.0);
        let edges = h.x_bin_edges();
        assert_eq!(edges.len(), 5);
        assert!((edges[0] - 0.0).abs() < 1e-12);
        assert!((edges[2] - 20.0).abs() < 1e-12);
        assert!((edges[4] - 40.0).abs() < 1e-12);
    }

    #[test]
    fn histogram2d_y_bin_edges() {
        let h = Histogram2D::new(2, 0.0, 2.0, 5, -5.0, 5.0);
        let edges = h.y_bin_edges();
        assert_eq!(edges.len(), 6);
        assert!((edges[0] - (-5.0)).abs() < 1e-12);
        assert!((edges[5] - 5.0).abs() < 1e-12);
    }

    #[test]
    fn histogram2d_data_serde_roundtrip() {
        let data = Histogram2DData {
            name: "Test 2D".into(),
            x_bin_edges: vec![0.0, 10.0, 20.0],
            y_bin_edges: vec![-1.0, 0.0, 1.0],
            bin_contents: vec![1.0, 2.0, 3.0, 4.0],
            nx: 2,
            ny: 2,
            entries: 10,
            total_weight: 10.0,
        };
        let json = serde_json::to_string(&data).unwrap();
        let back: Histogram2DData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "Test 2D");
        assert_eq!(back.nx, 2);
        assert_eq!(back.ny, 2);
        assert_eq!(back.bin_contents.len(), 4);
        assert_eq!(back.entries, 10);
    }

    #[test]
    fn plot_definition_2d_serde() {
        let plot = PlotDefinition2D {
            name: "pT vs Î·".into(),
            x_observable_script: "event.momenta[0].pt()".into(),
            y_observable_script: "event.momenta[0].eta()".into(),
            nx: 20,
            x_min: 0.0,
            x_max: 100.0,
            ny: 25,
            y_min: -5.0,
            y_max: 5.0,
        };
        let json = serde_json::to_string(&plot).unwrap();
        let back: PlotDefinition2D = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "pT vs Î·");
        assert_eq!(back.nx, 20);
        assert_eq!(back.ny, 25);
    }

    #[test]
    fn run_analysis_with_2d_plots() {
        use crate::kinematics::RamboGenerator;

        let config = AnalysisConfig {
            plots: vec![PlotDefinition {
                name: "pT".into(),
                observable_script: "event.momenta[0].pt()".into(),
                n_bins: 10,
                min: 0.0,
                max: 60.0,
            }],
            cut_scripts: vec![],
            num_events: 500,
            cms_energy: 100.0,
            final_masses: vec![0.0, 0.0],
            detector_preset: None,
            particle_kinds: None,
            plots_2d: Some(vec![PlotDefinition2D {
                name: "pT vs Î·".into(),
                x_observable_script: "event.momenta[0].pt()".into(),
                y_observable_script: "event.momenta[0].eta()".into(),
                nx: 10,
                x_min: 0.0,
                x_max: 60.0,
                ny: 10,
                y_min: -5.0,
                y_max: 5.0,
            }]),
        };

        let mut gen = RamboGenerator::new();
        let result = run_analysis(&config, |_| 1.0, &mut gen).unwrap();

        assert_eq!(result.histograms.len(), 1);
        assert_eq!(result.histograms_2d.len(), 1);

        let h2 = &result.histograms_2d[0];
        assert_eq!(h2.name, "pT vs Î·");
        assert_eq!(h2.nx, 10);
        assert_eq!(h2.ny, 10);
        assert_eq!(h2.bin_contents.len(), 100);
        assert!(h2.entries > 0);
    }

    #[test]
    fn run_reco_analysis_with_2d_plots() {
        use crate::kinematics::RamboGenerator;

        let config = AnalysisConfig {
            plots: vec![PlotDefinition {
                name: "N jets".into(),
                observable_script: "reco.n_jets().to_float()".into(),
                n_bins: 10,
                min: 0.0,
                max: 10.0,
            }],
            cut_scripts: vec![],
            num_events: 200,
            cms_energy: 200.0,
            final_masses: vec![0.0, 0.0],
            detector_preset: Some("perfect".into()),
            particle_kinds: Some(vec!["hadron".into(), "hadron".into()]),
            plots_2d: Some(vec![PlotDefinition2D {
                name: "pT vs Î· (reco)".into(),
                x_observable_script: "event.momenta[0].pt()".into(),
                y_observable_script: "event.momenta[0].eta()".into(),
                nx: 8,
                x_min: 0.0,
                x_max: 120.0,
                ny: 8,
                y_min: -5.0,
                y_max: 5.0,
            }]),
        };

        let mut gen = RamboGenerator::new();
        let result = run_reco_analysis(&config, |_| 1.0, &mut gen).unwrap();

        assert_eq!(result.histograms.len(), 1);
        assert_eq!(result.histograms_2d.len(), 1);
        assert!(result.histograms_2d[0].entries > 0);
    }

    #[test]
    fn analysis_config_2d_serde_roundtrip() {
        let config = AnalysisConfig {
            plots: vec![],
            cut_scripts: vec![],
            num_events: 100,
            cms_energy: 100.0,
            final_masses: vec![0.0, 0.0],
            detector_preset: None,
            particle_kinds: None,
            plots_2d: Some(vec![PlotDefinition2D {
                name: "test 2d".into(),
                x_observable_script: "event.momenta[0].pt()".into(),
                y_observable_script: "event.momenta[0].eta()".into(),
                nx: 10,
                x_min: 0.0,
                x_max: 100.0,
                ny: 10,
                y_min: -5.0,
                y_max: 5.0,
            }]),
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: AnalysisConfig = serde_json::from_str(&json).unwrap();
        assert!(back.plots_2d.is_some());
        assert_eq!(back.plots_2d.unwrap().len(), 1);
    }

    #[test]
    fn analysis_result_with_2d_serde() {
        let result = AnalysisResult {
            histograms: vec![],
            histograms_2d: vec![Histogram2DData {
                name: "test".into(),
                x_bin_edges: vec![0.0, 1.0],
                y_bin_edges: vec![0.0, 1.0],
                bin_contents: vec![5.0],
                nx: 1,
                ny: 1,
                entries: 5,
                total_weight: 5.0,
            }],
            cross_section: 1.0e-6,
            cross_section_error: 1.0e-8,
            events_generated: 100,
            events_passed: 100,
            profile: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: AnalysisResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.histograms_2d.len(), 1);
        assert_eq!(back.histograms_2d[0].name, "test");
    }
}
