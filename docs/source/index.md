# SPIRE — Structured Particle Interaction & Reaction Engine

> A computational formalism for High Energy Physics and Quantum Field Theory.

---

## Project Manifest

**SPIRE** is a desktop application and scientific library that automates the
derivation of Feynman diagrams and scattering amplitudes from first principles.
It operates on the principle of strict isomorphism between three representations
of physical phenomena:

1. **Symbolic interaction** — reaction equations ($A + B \to C + D$)
2. **Topological representation** — Feynman diagrams (directed graphs)
3. **Analytical formulation** — invariant amplitudes ($\mathcal{M}$) and cross-sections

The system is grounded in S-Matrix theory, treating physical processes as
transitions between asymptotic states in Hilbert space. Rather than retrieving
pre-calculated scenarios, SPIRE *constructs* valid processes from fundamental
axioms: Lagrangian densities, symmetry groups, and conservation laws.

## Architecture

| Layer | Technology | Purpose |
|---|---|---|
| **Kernel** | Rust (`spire-kernel`) | Physics engine — ontology, groups, S-matrix, Lagrangian, graphs, algebra, kinematics |
| **Bindings** | Rust (`spire-bindings`) | WASM + Python FFI wrappers |
| **Desktop** | Tauri + SvelteKit | GUI with IPC bridge to the kernel |

### Module Map

- `ontology` — Particle fields, quantum numbers, state construction
- `groups` — Gauge group validation, conservation law checks, CPT symmetry
- `s_matrix` — Reaction construction, final-state reconstruction, mediator identification
- `lagrangian` — Model parsing (TOML), vertex factor / propagator derivation
- `graph` — Feynman diagram topology generation via `petgraph`
- `algebra` — Symbolic amplitude construction, Dirac traces, Lorentz contractions
- `kinematics` — Thresholds, phase space, Mandelstam boundaries, Dalitz plots

## Status

All 11 roadmap items are **complete**:

- [x] Rust kernel with 179 tests + 2 doc-tests passing
- [x] WASM, Python, and Tauri IPC bindings
- [x] SvelteKit frontend with 6 interactive components
- [x] Mermaid.js graphical Feynman diagram rendering
- [x] Embedded Standard Model data (22 particles, 14 vertices)

## Getting Started

See the [Quick Start Guide](guide/quickstart.md) for instructions on loading a
model, defining reactions, and visualising diagrams.

## License

This project is developed as a scientific research tool. See the repository root
for licensing information.
