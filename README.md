# SPIRE: Structured Particle Interaction and Reaction Engine

![Build Status](https://img.shields.io/github/actions/workflow/status/Wartets/spire/rust.yml?branch=main&style=for-the-badge&logo=github)
![Crates.io](https://img.shields.io/crates/v/spire-kernel?style=for-the-badge&logo=rust)
![License](https://img.shields.io/github/license/Wartets/spire?style=for-the-badge)
[![Documentation](https://img.shields.io/badge/docs-mkdocs-blue?style=for-the-badge&logo=markdown)](https://github.com/Wartets/spire/tree/main/docs)

## Table of Contents
1.  [Abstract](#abstract)
2.  [Key Features](#key-features)
3.  [Theoretical Formalism](#theoretical-formalism)
4.  [System Architecture and Capabilities](#system-architecture-and-capabilities)
5.  [Installation and Usage](#installation-and-usage)
6.  [Documentation](#documentation)
7.  [Contributing](#contributing)
8.  [License](#license)
9.  [Citation](#citation)

## Abstract

SPIRE is a computational formalism and software environment designed for the rigorous analysis of High Energy Physics (HEP) and Quantum Field Theory (QFT). Operating on the principle of strict isomorphism between symbolic, topological, and analytical representations of physical phenomena, SPIRE functions as a deductive engine. Rather than retrieving pre-calculated scenarios, the system constructs valid physical processes from fundamental axioms—Lagrangian densities, symmetry groups, and conservation laws. It provides a unified workspace for the automated derivation of Feynman diagrams, the evaluation of symbolic scattering amplitudes, and the computation of kinematic phase space boundaries within the Standard Model and user-defined Beyond Standard Model (BSM) effective field theories.

## Key Features

*   **Deductive Physics Engine**: Derives valid reactions from first principles, not from a static database.
*   **Automated Topology Generation**: Enumerates all topologically distinct tree-level Feynman diagrams for any valid process.
*   **Symbolic Amplitude Calculation**: Constructs the invariant amplitude $\mathcal{M}$ from Feynman rules.
*   **Advanced Kinematic Analysis**: Computes reaction thresholds, Mandelstam variables, and Dalitz plot distributions.
*   **BSM & EFT Support**: Enables the definition of custom particles, interactions, and higher-dimensional operators via simple TOML-based model files.
*   **Interoperability**: Exports symbolic amplitudes to **LaTeX** and theoretical models to the **Universal FeynRules Output (UFO)** format for integration with external tools.
*   **High-Performance Core**: Built with a memory-safe, computationally efficient Rust kernel for scientific-grade precision and stability.

## Theoretical Formalism

The core architecture of SPIRE is grounded in S-Matrix theory, treating physical processes as transitions between asymptotic states in a Hilbert space. The system enforces theoretical consistency through three primary layers:

1.  **Isomorphic State Representation**: A physical process is represented by a single, unified data structure that maintains a strict isomorphism between its three manifestations: the symbolic reaction equation ($A+B \to C+D$), the topological graph (Feynman diagrams), and the analytical expression (invariant amplitude $\mathcal{M}$).

2.  **Quantum Ontology**: Fundamental particles are treated as irreducible representations of the Poincaré group and relevant internal gauge groups (e.g., $SU(3)_C \times SU(2)_L \times U(1)_Y$). Quantum state vectors are rigorously defined by their invariant mass, spin, and a complete set of quantum numbers.

3.  **Lagrangian Dynamics**: Interaction vertices are derived directly from a user-specified Lagrangian density. The system parses field theoretic terms to extract Feynman rules, ensuring that all generated topologies correspond to valid terms in the interaction Hamiltonian.

4.  **Symmetry & Conservation Invariance**: Transition validity is determined by group-theoretic invariants, not arithmetic heuristics. The engine verifies the conservation of four-momentum, angular momentum, and internal quantum numbers, while enforcing selection rules (C, P, T symmetries) appropriate to the interaction type (strong, electromagnetic, or weak).

## System Architecture and Capabilities

### Symbolic Reaction Interface
*   **Asymptotic State Definition**: Users define initial and final states as formal quantum state vectors, not simple text.
*   **Conservation Validator**: Every proposed reaction is validated against the full set of conservation laws dictated by the active theoretical model. Forbidden processes are rejected with a precise diagnostic of the violated symmetry.
*   **Reaction Reconstruction**: Given an initial state and center-of-mass energy, the engine can deduce and enumerate all kinematically and dynamically allowed two-body final states.

### Topological Construction & Feynman Calculus
*   **Automated Topology Generation**: The system enumerates all topologically distinct Feynman diagrams for a given reaction at tree-level.
*   **Channel Isolation**: Diagrams are algorithmically categorized by kinematic channels ($s, t, u$), allowing for the independent analysis of resonance structures and exchange forces.
*   **Symmetry Factor Calculation**: Combinatorial factors arising from identical particles in the final state are computed automatically.
*   **Graphical Visualization**: Topologies are rendered as vector graphics, providing a clear visual representation of the underlying mathematical structure.

### Symbolic Amplitude Derivation
*   **Feynman Rule Mapping**: For every valid topological graph, SPIRE constructs the invariant amplitude $\mathcal{M}$ by mapping graph nodes and edges to their corresponding symbolic expressions (Dirac spinors, polarization vectors, metric tensors, and propagators).
*   **Algebraic Structuring**: The system correctly orders Dirac matrices and contracts Lorentz indices, producing a symbolic Abstract Syntax Tree (AST) of the amplitude.
*   **LaTeX Export**: Symbolic expressions can be exported directly into publication-quality LaTeX source code for inclusion in scientific documents.

### Kinematic & Phase Space Analysis
*   **Threshold Calculation**: Determines the minimum center-of-mass energy required for particle production.
*   **Mandelstam Variables**: Computes the kinematic invariants ($s, t, u$) and their physical boundaries, which are defined by the Källén function $\lambda(x, y, z)$.
*   **Dalitz Plot Generation**: For three-body decays, the system generates the full phase space distribution ($m_{ab}^2$ vs $m_{bc}^2$) to visualize resonance bands and interference effects.
*   **Lorentz Transformations**: The engine provides functions for boosting four-momenta between reference frames (e.g., Center-of-Mass to Laboratory frame).

### Extensibility: BSM & Effective Field Theory
*   **Custom Lagrangians**: Users can define arbitrary theoretical models, including new particle spectra and interaction vertices, via human-readable TOML-based files. The engine performs strict referential integrity checks to ensure model consistency.
*   **Effective Field Theory (EFT)**: The system supports the inclusion of higher-dimensional operators (e.g., dimension-6 four-fermion contact interactions) for model-independent analyses of new physics.
*   **UFO Interoperability**: Theoretical models defined within SPIRE can be exported to a simplified Universal FeynRules Output (UFO) format, enabling integration with external Monte Carlo event generators such as MadGraph5_aMC@NLO.

### Computational Core & API Design
*   **Rust Kernel (`spire-kernel`)**: A headless, high-performance computational engine containing the complete physics logic. Its memory-safe and strongly-typed design guarantees numerical stability and correctness.
*   **Decoupled Interface Layer**: A cross-platform desktop application provides a reactive environment for model building and analysis, communicating with the kernel via a stateless serialization protocol.
*   **Multi-Language Bindings (`spire-bindings`)**: The kernel's core functionality is exposed via stable APIs for Python, WebAssembly, and other environments, facilitating integration into diverse scientific computing workflows.

## Installation and Usage

SPIRE requires the Rust toolchain, a Node.js environment, and standard system build tools.

### Prerequisites
*   Rust (edition 2021 or later) and Cargo
*   Node.js (LTS version) and npm
*   On Linux: `build-essential`, `libwebkit2gtk-4.0-dev`, `libgtk-3-dev`.

### Build Instructions

1.  **Clone the Repository**
    ```bash
    git clone https://github.com/Wartets/spire.git
    cd spire
    ```

2.  **Install Frontend Dependencies**
    ```bash
    npm install
    ```

3.  **Build and Run the Application**
    *   To run in development mode (with hot-reloading):
        ```bash
        npm run tauri dev
        ```
    *   To build a release-optimized executable:
        ```bash
        npm run tauri build
        ```
        The application binary will be located in `apps/spire-desktop/src-tauri/target/release/`.

### Running Tests
To verify the physical correctness and computational integrity of the kernel:
```bash
cargo test --workspace
```

## Documentation

Comprehensive documentation regarding the theoretical formalism, API reference, and user guides is available in the [`docs/`](https://github.com/Wartets/spire/tree/main/docs) directory of this repository.

*   **Theoretical Formalism**: [`docs/source/theory/formalism.md`](https://github.com/Wartets/spire/blob/main/docs/source/theory/formalism.md)
*   **GUI Quickstart Guide**: [`docs/source/guide/quickstart.md`](https://github.com/Wartets/spire/blob/main/docs/source/guide/quickstart.md)
*   **API Reference**: [`docs/source/api/reference.md`](https://github.com/Wartets/spire/blob/main/docs/source/api/reference.md)

To build and serve the documentation locally using MkDocs:
```bash
# pip install mkdocs mkdocs-material
mkdocs serve
```

## Contributing

Contributions are welcome. Please refer to the project's issue tracker for areas of active development. For major changes, please open an issue first to discuss what you would like to change.

## License

This project is licensed under the MIT License. See the [`LICENSE`](https://github.com/Wartets/spire/blob/main/LICENSE) file for details.

## Citation

If you use SPIRE for research or educational purposes, please cite this repository:
```
Wartets, et al. "SPIRE: Structured Particle Interaction and Reaction Engine." GitHub, 2024, https://github.com/Wartets/spire.
```