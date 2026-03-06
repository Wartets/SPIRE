<p align="center">
  <strong>S P I R E</strong><br>
  <em>Structured Particle Interaction and Reaction Engine</em>
</p>

<p align="center">
  A full-stack computational phenomenology framework for High Energy Physics,<br>
  from Lagrangian densities to publication-ready cross-sections, entirely from first principles.
</p>

<p align="center">
  <a href="https://github.com/Wartets/SPIRE">Repository</a> 
  <a href="#getting-started">Getting Started</a> 
  <a href="#usage-walkthrough">Usage</a> 
  <a href="#theoretical-formalism">Formalism</a> 
  <a href="#roadmap--philosophy">Roadmap</a>
</p>

---

## Abstract

**SPIRE** is a deductive physics engine and computational formalism designed for the automated analysis of particle interactions within Quantum Field Theory (QFT). Unlike conventional tools that rely on pre-calculated look-up tables or hard-coded process libraries, SPIRE constructs valid physical processes *ab initio*,  from user-specified Lagrangian densities, symmetry groups, and conservation laws. Its core design enforces a strict **isomorphism** between the three canonical representations of a scattering process:

1. The **symbolic** reaction equation ($A + B \to C + D$),
2. The **topological** structure (Feynman diagrams),
3. The **analytical** expression (the invariant amplitude $\mathcal{M}$).

A single unified data structure carries all three representations simultaneously, ensuring that no information is lost or desynchronised across the derivation chain. SPIRE provides a complete computational pipeline: Lagrangian parsing → vertex rule extraction → Feynman topology enumeration → symbolic amplitude construction → Dirac/Lorentz algebra simplification → Monte Carlo phase-space integration → histogram analysis → detector-level reconstruction — all within a single, memory-safe Rust kernel exposed through an interactive SvelteKit desktop application.

The kernel consists of **25 modules** totalling approximately **37,000 lines** of pure Rust (zero `unsafe`), validated by a comprehensive test suite of **940+ tests** (unit, property-based, fuzz, integration, and doc-tests). The desktop application exposes **37 Tauri IPC commands** driving **19 interactive widget types** on a dockable/infinite-canvas workspace with recursive split panes and multi-workspace tab management.

All project source code, documentation, and data files are hosted at [github.com/Wartets/SPIRE](https://github.com/Wartets/SPIRE).

---

## Table of Contents

- [Core Capabilities](#core-capabilities)
- [System Architecture](#system-architecture)
- [Getting Started](#getting-started)
- [Usage Walkthrough](#usage-walkthrough)
- [Data Provenance & Reproducibility](#data-provenance--reproducibility)
- [Repository Structure](#repository-structure)
- [Theoretical Formalism](#theoretical-formalism)
- [Contributing](#contributing)
- [License](#license)

---

## Core Capabilities

### 1. Deductive Physics Engine
SPIRE derives physically valid reactions from fundamental axioms rather than enumerating them from a static database. Given an initial state and centre-of-mass energy $\sqrt{s}$, the engine:
- Validates **all** conservation laws mandated by the active gauge symmetry: electric charge, baryon number, lepton family numbers ($L_e, L_\mu, L_\tau$), weak isospin $T_3$, hypercharge $Y$, and colour.
- Enforces discrete symmetry selection rules ($C$, $P$, $T$) appropriate to the interaction type.
- Reconstructs all kinematically and dynamically allowed final states from the gauge symmetry and vertex rule inventory.
- Identifies mediating gauge bosons and determines the perturbative order.
- Produces detailed violation diagnostics when a process is rejected (e.g., "Forbidden by Lepton Family Number conservation: $\Delta L_\mu = +1$").

### 2. Lagrangian Workbench & Automated Feynman Rule Derivation
Physicists define theories at the Lagrangian level using a human-readable syntax:
```
e * psi_bar * gamma^mu * psi * A_mu
```
SPIRE parses this into a typed Abstract Syntax Tree (AST) with 12 node types (`FieldOp`, `GammaMat`, `CovariantDerivative`, `FieldStrength`, `LorentzIndex`, `GroupIndex`, `MetricTensor`, `LeviCivita`, `Number`, `CouplingConst`, `Product`, `Sum`), then:
- **Derives vertex rules** via successive functional differentiation with respect to external fields, implementing the full Leibniz rule with Grassmann sign tracking for fermionic operators.
- **Validates theoretical consistency** on five axes: Lorentz invariance (all spacetime indices contracted), gauge singlet (U(1) charge conservation, SU(N) $N$-ality), Hermiticity (fermion bilinear pairing), renormalisability (operator mass dimension $\leq 4$), and mass-dimension counting.
- **Computes RGE flows** via 4th-order Runge-Kutta integration of user-defined $\beta$-functions scripted in the embedded Rhai language. Users specify the differential equations; SPIRE integrates them over energy scales.

### 3. Computer Algebra System (CAS) for Spacetime Algebra
A dedicated symbolic engine handles the full Dirac-Lorentz algebra over arbitrary spacetime dimensions:
- **Trace evaluation**: $\text{Tr}[\gamma^\mu \gamma^\nu \gamma^\rho \gamma^\sigma] = 4(g^{\mu\nu}g^{\rho\sigma} - g^{\mu\rho}g^{\nu\sigma} + g^{\mu\sigma}g^{\nu\rho})$ with $d$-dimensional extensions for loop calculations.
- **Index contraction**: Automated Einstein summation across all Lorentz and gauge indices, with metric tensor simplification ($g^{\mu\nu}g_{\nu\rho} = \delta^\mu_\rho$).
- **Spinor algebra**: Dirac equation simplification ($\bar{u}\slashed{p} = \bar{u}m$), completeness relations, and spin-sum evaluations using $\sum_s u(p,s)\bar{u}(p,s) = \slashed{p} + m$.
- **Dimensional regularisation**: Full $d = 4 - 2\varepsilon$ algebra for one-loop calculations with Passarino-Veltman tensor reduction to scalar master integrals ($A_0$, $B_0$, $C_0$, $D_0$).
- **Algebraic proof generation**: Every step in an amplitude derivation is recorded in a hierarchical `ProofDocument` structure that can be compiled to publication-ready LaTeX documents using the `revtex4-2` document class.

### 4. Monte Carlo Integration & Event Generation
- **RAMBO algorithm**: Flat $N$-body Lorentz-Invariant Phase Space (LIPS) generation with exact energy-momentum conservation for arbitrary final-state multiplicities.
- **Cross-section computation**: $\sigma = \frac{1}{2s} \int d\Phi_n \, |\mathcal{M}|^2$ via Monte Carlo sampling with variance estimation and relative error reporting.
- **Hadronic convolution**: Parton-level cross-sections folded with Parton Distribution Functions:

$$\sigma_{pp} = \sum_{a,b} \int dx_1 \, dx_2 \; f_a(x_1, Q^2) \, f_b(x_2, Q^2) \; \hat{\sigma}_{ab}(\hat{s})$$

- **Histogramming engine**: 1D and 2D histograms with bin-level variance estimation, underflow/overflow tracking, thread-safe merge strategy for parallel Monte Carlo, and statistical error propagation.
- **Distributed compute grid**: Phase-space integration partitioned across Web Worker threads with fault-tolerant scheduling, heartbeat monitoring, $1/\sqrt{N}$ convergence tracking, and real-time dashboard.

### 5. Decay Width Calculator & Cascade Decays
- **Automatic partial widths**: For any particle in the active model, SPIRE enumerates all kinematically allowed 2-body (and 3-body) decay channels, constructs the Feynman diagrams, computes $|\mathcal{M}|^2$, and integrates over phase space.
- **Branching ratios**: Normalised from partial widths with exact sum-to-unity enforcement.
- **Narrow-Width Approximation (NWA)**: Cascade decay chains via sequential on-shell propagators $P \to A + B, A \to C + D$, with proper Breit-Wigner insertion.
- **SLHA export**: Decay tables formatted in the SUSY Les Houches Accord standard for interoperability with external tools.

### 6. NLO Dipole Subtraction Framework
- **Catani-Seymour dipole subtraction**: Systematic subtraction of infrared (soft and collinear) divergences in real-emission contributions at NLO QCD.
- **Three subtraction schemes**: Catani-Seymour (default), FKS (Frixione-Kunszt-Signer), and Antenna.
- **Counterterm generation**: Automated multiplicative field renormalisation $\phi_0 = \sqrt{Z}\,\phi_R$ with extraction of all $\mathcal{O}(\delta)$ counterterm vertices and their Feynman rules.
- **Virtual corrections**: Passarino-Veltman tensor reduction framework for one-loop integral decomposition.

### 7. Phenomenological Detector Simulation
- **Parametric detector models**: Configurable efficiency maps, Gaussian energy smearing with $\sigma/E = a/\sqrt{E} \oplus b/E \oplus c$, and $\eta$-acceptance cuts.
- **Anti-$k_t$ jet clustering**: E-scheme four-momentum recombination with configurable jet radius $R$ and $p_T$ threshold.
- **Reconstructed event model**: Jets, isolated leptons ($e$, $\mu$), photons, and missing transverse energy $E_T^{\text{miss}}$.
- **Three presets**: Perfect (unity efficiency), LHC-like (ATLAS/CMS parametrisation with typical acceptances), ILC-like ($e^+e^-$ collider with hermetic coverage).

### 8. Parton Shower Interface
- **External shower bridge**: Configurable interface to Pythia 8, Herwig 7, Sherpa, or custom shower generators via a command-line subprocess protocol.
- **Hadronisation toggle**: Enable/disable non-perturbative hadronisation.
- **Multi-Parton Interactions (MPI)**: Toggle underlying-event simulation.

### 9. External Theory Bridge (SLHA & UFO)
- **SLHA parser**: Full SUSY Les Houches Accord v1 ingestion — mass spectra (`BLOCK MASS`), mixing matrices (`BLOCK NMIX`, `BLOCK UMIX`, `BLOCK VMIX`), decay tables (`DECAY`), running parameters with $Q$-scale tracking, and alpha corrections.
- **UFO bridge**: Pure-Rust parser for Universal FeynRules Output models (WASM-compatible, no Python dependency). Reads `particles.py`, `vertices.py`, `couplings.py`, `parameters.py`, and `lorentz.py`, mapping them into SPIRE's ontology. Handles spin convention translation ($2S+1 \to 2S$), colour codes, and complex coupling parameters.
- **BSM / EFT support**: Arbitrary gauge groups ($SU(N)$, $SO(N)$, $U(1)'$), higher-dimensional operators (dimension-5, -6, -8), and variable spacetime dimensionality ($d = 4 - 2\varepsilon$).

### 10. Parameter Scanning
- **1D parameter scans**: Sweep any model parameter (mass, coupling, width) over a user-defined range while recording the cross-section at each point. Results returned as arrays suitable for plotting $\sigma(p)$ curves.
- **2D parameter scans**: Sweep two parameters simultaneously over independent ranges, producing a $\sigma(p_1, p_2)$ grid for heatmap visualisation (exclusion plots, sensitivity maps).

### 11. Visualisation & Interactive Analysis
- **Feynman diagram renderer**: Vector-graphic topology visualisation with channel ($s$, $t$, $u$) colour coding and symmetry-factor annotation.
- **Interactive diagram editor**: SVG-based drag-and-drop vertex manipulation with force-directed layout.
- **Dalitz plots**: Phase-space boundary computation and point generation for three-body decays.
- **3D event display**: Three.js-powered detector visualisation with energy-scaled jet cones, lepton tracks, photon lines, and $E_T^{\text{miss}}$ arrows, with event-by-event animation and camera orbit.
- **GPU-accelerated heatmaps**: WebGL-based 2D correlation plots with Viridis colourmap.
- **Workspace system**: Multiple named workspaces with tab switching, infinite canvas mode with dot-grid, and docking mode with recursive split-pane layout. Workspace state serialisable to JSON.

### 12. Data Provenance & Reproducibility
- **Cryptographic state hashing**: SHA-256 fingerprinting of the entire computational state (model, reaction, kinematics, random seed, SPIRE version) via canonical JSON serialization.
- **Embedded provenance metadata**: Every exported dataset (LHE files, SLHA tables, LaTeX proof documents) carries the provenance hash and full serialized state in its header, enabling independent verification.
- **Reproducibility loader**: Import a provenance record (from file header or clipboard paste) to perfectly reconstruct the exact computational workspace, guaranteeing bit-for-bit reproducible Monte Carlo integrations.
- **Integrity verification**: Automatic SHA-256 recomputation upon import detects any tampering or corruption.

### 13. Algebraic Proof Generation
- **Hierarchical proof tracking**: Every derivation step (Feynman rule application, trace evaluation, index contraction, simplification) is recorded as a `ProofStep` node in a tree structure.
- **LaTeX compilation**: The proof tree compiles to a publication-ready `.tex` document using the `revtex4-2` document class (APS journal formatting) with proper sectioning, numbered equation environments, and a conclusion.
- **Export from UI**: Context-menu integration in the Amplitude Panel for one-click proof export.

### 14. Extensibility & Interoperability
- **Custom model files**: TOML-based particle and vertex definitions with referential integrity validation and automatic propagator derivation from spin quantum numbers.
- **LaTeX export**: Publication-quality amplitude expressions and complete proof documents.
- **UFO export**: Model export to the Universal FeynRules Output format for MadGraph5_aMC@NLO compatibility.
- **LHE output**: Les Houches Event file generation for interfacing with shower Monte Carlo generators, with optional embedded provenance metadata for reproducibility.
- **Rhai scripting**: User-defined observable and kinematic cut functions evaluated at runtime, with full access to event kinematics and phase-space variables.

### 15. Cosmological Relic Density Calculator
SPIRE computes the present-day dark matter relic abundance $\Omega h^2$ by solving the Boltzmann equation through the thermal freeze-out epoch:

\[
\frac{dY}{dx} =
- \sqrt{\frac{\pi}{45}}\,
M_{\mathrm{Pl}}\,
m_{\chi}\,
\frac{g_{*s}}{\sqrt{g_*}}\,
\frac{\langle \sigma v \rangle}{x^2}\,
\left(Y^2 - Y_{\mathrm{eq}}^2\right)
\]

Key features:
- **Semi-analytic freeze-out method** (Kolb & Turner): Iterative $x_f$ determination via the condition $n_{\text{eq}} \langle\sigma v\rangle = H$, with analytic post-freeze-out solution.
- **Velocity expansion**: $\langle\sigma v\rangle = a + 6b/x$ supporting both $s$-wave and $p$-wave annihilation channels.
- **Numerical post-freeze-out integration**: Optional Dormand-Prince 4(5) adaptive solver for detailed evolution curves beyond the analytic approximation.
- **Planck comparison**: Automatic classification against the Planck satellite measurement ($\Omega_c h^2 = 0.120 \pm 0.001$) as "under-abundant", "compatible", or "over-closes".
- **Interactive dashboard**: Cosmology Panel widget with configurable DM mass, cross-section, and degrees of freedom, plus Chart.js log-log freeze-out evolution plot.

### 16. Generic ODE Solver Infrastructure
The `math::ode` module provides reusable numerical solvers for ordinary differential equations:
- **Runge-Kutta 4**: Classical fourth-order fixed-step method with both scalar and coupled-system support.
- **Dormand-Prince 4(5)**: Adaptive step-size control with embedded error estimation, configurable absolute/relative tolerances, and safety-factor step scaling.
- **Convenience API**: `solve_ode_rk4()` and `solve_ode_adaptive()` one-liner functions for quick integration.

---

## System Architecture

SPIRE follows a strict three-layer architecture that enforces separation of concerns and enables multi-platform deployment:

```
+--------------------------------------------------------------+
|                 SvelteKit Adaptive Workbench                 |
|         19 widget types . Chart.js . Three.js . WebGL        |
|    Compute Grid . Workspace Persistence . CommandRegistry    |
+--------------------------------------------------------------+
|               Tauri IPC / WASM Boundary Layer                |
|     37 IPC commands . serde-wasm-bindgen . PyO3 bindings     |
+--------------------------------------------------------------+
|                      Rust Physics Kernel                     |
|      Ontology . S-Matrix . Graph . Algebra . Kinematics      |
|       Analysis . PDF . Integration . Theory . Scripting      |
|    Decay . NLO . Shower . NWA . Scanner . IO . Provenance    |
|        Math (ODE) . Cosmology (Relic) . Telemetry            |
|           ~37,000 lines . 940+ tests . zero unsafe           |
+--------------------------------------------------------------+
```

### The Rust Kernel (`spire-kernel`)
The headless computational core contains all physics logic. It is a pure Rust library with no runtime dependencies on the UI layer, enabling it to be compiled to WebAssembly, linked into Python via PyO3, or used as a standalone CLI tool. All 25 modules:

| Module | Responsibility |
|--------|----------------|
| `algebra` | CAS engine, Dirac algebra, trace evaluation, amplitude derivation, proof tracking |
| `analysis` | Histogram engine, observable scripting, detector simulation, jet clustering |
| `data_loader` | TOML model ingestion, propagator auto-derivation from spin |
| `decay` | Automatic partial-width calculation, branching ratios, cascade chains |
| `graph` | Feynman topology generation, channel classification, symmetry factors |
| `groups` | Lie algebra engine, gauge symmetry validation, conservation laws |
| `integration` | Monte Carlo integrator, hadronic cross-section convolution with PDFs |
| `interface` | External solver communication (CLI subprocess wrapper) |
| `io` | Event serialization (LHE v3.0), LaTeX proof compiler, data provenance |
| `kinematics` | Mandelstam variables, RAMBO phase-space, Dalitz, Lorentz boosts |
| `lagrangian` | TheoreticalModel assembly, vertex factors, propagators |
| `nlo` | NLO dipole subtraction (Catani-Seymour, FKS, Antenna) |
| `nwa` | Narrow-Width Approximation, cascade decay kinematics |
| `ontology` | Quantum field definitions, state vectors, Poincare representations |
| `pdf` | Parton Distribution Function framework, analytical toy models |
| `reco` | Anti-$k_t$ jet clustering, detector parametrisation, $E_T^{\text{miss}}$ |
| `s_matrix` | Reaction construction, final-state reconstruction, cross-sections |
| `scanner` | 1D and 2D parameter scans over model parameters |
| `scripting` | Rhai script compilation and evaluation |
| `session` | Persistent session management, script/config execution |
| `shower` | External parton shower bridge (Pythia/Herwig/Sherpa) |
| `telemetry` | Computation profiling, timing, resource tracking |
| `theory/` | Lagrangian AST, functional differentiation, validation, RGE solver, SLHA parser, UFO bridge, NLO counterterms |
| `math` | Generic ODE solvers: Runge-Kutta 4 (fixed-step), Dormand-Prince 4(5) (adaptive) |
| `cosmology` | Relic density calculator: Boltzmann freeze-out, thermal cross-sections, $\Omega h^2$ |

### The Bindings Layer (`spire-bindings`)
Exposes the kernel through stable Foreign Function Interfaces:
- **WebAssembly** (`wasm-bindgen`): Full kernel access in the browser via `serde-wasm-bindgen` for zero-copy JS object interchange. Enables offline-capable operation when the Tauri backend is unavailable.
- **Python** (`PyO3`, feature-gated): Extension module for integration with NumPy, SciPy, and Jupyter.

### The Desktop Application (`spire-desktop`)
A **Tauri** + **SvelteKit** application providing a reactive scientific workbench:
- **37 Tauri IPC commands** bridging the frontend to the Rust backend via JSON-serialised, stateless function calls.
- **19 interactive widget types**: Model Loader, Reaction Workspace, Diagram Visualiser, Amplitude Panel, Kinematics View, Dalitz Plotter, Analysis Widget, Event Display, Diagram Editor, Lagrangian Workbench, External Models, Compute Grid Dashboard, Console, References, Telemetry, Particle Table, Decay Table, Notebook, and Cosmology Dashboard.
- **Dual layout modes**: Recursive split-pane docking (with drag-and-drop reorganisation) and infinite-canvas mode (dot-grid, spacebar panning, scroll zoom, magnetic snapping).
- **Multi-workspace tabs**: Named workspaces with independent layout state, canvas viewports, and widget configurations. Tab bar with add/remove/rename.
- **Command Registry**: 20+ dot-namespaced commands (`spire.ui.*`, `spire.view.*`, `spire.export.*`, `spire.provenance.*`) accessible via a command palette (Mod+K).
- **Context menu system**: Global right-click override with per-widget context menus for export, split, tear-off, close, and pipeline operations.
- **Pipeline service**: Widget-to-widget pub/sub data flow with typed ports and link management.
- **Workspace persistence**: Layouts serialisable to JSON with debounced auto-save and LocalStorage restore.

---

## Getting Started

### Prerequisites

| Component | Version | Purpose |
|-----------|---------|---------|
| **Rust** | Edition 2021+ (stable) | Kernel compilation |
| **Cargo** | Latest (bundled with Rust) | Dependency management, build |
| **Node.js** | LTS >= 18 | Frontend build toolchain |
| **npm** | >= 9 | JavaScript package management |

Install Rust via [rustup.rs](https://rustup.rs):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Platform-Specific Dependencies

#### Ubuntu / Debian

Tauri requires several system libraries for WebKit and GTK:

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libwebkit2gtk-4.0-dev \
  libjavascriptcoregtk-4.0-dev \
  libsoup-3.0-dev \
  libglib2.0-dev \
  pkg-config
```

For Fedora / RHEL-based systems:
```bash
sudo dnf install -y \
  openssl-devel \
  gtk3-devel \
  webkit2gtk4.0-devel \
  libsoup3-devel \
  librsvg2-devel \
  file-devel \
  glib2-devel
```

#### macOS

Install Xcode Command Line Tools and Homebrew dependencies:

```bash
xcode-select --install
brew install node
```

macOS ships with all required WebKit and system libraries. No additional packages are needed beyond the Xcode CLT.

#### Windows

1. Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the "Desktop development with C++" workload (MSVC v143+, Windows SDK 10.0+).
2. Install [Node.js LTS](https://nodejs.org/) from the official installer.
3. Install Rust via `rustup-init.exe` from [rustup.rs](https://rustup.rs).
4. Ensure `cargo`, `node`, and `npm` are all on your PATH. Restart your terminal after installation.

### Build & Run

```bash
# 1. Clone the repository
git clone https://github.com/Wartets/SPIRE.git
cd SPIRE

# 2. Install frontend dependencies
cd apps/spire-desktop
npm install

# 3. Run in development mode (with hot-reloading)
npm run dev
# (alternatively: npm run tauri dev)

# 4. Build a release-optimised binary
npm run build
# (alternatively: npm run tauri build)
# Output: apps/spire-desktop/src-tauri/target/release/
```

### Relaunching After Compilation

Once you have built a release binary with `npm run build` (or `npm run tauri build`), you can relaunch the application directly **without recompiling**:

**Windows:**
```bash
# The compiled executable is located at:
apps\spire-desktop\src-tauri\target\release\spire-desktop-backend.exe

# Simply run it:
.\apps\spire-desktop\src-tauri\target\release\spire-desktop-backend.exe
```

**macOS / Linux:**
```bash
# The compiled binary is located at:
apps/spire-desktop/src-tauri/target/release/spire-desktop-backend

# Run it directly:
./apps/spire-desktop/src-tauri/target/release/spire-desktop-backend
```

> **Tip:** You can create a desktop shortcut or alias pointing to the binary for quick access. The release binary is fully self-contained and does not require Node.js or Cargo at runtime.
>
> On Windows you can also find an **NSIS installer** (`.exe`) or **MSI installer** in `target/release/bundle/` after a release build.

### Running the Test Suite

```bash
# Run all kernel tests (940+ tests)
cargo test --workspace

# Run only the physics kernel unit tests
cargo test -p spire-kernel

# Run benchmarks (Criterion)
cargo bench -p spire-kernel

# Run code formatting check
cargo fmt --all -- --check

# Run linter with all workspace lints
cargo clippy --workspace

# Run frontend type/lint check
cd apps/spire-desktop && npx svelte-check
```

### Building the Web Frontend Only

For development without the Tauri desktop wrapper (pure browser mode via WASM):

```bash
cd apps/spire-desktop
npm run build:web
# Output: apps/spire-desktop/build/
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
- Display validation badges: **Lorentz OK** (all indices contracted), **Gauge OK** (charge-neutral), **Hermitian OK**, **Renormalisable OK** (dimension 4).
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

Open the **External Models** widget then the **UFO** tab. Upload the `.py` files from a FeynRules/SARAH UFO output directory (e.g., `particles.py`, `vertices.py`, `couplings.py`, `parameters.py`). SPIRE's pure-Rust parser reads the Python constructor calls, maps UFO spin conventions ($2S+1$) to SPIRE's $2S$ representation, converts colour codes, and assembles a complete `TheoreticalModel`. Click **Apply as Active Model** to use it in all subsequent calculations.

### 4. Inspecting SLHA Spectra

Open the **External Models** widget then the **SLHA** tab. Upload or paste an SLHA file from SPheno, SoftSUSY, or any compatible spectrum generator. The block browser displays mass spectra (`BLOCK MASS`), mixing matrices (`BLOCK NMIX`), and Higgs sector parameters. Decay tables show branching ratios and daughter particles.

### 5. NLO Counterterm Generation

In the **External Models** widget then the **NLO** tab, enter a tree-level term (e.g., `e * psi_bar * gamma^mu * psi * A_mu`), define the field content and external legs, then click **Derive Counterterms**. SPIRE performs multiplicative field renormalisation ($\phi \to \sqrt{Z}\phi$), coupling renormalisation ($g \to g + \delta g$), extracts all $\mathcal{O}(\delta)$ counterterm expressions, and derives their Feynman rules.

### 6. Exporting a Mathematical Proof

After generating a Feynman diagram in the **Diagram Visualiser**, open the **Amplitude Panel** and right-click to select "Export Proof (LaTeX)". SPIRE will:
- Drive the algebraic proof tracker through the complete amplitude derivation.
- Record every step (Feynman rule application, spinor completeness, trace evaluation, index contraction, simplification) in a hierarchical proof tree.
- Compile the tree into a standalone `.tex` file with `revtex4-2` document class, numbered equation environments, and proper APS journal formatting.
- Download the file directly to your system.

### 7. Using the Compute Grid

Open the **Compute Grid Dashboard** widget. Configure the total number of events and optional chunk size, then launch a distributed integration. The dashboard displays:
- **Active nodes** (Web Workers) with real-time status indicators (Idle / Computing / Merging).
- **Global progress bar** with events completed and estimated time remaining.
- **Convergence plot** showing the cross-section uncertainty decreasing as $1/\sqrt{N}$.
- **Merged result** with statistically combined histograms upon completion.

---

## Data Provenance & Reproducibility

SPIRE implements a rigorous data provenance system to guarantee bit-for-bit reproducibility of all Monte Carlo computations. Every numerical result can be independently verified and exactly reproduced.

### How It Works

1. **State Capture**: Before any computation, the entire deterministic state is captured:
   - Full `TheoreticalModel` (all masses, widths, couplings, vertex factors, propagators, gauge symmetry)
   - `Reaction` specification (initial/final states)
   - Kinematic configuration (centre-of-mass energy, event count)
   - Monte Carlo random seed
   - SPIRE version string

2. **Canonical Hashing**: The state is serialized to a deterministic JSON form (struct fields in declaration order, no platform-dependent hash functions) and passed through SHA-256. The resulting 64-character hex digest uniquely identifies the computation.

3. **Metadata Embedding**: The provenance hash and full JSON payload are injected into exported files:
   - **LHE files**: XML comments in the `<header>` block
   - **LaTeX proofs**: LaTeX comments (`%`) before the `\documentclass`

4. **Verification & Restoration**: Anyone with an exported file can:
   - Extract the provenance block from the file header
   - Recompute the SHA-256 to verify integrity
   - Import the payload via the Command Palette ("Load Provenance State") to reconstruct the exact workspace configuration

### Using Provenance

To compute a provenance hash for the current state, use the API:
```typescript
const record = await computeProvenanceHash(model, reaction, cmsEnergy, numEvents, seed);
console.log(record.sha256);   // "7f83b1657ff1fc53b92dc18148a1d65dfc..."
console.log(record.payload);  // Full JSON state
```

To restore a state, open the Command Palette (Mod+K) and select "Load Provenance State", or call:
```typescript
const state = await loadProvenanceState(jsonRecordString);
// state.model, state.reaction, state.kinematics, state.seed restored
```

---

## Repository Structure

```
SPIRE/
+-- crates/
|   +-- spire-kernel/                   Rust physics engine (~37,000 LOC)
|   |   +-- src/
|   |   |   +-- algebra.rs              CAS engine, Dirac algebra, proof tracking
|   |   |   +-- analysis.rs             Histogramming, detector sim, jet clustering
|   |   |   +-- data_loader.rs          TOML model ingestion, propagator derivation
|   |   |   +-- decay.rs                Partial widths, branching ratios
|   |   |   +-- graph.rs                Feynman topology generation
|   |   |   +-- groups.rs               Lie algebra, gauge symmetries, conservation laws
|   |   |   +-- integration.rs          Hadronic cross-section convolution
|   |   |   +-- interface.rs            External solver communication
|   |   |   +-- kinematics.rs           Phase space, Mandelstam, Dalitz, RAMBO
|   |   |   +-- lagrangian.rs           TheoreticalModel, vertex factors, propagators
|   |   |   +-- nlo/                    NLO dipole subtraction framework
|   |   |   +-- nwa.rs                  Narrow-Width Approximation
|   |   |   +-- ontology.rs             Quantum fields, particles, state vectors
|   |   |   +-- pdf.rs                  Parton distribution functions
|   |   |   +-- reco/                   Detector reconstruction, anti-kt clustering
|   |   |   +-- s_matrix.rs             Reactions, cross-sections, event generation
|   |   |   +-- scanner.rs              1D/2D parameter scans
|   |   |   +-- scripting.rs            Rhai observable/cut engine
|   |   |   +-- session.rs              Persistent session management
|   |   |   +-- shower/                 Parton shower interface
|   |   |   +-- telemetry.rs            Computation profiling
|   |   |   +-- io/
|   |   |   |   +-- mod.rs              EventWriter trait, common types
|   |   |   |   +-- lhe.rs              Les Houches Event v3.0 writer
|   |   |   |   +-- latex.rs            LaTeX proof document compiler
|   |   |   |   +-- provenance.rs       SHA-256 state hashing, reproducibility
|   |   |   +-- theory/
|   |   |       +-- ast.rs              Lagrangian AST (12 node types)
|   |   |       +-- derivation.rs       Functional differentiation, Leibniz rule
|   |   |       +-- validation.rs       Lorentz, gauge, Hermiticity checks
|   |   |       +-- renormalization.rs  NLO counterterm generation
|   |   |       +-- rge.rs              Renormalisation Group Equations
|   |   |       +-- slha.rs             SLHA v1 parser
|   |   |       +-- ufo.rs              UFO model importer
|   |   +-- tests/                      Property-based, fuzz, & integration tests
|   |   +-- benches/                    Criterion benchmarks
|   +-- spire-bindings/                 WASM (wasm-bindgen) & Python (PyO3) FFI
+-- apps/
|   +-- spire-desktop/                  Tauri + SvelteKit desktop application
|       +-- src/
|       |   +-- lib/
|       |   |   +-- api.ts              Typed backend API facade (30+ functions)
|       |   |   +-- components/         19 interactive widget components
|       |   |   +-- core/
|       |   |   |   +-- backend/        Backend abstraction (Tauri/WASM/Mock)
|       |   |   |   +-- services/       CommandRegistry, Pipeline, Tutorial, Citation
|       |   |   +-- stores/             Svelte stores (physics, layout, notebook)
|       |   |   +-- types/              TypeScript type definitions
|       |   |   +-- utils/              Export utilities, helpers
|       |   |   +-- workers/            Web Worker compute nodes
|       |   +-- routes/
|       +-- src-tauri/
|           +-- src/main.rs             37 Tauri IPC commands
+-- data/                               Default SM model files (TOML)
+-- docs/                               MkDocs documentation source
+-- agent_notes/                        Development notes and knowledge base
+-- README.md                           This file
+-- roadmap.md                          Master development roadmap
+-- architecture.md                     Technical architecture specification
+-- description.md                      Theoretical framework description
+-- Cargo.toml                          Workspace manifest
```

---

## Theoretical Formalism

SPIRE's computational architecture is grounded in the operator formalism of Quantum Field Theory and the LSZ reduction theorem. The central objects are:

### S-Matrix and Asymptotic States
Physical processes are transitions between asymptotic in- and out-states in Fock space. A scattering amplitude is computed as:

$$\langle f | S | i \rangle = \langle f | T \exp\!\left(-i \int d^4x \, \mathcal{H}_{\text{int}}(x)\right) | i \rangle$$

where $\mathcal{H}_{\text{int}}$ is the interaction Hamiltonian density derived from the user-specified Lagrangian $\mathcal{L}_{\text{int}}$.

### Quantum Ontology
Particles are classified as irreducible representations of the Poincare group $\text{ISO}(1,3)$ (mass $m$ and spin $s$) tensored with representations of internal gauge groups. Each field carries a complete set of quantum numbers validated against the Gell-Mann-Nishijima relation:

$$Q = T_3 + \frac{Y}{2}$$

### Lagrangian Dynamics
Interaction vertices are extracted from $\mathcal{L}_{\text{int}}$ by functional differentiation:

$$V_{\phi_1 \cdots \phi_n}(p_1, \ldots, p_n) = \frac{\delta^n \mathcal{L}}{\delta\phi_1(p_1) \cdots \delta\phi_n(p_n)}$$

with Fourier-convention $\partial_\mu \to -ip_\mu$ for external leg momentum replacement. Grassmann signs are tracked for fermionic operators using an explicit anticommutation rule during the Leibniz expansion.

### Phase-Space Integration
Total cross-sections are computed via Monte Carlo integration over Lorentz-Invariant Phase Space:

$$\sigma = \frac{1}{2s} \int d\Phi_n \, |\mathcal{M}|^2, \qquad d\Phi_n = (2\pi)^4 \delta^{(4)}\!\left(P - \sum_i p_i\right) \prod_{i=1}^{n} \frac{d^3 p_i}{(2\pi)^3 \, 2E_i}$$

The RAMBO algorithm generates unweighted $n$-body phase-space points with exact four-momentum conservation and uniform coverage of the Lorentz-invariant measure.

### Renormalisation
NLO counterterms are generated via multiplicative field renormalisation:

$$\phi_0 = \sqrt{Z_\phi}\,\phi_R \approx \left(1 + \tfrac{1}{2}\delta Z_\phi\right)\phi_R, \qquad g_0 = g_R + \delta g$$

and extraction of all terms linear in the renormalisation constants $\delta Z$, $\delta m$, $\delta g$. This yields the complete set of counterterm Feynman rules required for UV-finite NLO cross-section calculations in the $\overline{\text{MS}}$ scheme.

### Data Provenance
Reproducibility is guaranteed by the provenance engine, which computes a SHA-256 fingerprint of the complete computational state. For any two runs with identical provenance hashes, the Monte Carlo integration produces bit-for-bit identical results (same model, same seed, same kinematics). The cryptographic hash serves as a unique identifier for the "computational reality" that generated a given dataset.

---

## Contributing

Contributions are welcome. Please refer to the project's issue tracker for areas of active development. For major changes, please open an issue first to discuss the proposed modification. All submitted code should:
- Maintain the existing test coverage (currently 940+ tests)
- Compile without warnings under `cargo clippy --workspace`
- Pass `cargo fmt --all -- --check` formatting checks
- Pass `npx svelte-check` with zero errors in the frontend

---

## License

This project is licensed under the MIT License. See the [LICENSE](https://github.com/Wartets/SPIRE/blob/main/LICENSE) file for details.
