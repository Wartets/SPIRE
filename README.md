<p align="center">
  <strong>S P I R E</strong><br>
  <em>Structured Particle Interaction and Reaction Engine</em>
</p>

<p align="center">
  A rigorous computational framework for High Energy Physics and Quantum Field Theory —<br>
  from Lagrangian densities to publication-ready cross-sections, entirely from first principles.
</p>

<p align="center">
  <a href="https://github.com/Wartets/SPIRE">Repository</a> ·
  <a href="#getting-started">Getting Started</a> ·
  <a href="#usage-walkthrough">Usage</a> ·
  <a href="#theoretical-formalism">Formalism</a> ·
  <a href="#roadmap--philosophy">Roadmap</a>
</p>

---

## Abstract

**SPIRE** is a deductive physics engine and computational formalism designed for the automated analysis of particle interactions within Quantum Field Theory (QFT). Unlike conventional tools that rely on pre-calculated look-up tables or hard-coded process libraries, SPIRE constructs valid physical processes *ab initio* — from user-specified Lagrangian densities, symmetry groups, and conservation laws. Its core design enforces a strict **isomorphism** between the three canonical representations of a scattering process:

1. The **symbolic** reaction equation ($A + B \to C + D$),
2. The **topological** structure (Feynman diagrams),
3. The **analytical** expression (the invariant amplitude $\mathcal{M}$).

A single unified data structure carries all three representations simultaneously, ensuring that no information is lost or desynchronised across the derivation chain. SPIRE provides a complete computational pipeline: Lagrangian parsing → vertex rule extraction → Feynman topology enumeration → symbolic amplitude construction → Dirac/Lorentz algebra simplification → Monte Carlo phase-space integration → histogram analysis → detector-level reconstruction — all within a single, memory-safe Rust kernel exposed through an interactive SvelteKit desktop application.

All project source code, documentation, and data files are hosted at [github.com/Wartets/SPIRE](https://github.com/Wartets/SPIRE).

---

## Table of Contents

- [Core Capabilities](#core-capabilities)
- [System Architecture](#system-architecture)
- [Getting Started](#getting-started)
- [Usage Walkthrough](#usage-walkthrough)
- [Repository Structure](#repository-structure)
- [Theoretical Formalism](#theoretical-formalism)
- [Roadmap & Philosophy](#roadmap--philosophy)
- [Contributing](#contributing)
- [License](#license)
- [Citation](#citation)

---

## Core Capabilities

### Deductive Physics Engine
SPIRE derives physically valid reactions from fundamental axioms rather than enumerating them from a static database. Given an initial state and centre-of-mass energy $\sqrt{s}$, the engine:
- Validates **all** conservation laws mandated by the active gauge symmetry: electric charge, baryon number, lepton family numbers ($L_e, L_\mu, L_\tau$), weak isospin $T_3$, hypercharge $Y$, and colour.
- Enforces discrete symmetry selection rules ($C$, $P$, $T$) appropriate to the interaction type.
- Reconstructs all kinematically and dynamically allowed final states.
- Identifies mediating gauge bosons and determines the perturbative order.

### Lagrangian Workbench & Automated Feynman Rule Derivation
Physicists define theories at the Lagrangian level using a human-readable syntax:
```
e * psi_bar * gamma^mu * psi * A_mu
```
SPIRE parses this into a typed Abstract Syntax Tree (AST) with 12 node types (`FieldOp`, `GammaMat`, `CovariantDerivative`, `FieldStrength`, etc.), then:
- **Derives vertex rules** via successive functional differentiation with respect to external fields, implementing the full Leibniz rule with Grassmann sign tracking for fermionic operators.
- **Validates theoretical consistency**: Lorentz invariance (all spacetime indices contracted), gauge singlet (U(1) charge conservation, SU(N) $N$-ality), Hermiticity (fermion bilinear pairing), and renormalisability (operator mass dimension $\leq 4$).
- **Computes RGE flows** via 4th-order Runge–Kutta integration of user-defined $\beta$-functions scripted in the embedded Rhai language.

### Computer Algebra System (CAS) for Spacetime Algebra
A dedicated symbolic engine handles the full Dirac–Lorentz algebra:
- **Trace evaluation**: $\text{Tr}[\gamma^\mu \gamma^\nu \gamma^\rho \gamma^\sigma] = 4(g^{\mu\nu}g^{\rho\sigma} - g^{\mu\rho}g^{\nu\sigma} + g^{\mu\sigma}g^{\nu\rho})$ with $d$-dimensional extensions for loop calculations.
- **Index contraction**: Automated Einstein summation across all Lorentz and gauge indices.
- **Spinor algebra**: Dirac equation simplification ($\bar{u}\slashed{p} = \bar{u}m$), completeness relations, and spin-sum evaluations.
- **Dimensional regularisation**: Full $d = 4 - 2\varepsilon$ algebra for one-loop calculations with Passarino–Veltman tensor reduction to scalar master integrals ($A_0$, $B_0$, $C_0$, $D_0$).

### Monte Carlo Integration & Event Generation
- **RAMBO algorithm**: Flat $N$-body Lorentz-Invariant Phase Space (LIPS) generation with exact energy-momentum conservation.
- **Cross-section computation**: $\sigma = \frac{1}{2s} \int d\Phi_n \, |\mathcal{M}|^2$ via Monte Carlo sampling with variance estimation.
- **Hadronic convolution**: Parton-level cross-sections folded with Parton Distribution Functions:

$$\sigma_{pp} = \sum_{a,b} \int dx_1 \, dx_2 \; f_a(x_1, Q^2) \, f_b(x_2, Q^2) \; \hat{\sigma}_{ab}(\hat{s})$$

- **Histogramming engine**: 1D and 2D histograms with bin-level variance estimation, underflow/overflow tracking, and statistical merging.
- **Distributed compute grid**: Phase-space integration partitioned across Web Worker threads with fault-tolerant scheduling and $1/\sqrt{N}$ convergence monitoring.

### Phenomenological Detector Simulation
- **Parametric detector models**: Configurable efficiency maps, Gaussian energy smearing, and $\eta$-acceptance cuts.
- **Anti-$k_t$ jet clustering**: E-scheme recombination with configurable jet radius $R$.
- **Reconstructed event model**: Jets, isolated leptons ($e$, $\mu$), photons, and missing transverse energy $E_T^{\text{miss}}$.
- **Three presets**: Perfect (unity efficiency), LHC-like (ATLAS/CMS parametrisation), ILC-like ($e^+e^-$ collider).

### External Theory Bridge (SLHA & UFO)
- **SLHA parser**: Full SUSY Les Houches Accord v1 ingestion — mass spectra (`BLOCK MASS`), mixing matrices (`BLOCK NMIX`, `BLOCK UMIX`), decay tables (`DECAY`), and running parameters with $Q$-scale tracking.
- **UFO bridge**: Pure-Rust parser for Universal FeynRules Output models (WASM-compatible, no Python dependency). Reads `particles.py`, `vertices.py`, `couplings.py`, `parameters.py`, and `lorentz.py`, mapping them into SPIRE's ontology.
- **NLO counterterm generator**: Automated multiplicative field renormalisation $\phi_0 = \sqrt{Z}\phi_R$ with extraction of all $\mathcal{O}(\delta)$ counterterm vertices and their Feynman rules.

### Visualisation & Interactive Analysis
- **Feynman diagram renderer**: Vector-graphic topology visualisation with channel ($s$, $t$, $u$) colour coding.
- **Interactive diagram editor**: SVG-based drag-and-drop vertex manipulation with force-directed layout.
- **Dalitz plots**: Phase-space boundary computation and point generation for three-body decays.
- **3D event display**: Three.js-powered detector visualisation with energy-scaled jet cones, lepton tracks, photon lines, and $E_T^{\text{miss}}$ arrows, with event-by-event animation.
- **GPU-accelerated heatmaps**: WebGL-based 2D correlation plots with Viridis colourmap.

### Extensibility & Interoperability
- **Custom model files**: TOML-based particle and vertex definitions with referential integrity validation.
- **BSM / EFT support**: Arbitrary gauge groups ($SU(N)$, $SO(N)$, $U(1)'$), higher-dimensional operators (dimension-5, -6, -8), and variable spacetime dimensionality.
- **LaTeX export**: Publication-quality amplitude expressions.
- **UFO export**: Model export to the Universal FeynRules Output format for MadGraph5_aMC@NLO compatibility.
- **LHE output**: Les Houches Event file generation for interfacing with shower Monte Carlo generators.
- **Rhai scripting**: User-defined observable and kinematic cut functions evaluated at runtime.

---

## System Architecture

SPIRE follows a strict three-layer architecture that enforces separation of concerns and enables multi-platform deployment:

```
┌──────────────────────────────────────────────────────────┐
│               SvelteKit Adaptive Workbench               │
│      13 widget types · Chart.js · Three.js · WebGL       │
│      Compute Grid Dashboard · Workspace persistence      │
├──────────────────────────────────────────────────────────┤
│              Tauri IPC / WASM Boundary Layer             │
│   22 IPC commands · serde-wasm-bindgen · PyO3 bindings   │
├──────────────────────────────────────────────────────────┤
│                   Rust Physics Kernel                    │
│    Ontology · S-Matrix · Graph · Algebra · Kinematics    │
│    Analysis · PDF · Integration · Theory · Scripting     │
│          ~15,000 lines · 764 tests · zero unsafe         │
└──────────────────────────────────────────────────────────┘
```

### The Rust Kernel (`spire-kernel`)
The headless computational core contains all physics logic. It is a pure Rust library with no runtime dependencies on the UI layer, enabling it to be compiled to WebAssembly, linked into Python via PyO3, or used as a standalone CLI tool. Key modules:

| Module | Responsibility |
|--------|---------------|
| `ontology` | Quantum field definitions, state vectors, Poincaré representations |
| `groups` | Lie algebra engine, gauge symmetry validation, conservation laws |
| `s_matrix` | Reaction construction, final-state reconstruction, cross-section integration |
| `lagrangian` | Theoretical model assembly, vertex factors, propagators |
| `graph` | Feynman topology generation, channel classification, symmetry factors |
| `algebra` | CAS engine — Dirac algebra, trace evaluation, amplitude derivation |
| `kinematics` | Mandelstam variables, RAMBO phase-space, Dalitz boundaries, Lorentz boosts |
| `analysis` | Histogram engine, observable scripting, detector simulation, jet clustering |
| `theory` | Lagrangian AST, functional differentiation, validation, RGE solver, SLHA/UFO parsers, NLO counterterms |
| `integration` | Hadronic cross-section convolution, PDF providers |
| `scripting` | Rhai script compilation and evaluation for user-defined observables |

### The Bindings Layer (`spire-bindings`)
Exposes the kernel through stable Foreign Function Interfaces:
- **WebAssembly** (`wasm-bindgen`): Full kernel access in the browser via `serde-wasm-bindgen` for zero-copy JS object interchange.
- **Python** (`PyO3`, feature-gated): Extension module for integration with NumPy, SciPy, and Jupyter.

### The Desktop Application (`spire-desktop`)
A **Tauri** + **SvelteKit** application providing a reactive scientific workbench:
- **22 Tauri IPC commands** bridging the frontend to the Rust backend via JSON-serialised, stateless function calls.
- **13 interactive widget types** arranged on a CSS-Grid canvas: Model Loader, Reaction Workspace, Diagram Visualiser, Amplitude Panel, Kinematics View, Dalitz Plotter, Analysis Widget, Event Display, Diagram Editor, Lagrangian Workbench, External Models, Compute Grid Dashboard, and Console.
- **Workspace persistence**: Layouts serialisable to JSON with auto-save and LocalStorage restore.

---

## Getting Started

### Prerequisites
- **Rust** (edition 2021+) and Cargo — [rustup.rs](https://rustup.rs)
- **Node.js** (LTS, ≥ 18) and npm
- **System libraries** (Linux only): `build-essential`, `libwebkit2gtk-4.0-dev`, `libgtk-3-dev`, `libssl-dev`
- **System libraries** (macOS): Xcode Command Line Tools

### Build & Run

```bash
# 1. Clone the repository
git clone https://github.com/Wartets/SPIRE.git
cd SPIRE

# 2. Install frontend dependencies
npm install

# 3. Run in development mode (with hot-reloading)
npm run tauri dev

# 4. Or build a release-optimised binary
npm run tauri build
# → Output: apps/spire-desktop/src-tauri/target/release/
```

### Running the Test Suite
```bash
# Run all 764+ kernel tests (unit, fuzz, property-based, doc-tests)
cargo test --workspace

# Run benchmarks (requires nightly for some features)
cargo bench -p spire-kernel
```

---

## Usage Walkthrough

### 1. Defining a Lagrangian and Deriving Vertex Rules

Open the **Lagrangian Workbench** widget. Define your field content (e.g., `psi` as Fermion, `A` as Vector), then type a Lagrangian interaction term:

```
e * psi_bar * gamma^mu * psi * A_mu
```

SPIRE will:
- Parse the term into an AST and render its LaTeX form: $e\bar{\psi}\gamma^\mu\psi A_\mu$.
- Display validation badges: **Lorentz ✓** (all indices contracted), **Gauge ✓** (charge-neutral), **Hermitian ✓**, **Renormalisable ✓** (dimension 4).
- Derive the QED vertex rule: select external fields $\bar{\psi}$, $\psi$, $A^\mu$ and click "Derive Vertex Rule" to obtain $-ie\gamma^\mu$.

### 2. Running a Monte Carlo Analysis with Custom Observables

Open the **Analysis** widget. Configure:
- **CMS Energy**: 91.2 GeV (Z-pole)
- **Final masses**: [0.000511, 0.000511] (electron pair)
- **Events**: 100,000

Define a custom observable script (Rhai syntax):
```rust
// Invariant mass of the final-state pair
let p1 = event.momenta[0];
let p2 = event.momenta[1];
let s = (p1.e + p2.e).powi(2) - (p1.px + p2.px).powi(2)
      - (p1.py + p2.py).powi(2) - (p1.pz + p2.pz).powi(2);
s.sqrt()
```

Click **Run Analysis** to generate events, fill histograms, and display the $m_{ee}$ distribution with cross-section diagnostics. For large-scale integrations (millions of events), route the computation through the **Compute Grid Dashboard** to distribute work across all available CPU cores via Web Workers.

### 3. Importing a UFO Model

Open the **External Models** widget → **UFO** tab. Upload the `.py` files from a FeynRules/SARAH UFO output directory (e.g., `particles.py`, `vertices.py`, `couplings.py`, `parameters.py`). SPIRE's pure-Rust parser reads the Python constructor calls, maps UFO spin conventions ($2S+1$) to SPIRE's $2S$ representation, converts colour codes, and assembles a complete `TheoreticalModel`. Click **Apply as Active Model** to use it in all subsequent calculations.

### 4. Inspecting SLHA Spectra

Open the **External Models** widget → **SLHA** tab. Upload or paste an SLHA file from SPheno, SoftSUSY, or any compatible spectrum generator. The block browser displays mass spectra (`BLOCK MASS`), mixing matrices (`BLOCK NMIX`), and Higgs sector parameters. Decay tables show branching ratios and daughter particles.

### 5. NLO Counterterm Generation

In the **External Models** widget → **NLO** tab, enter a tree-level term (e.g., `e * psi_bar * gamma^mu * psi * A_mu`), define the field content and external legs, then click **Derive Counterterms**. SPIRE performs multiplicative field renormalisation ($\phi \to \sqrt{Z}\phi$), coupling renormalisation ($g \to g + \delta g$), extracts all $\mathcal{O}(\delta)$ counterterm expressions, and derives their Feynman rules.

### 6. Using the Compute Grid

Open the **Compute Grid Dashboard** widget. Configure the total number of events and optional chunk size, then launch a distributed integration. The dashboard displays:
- **Active nodes** (Web Workers) with real-time status indicators (Idle / Computing / Merging).
- **Global progress bar** with events completed and estimated time remaining.
- **Convergence plot** showing the cross-section uncertainty decreasing as $1/\sqrt{N}$.
- **Merged result** with statistically combined histograms upon completion.

---

## Repository Structure

```
SPIRE/
├── crates/
│   ├── spire-kernel/                   Rust physics engine
│   │   ├── src/
│   │   │   ├── algebra.rs              CAS engine, Dirac algebra, amplitude derivation
│   │   │   ├── analysis.rs             Histogramming, detector simulation, jet clustering
│   │   │   ├── data_loader.rs          TOML model ingestion
│   │   │   ├── graph.rs                Feynman topology generation
│   │   │   ├── groups.rs               Lie algebra, gauge symmetries, conservation laws
│   │   │   ├── integration.rs          Hadronic cross-section convolution
│   │   │   ├── kinematics.rs           Phase space, Mandelstam, Dalitz, RAMBO
│   │   │   ├── lagrangian.rs           TheoreticalModel, vertex factors, propagators
│   │   │   ├── ontology.rs             Quantum fields, particles, state vectors
│   │   │   ├── pdf.rs                  Parton distribution functions
│   │   │   ├── s_matrix.rs             Reactions, cross-sections, event generation
│   │   │   ├── scripting.rs            Rhai observable/cut script engine
│   │   │   └── theory/                 Lagrangian AST, derivation, validation, RGE,
│   │   │       ├── ast.rs              SLHA parser, UFO bridge, NLO counterterms
│   │   │       ├── derivation.rs
│   │   │       ├── renormalization.rs
│   │   │       ├── rge.rs
│   │   │       ├── slha.rs
│   │   │       ├── ufo.rs
│   │   │       └── validation.rs
│   │   ├── tests/                      Property-based & fuzz tests
│   │   └── benches/                    Criterion benchmarks
│   └── spire-bindings/                 WASM (wasm-bindgen) & Python (PyO3) FFI
├── apps/
│   └── spire-desktop/                  Tauri + SvelteKit desktop application
│       ├── src/
│       │   ├── lib/
│       │   │   ├── api.ts              Typed Tauri IPC wrappers
│       │   │   ├── components/         13 interactive widget components
│       │   │   ├── services/           Compute grid, workspace manager
│       │   │   ├── stores/             Svelte stores (physics, notebook, inputs)
│       │   │   ├── types/              TypeScript type definitions
│       │   │   └── workers/            Web Worker compute nodes
│       │   └── routes/
│       └── src-tauri/
│           └── src/main.rs             22 Tauri IPC commands
├── data/                               Default SM model files (TOML)
├── docs/                               MkDocs documentation source
├── README.md                           This file
├── roadmap.md                          Master development roadmap (34 phases)
└── Cargo.toml                          Workspace manifest
```

---

## Theoretical Formalism

SPIRE's computational architecture is grounded in the operator formalism of Quantum Field Theory and the LSZ reduction theorem. The central objects are:

### S-Matrix and Asymptotic States
Physical processes are transitions between asymptotic in- and out-states in Fock space. A scattering amplitude is computed as:

$$\langle f | S | i \rangle = \langle f | T \exp\!\left(-i \int d^4x \, \mathcal{H}_{\text{int}}(x)\right) | i \rangle$$

where $\mathcal{H}_{\text{int}}$ is the interaction Hamiltonian density derived from the user-specified Lagrangian $\mathcal{L}_{\text{int}}$.

### Quantum Ontology
Particles are classified as irreducible representations of the Poincaré group $\text{ISO}(1,3)$ (mass $m$ and spin $s$) tensored with representations of internal gauge groups. Each field carries a complete set of quantum numbers validated against the Gell-Mann–Nishijima relation:

$$Q = T_3 + \frac{Y}{2}$$

### Lagrangian Dynamics
Interaction vertices are extracted from $\mathcal{L}_{\text{int}}$ by functional differentiation:

$$V_{\phi_1 \cdots \phi_n}(p_1, \ldots, p_n) = \frac{\delta^n \mathcal{L}}{\delta\phi_1(p_1) \cdots \delta\phi_n(p_n)}$$

with Fourier-convention $\partial_\mu \to -ip_\mu$ for external leg momentum replacement.

### Phase-Space Integration
Total cross-sections are computed via Monte Carlo integration over Lorentz-Invariant Phase Space:

$$\sigma = \frac{1}{2s} \int d\Phi_n \, |\mathcal{M}|^2, \qquad d\Phi_n = (2\pi)^4 \delta^{(4)}\!\left(P - \sum_i p_i\right) \prod_{i=1}^{n} \frac{d^3 p_i}{(2\pi)^3 \, 2E_i}$$

The RAMBO algorithm generates unweighted $n$-body phase-space points with exact four-momentum conservation.

### Renormalisation
NLO counterterms are generated via multiplicative field renormalisation:

$$\phi_0 = \sqrt{Z_\phi}\,\phi_R \approx \left(1 + \tfrac{1}{2}\delta Z_\phi\right)\phi_R, \qquad g_0 = g_R + \delta g$$

and extraction of all terms linear in the renormalisation constants $\delta Z$, $\delta m$, $\delta g$.

---

## Contributing

Contributions are welcome. Please refer to the project's issue tracker for areas of active development. For major changes, please open an issue first to discuss the proposed modification. All submitted code should maintain the existing test coverage (currently 764+ tests) and compile without warnings.

---

## License

This project is licensed under the MIT License. See the [LICENSE](https://github.com/Wartets/SPIRE/blob/main/LICENSE) file for details.
