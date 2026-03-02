# API Reference

This page documents the programmatic interfaces exposed by SPIRE across its
three binding layers: **Rust kernel**, **Tauri IPC**, and **TypeScript client**.

---

## Rust Kernel (`spire-kernel`)

The kernel is a pure Rust library. All public functions and types are documented
with `///` doc-comments. Generate the full API docs with:

```bash
cargo doc --workspace --no-deps --open
```

### Key Entry Points

| Module | Function | Description |
|---|---|---|
| `data_loader` | `load_particle_database(toml)` | Parse a TOML string into `ParticleDatabase` |
| `data_loader` | `load_vertex_database(toml)` | Parse a TOML string into `VertexDatabase` |
| `data_loader` | `build_model(particles, vertices, name)` | Construct a `TheoreticalModel` |
| `s_matrix` | `construct_reaction(initial, final, model, energy)` | Build and validate a `Reaction` |
| `s_matrix` | `reconstruct_reaction(initial, model, energy)` | Enumerate allowed final states |
| `graph` | `generate_topologies(reaction, model, loop_order)` | Produce `TopologySet` of Feynman diagrams |
| `algebra` | `generate_amplitude(diagram)` | Derive symbolic `AmplitudeResult` |
| `kinematics` | `calculate_thresholds(masses)` | Compute threshold energy |
| `kinematics` | `compute_mandelstam_boundaries(masses, s)` | Mandelstam variable limits |
| `groups` | `validate_conservation_laws(initial, final, interaction)` | Check quantum number conservation |

### Core Types

```rust
// Particle ontology
pub struct Field { id, name, symbol, mass, width, quantum_numbers, interactions }
pub struct Particle { field, shell, helicity, chirality, is_anti }
pub struct QuantumState { particle, four_momentum, normalization }

// Reaction
pub struct Reaction { initial, final_state, is_valid, violation_diagnostics, ... }
pub struct ReconstructedFinalState { particles, weight, interaction_chain }

// Lagrangian
pub struct TheoreticalModel { name, description, fields, terms, vertex_factors, propagators }
pub struct LagrangianTerm { id, description, coupling_symbol, coupling_value, ... }
pub struct VertexFactor { term_id, field_ids, expression, coupling_value, n_legs }
pub struct Propagator { field_id, spin, mass, width, expression, gauge_parameter }

// Diagrams
pub struct FeynmanDiagram { id, nodes, edges, channels, loop_order, symmetry_factor }
pub struct TopologySet { reaction_id, max_loop_order, diagrams, count }

// Amplitudes & Kinematics
pub struct AmplitudeResult { diagram_id, expression, couplings, momenta_labels }
pub struct KinematicsReport { threshold, is_allowed, phase_space, mandelstam_boundaries }
```

---

## Tauri IPC Commands

The desktop application exposes five `#[tauri::command]` functions accessible
via the `invoke()` bridge:

| Command | Parameters | Returns |
|---|---|---|
| `load_theoretical_model` | `particles_toml`, `vertices_toml`, `model_name?` | `TheoreticalModel` |
| `validate_and_reconstruct_reaction` | `initial_ids`, `final_ids?`, `cms_energy`, `model` | `Reaction` or `ReconstructedFinalState[]` |
| `generate_feynman_diagrams` | `reaction`, `model`, `max_loop_order?` | `TopologySet` |
| `derive_amplitude` | `diagram` | `AmplitudeResult` |
| `compute_kinematics` | `final_masses`, `cms_energy`, `target_mass?`, `external_masses?` | `KinematicsReport` |

All commands are stateless — the full model and reaction objects are passed
in each call as serialized JSON.

---

## TypeScript Client (`$lib/api.ts`)

Typed wrapper functions around Tauri's `invoke()`:

```typescript
import { loadModel, constructReaction, generateDiagrams,
         deriveAmplitude, computeKinematics } from '$lib/api';

// Load the Standard Model
const model = await loadModel(particlesToml, verticesToml, "Standard Model");

// Construct a reaction
const reaction = await constructReaction(
  ["e-", "e+"], ["mu-", "mu+"], 10.0, model
);

// Generate tree-level diagrams
const diagrams = await generateDiagrams(reaction, model, 0);

// Derive amplitude for the first diagram
const amplitude = await deriveAmplitude(diagrams.diagrams[0]);

// Compute kinematics
const kinematics = await computeKinematics([0.10566, 0.10566], 10.0);
```

---

## Python Bindings (`spire_bindings`)

Feature-gated behind `python` in `Cargo.toml`. Available when built with PyO3:

```python
import spire_bindings as spire

# Load model
model_json = spire.load_model(particles_toml, vertices_toml, "Standard Model")

# Construct reaction
reaction = spire.construct_reaction(["e-", "e+"], ["mu-", "mu+"], 10.0, model_json)

# Generate diagrams
diagrams = spire.generate_diagrams(reaction.to_json(), model_json)

# Compute amplitude
amplitude = spire.compute_amplitude(diagrams.diagrams_json[0])

# Check threshold
kinematics = spire.calculate_threshold([0.10566, 0.10566], None)
```

---

## WASM Bindings

For browser-based usage via `wasm-bindgen`:

```javascript
import init, { load_model_wasm, construct_reaction_wasm,
               generate_diagrams_wasm } from 'spire_bindings';

await init();
const model = load_model_wasm(particlesToml, verticesToml, "Standard Model");
```

---

*For the full Rust API with doc-comments, run `cargo doc --workspace --no-deps --open`.*
