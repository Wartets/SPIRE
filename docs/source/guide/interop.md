# Interoperability & Export

SPIRE is designed to interoperate with the broader high-energy physics
software ecosystem. This guide covers two key integration pathways:

1. **LHE Event Export** - writing generated events to the standard
   Les Houches Event format for downstream processing.
2. **Python Library (`spire-hep`)** - using SPIRE's physics engine
   directly from Python with zero-copy NumPy array access.

---

## LHE Event Export

### Background

The **Les Houches Event** (LHE) format is the de facto standard for
exchanging parton-level Monte Carlo events between generators and
detector simulation tools. SPIRE implements version 3.0 of the format
as specified in [hep-ph/0609017](https://arxiv.org/abs/hep-ph/0609017).

### Architecture

The export system is built around a trait-based abstraction:

```
EventWriter (trait)
└── LheWriter<W: Write>    ← concrete LHE v3.0 implementation
```

The `EventWriter` trait defines three operations:

| Method | Description |
|---|---|
| `write_header(config)` | Write the file preamble (beam parameters, process list) |
| `write_event(event)` | Write a single event record |
| `finish()` | Flush buffers and close the document |

This design allows future format implementations (HepMC3, ROOT TTree)
to share the same data structures.

### Data Structures

Events are described by format-agnostic intermediate types:

- **`EventParticle`** - a single particle entry with PDG ID, status code
  ($-1$ = initial, $+1$ = final, $+2$ = intermediate), mother indices,
  colour flow tags, 4-momentum $(p_x, p_y, p_z, E, m)$, proper lifetime,
  and spin information.
- **`EventRecord`** - a complete event: list of particles, event weight,
  factorisation scale, $\alpha_{\text{QED}}$, $\alpha_{\text{QCD}}$, and
  process identifier.
- **`InitConfig`** - beam configuration, PDF metadata, and a list of
  `ProcessInfo` entries describing each subprocess (cross-section, error,
  maximum weight).

### Usage (Rust)

```rust
use spire_kernel::io::{EventWriter, InitConfig, ProcessInfo};
use spire_kernel::io::lhe::{LheWriter, event_record_from_phase_space};
use spire_kernel::kinematics::RamboGenerator;

// Create an LHE file
let file = std::fs::File::create("output.lhe").unwrap();
let mut writer = LheWriter::new(file);

// Write the header
let config = InitConfig {
    beam_id: [2212, 2212],          // proton–proton
    beam_energy: [6500.0, 6500.0],  // 13 TeV
    pdf_group: [0, 0],
    pdf_set: [0, 0],
    weight_strategy: 3,
    processes: vec![ProcessInfo {
        cross_section: 1.234e-3,
        error: 5.6e-5,
        max_weight: 1.0,
        process_id: 1,
    }],
    generator_name: Some("SPIRE 0.1.0".into()),
};
writer.write_header(&config).unwrap();

// Generate and write events
let mut gen = RamboGenerator::with_seed(42);
let final_masses = vec![0.10566, 0.10566]; // muon pair

for _ in 0..1000 {
    let ps = gen.generate_event(91.2, &final_masses).unwrap();
    let pdg_ids = vec![13, -13]; // μ⁻ μ⁺
    let record = event_record_from_phase_space(
        &ps, &pdg_ids, &[2212, 2212], 6500.0, 1,
    );
    writer.write_event(&record).unwrap();
}

writer.finish().unwrap();
```

### Performance

`LheWriter` wraps the output stream in a `BufWriter` (default 64 KB
buffer) to minimise system calls in the write-intensive event loop.
For custom buffer sizes, use `LheWriter::with_capacity(size, writer)`.

---

## Python Library: `spire-hep`

### Installation

SPIRE's Python bindings are distributed as a native extension module
built with [PyO3](https://pyo3.rs) and [Maturin](https://maturin.rs).

```bash
# Development install (requires Rust toolchain)
pip install maturin
cd /path/to/SPIRE
maturin develop --features python,extension-module

# Or install from a built wheel
pip install spire_hep-0.1.0-cp39-*.whl
```

### Module Overview

```python
import spire_hep
```

The module exposes the following classes and functions:

#### Classes

| Class | Description |
|---|---|
| `SpacetimeVector` | 4-vector with Minkowski inner product |
| `Complex` | Complex number with arithmetic operations |
| `IntegrationResult` | Monte Carlo integration output |
| `CrossSectionResult` | Partonic cross-section computation |
| `HadronicCrossSectionResult` | PDF-convoluted hadronic cross-section |

#### Functions

| Function | Description |
|---|---|
| `load_model(particles, vertices, name)` | Parse TOML and build a model (returns JSON) |
| `construct_reaction(initial, final, energy, model)` | Validate a scattering process |
| `reconstruct_reaction(initial, energy, model)` | Enumerate allowed final states |
| `generate_diagrams(reaction, model, max_loops)` | Generate Feynman diagrams |
| `compute_amplitude(diagram)` | Derive symbolic amplitude |
| `calculate_threshold(masses, target_mass)` | Compute reaction threshold |
| `is_kinematically_allowed(energy, masses)` | Check kinematic feasibility |
| `calculate_cross_section(energy, masses, n_events)` | Partonic MC cross-section |
| `calculate_hadronic_cross_section(s, masses, n_events)` | Hadronic MC cross-section |
| `generate_phase_space_events(energy, masses, n, seed)` | Generate RAMBO events → NumPy |

### NumPy Zero-Copy Event Generation

The `generate_phase_space_events` function returns phase-space events
directly as NumPy arrays with **zero intermediate copies**:

```python
import spire_hep
import numpy as np

# Generate 100,000 four-body events at √s = 91.2 GeV
momenta, weights = spire_hep.generate_phase_space_events(
    cms_energy=91.2,
    final_masses=[0.10566, 0.10566, 0.0, 0.0],
    num_events=100_000,
    seed=42,       # optional: deterministic seed
)

print(momenta.shape)   # (100000, 4, 4) - [event, particle, component]
print(weights.shape)   # (100000,)
print(momenta.dtype)   # float64
```

The `momenta` array has shape `(N_events, N_particles, 4)` where the
last axis is $(E, p_x, p_y, p_z)$. Both arrays are C-contiguous
`float64`, allocated directly in NumPy memory via `PyArray` - no
intermediate Rust `Vec` is created.

### Cross-Section Computation

```python
import spire_hep

# Partonic cross-section: e⁺e⁻ → μ⁺μ⁻ at √s = 91.2 GeV
result = spire_hep.calculate_cross_section(
    cms_energy=91.2,
    final_masses=[0.10566, 0.10566],
    num_events=50_000,
)
print(f"σ = {result.cross_section_pb:.4f} ± {result.uncertainty_pb:.4f} pb")
print(f"Events: {result.events_evaluated}, ε = {result.efficiency:.2%}")

# Hadronic cross-section: pp → μ⁺μ⁻ at √s = 13 TeV
had = spire_hep.calculate_hadronic_cross_section(
    beam_energy_sq=13000.0**2,
    final_masses=[0.10566, 0.10566],
    num_events=100_000,
)
print(f"σ_had = {had.cross_section_pb:.6f} ± {had.uncertainty_pb:.6f} pb")
print(f"PDF: {had.pdf_name}")
```

### Data Analysis with NumPy

A typical analysis workflow combining event generation with NumPy:

```python
import spire_hep
import numpy as np

# Generate events
momenta, weights = spire_hep.generate_phase_space_events(
    cms_energy=91.2,
    final_masses=[0.10566, 0.10566],
    num_events=200_000,
    seed=12345,
)

# Extract leading muon kinematics (particle index 0)
E  = momenta[:, 0, 0]
px = momenta[:, 0, 1]
py = momenta[:, 0, 2]
pz = momenta[:, 0, 3]

# Compute transverse momentum
pt = np.sqrt(px**2 + py**2)

# Compute pseudorapidity
p_mag = np.sqrt(px**2 + py**2 + pz**2)
eta = np.arctanh(np.clip(pz / p_mag, -0.9999, 0.9999))

# Compute di-muon invariant mass
p_total = momenta[:, 0, :] + momenta[:, 1, :]
m_inv = np.sqrt(np.abs(p_total[:, 0]**2
                        - p_total[:, 1]**2
                        - p_total[:, 2]**2
                        - p_total[:, 3]**2))

print(f"Mean pT:  {np.average(pt, weights=weights):.3f} GeV")
print(f"Mean |η|: {np.average(np.abs(eta), weights=weights):.3f}")
print(f"Mean M:   {np.average(m_inv, weights=weights):.3f} GeV")
```

### Histogram Example with Matplotlib

```python
import matplotlib.pyplot as plt
import numpy as np
import spire_hep

momenta, weights = spire_hep.generate_phase_space_events(
    cms_energy=91.2,
    final_masses=[0.10566, 0.10566],
    num_events=100_000,
)

p_total = momenta[:, 0, :] + momenta[:, 1, :]
m_inv = np.sqrt(np.abs(p_total[:, 0]**2
                        - p_total[:, 1]**2
                        - p_total[:, 2]**2
                        - p_total[:, 3]**2))

fig, ax = plt.subplots()
ax.hist(m_inv, bins=100, weights=weights, histtype='step', color='navy')
ax.set_xlabel(r'$M_{\mu\mu}$ [GeV]')
ax.set_ylabel('Weighted events')
ax.set_title(r'Di-muon invariant mass at $\sqrt{s} = 91.2$ GeV')
plt.tight_layout()
plt.savefig('m_inv.pdf')
```

---

## External Solver Interface

SPIRE provides a generic `ExternalSolver` trait for communicating with
external analytical tools (IBP reduction engines, symbolic solvers, etc.)
via subprocess invocation.

### `CliSolver`

The built-in `CliSolver` wraps `std::process::Command` with a builder
API:

```rust
use spire_kernel::interface::CliSolver;

let solver = CliSolver::new("fire")
    .args(&["-auto"])
    .timeout_secs(120)
    .working_dir("/tmp/fire_workspace")
    .env("FIRE_THREADS", "4");

let output = solver.solve("integrate[topology1]")?;
```

The solver spawns a child process, pipes the input string to `stdin`,
captures `stdout`, and checks the exit code. Non-zero exit codes are
mapped to `SpireError::InternalError` with the captured `stderr`
content.

### Implementing Custom Solvers

Implement the `ExternalSolver` trait for non-CLI backends (e.g., socket
connections, shared-memory IPC, or REST APIs):

```rust
use spire_kernel::interface::ExternalSolver;
use spire_kernel::SpireResult;

struct HttpSolver { endpoint: String }

impl ExternalSolver for HttpSolver {
    fn name(&self) -> &str { "http-solver" }

    fn solve(&self, input: &str) -> SpireResult<String> {
        // POST to the endpoint and return the response body
        todo!()
    }
}
```

---

*See also: [Scripting Guide](scripting.md) · [API Reference](../api/reference.md)*
