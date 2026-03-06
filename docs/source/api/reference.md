# API Reference

This page documents the programmatic interfaces exposed by SPIRE across its
four binding layers: **Rust kernel**, **Tauri IPC**, **TypeScript client**,
and **Python library**.

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
| `s_matrix` | `calculate_cross_section(energy, masses, amp, n)` | Monte Carlo cross-section |
| `s_matrix` | `calculate_hadronic_cross_section(...)` | PDF-convoluted hadronic cross-section |
| `graph` | `generate_topologies(reaction, model, loop_order)` | Produce `TopologySet` of Feynman diagrams |
| `algebra` | `generate_amplitude(diagram)` | Derive symbolic `AmplitudeResult` |
| `algebra` | `derive_amplitude_steps(diagram, dim)` | Step-by-step CAS derivation |
| `kinematics` | `calculate_thresholds(masses)` | Compute threshold energy |
| `kinematics` | `compute_mandelstam_boundaries(masses, s)` | Mandelstam variable limits |
| `kinematics` | `generate_dalitz_plot_data(M, masses, n)` | Dalitz plot boundary data |
| `groups` | `validate_conservation_laws(initial, final, interaction)` | Check quantum number conservation |
| `groups` | `validate_generalized_conservation(symmetry, init, fin)` | Product-group conservation |
| `scripting` | `SpireScriptEngine::new()` | Create embedded Rhai scripting engine |
| `io` | `LheWriter::new(writer)` | Create LHE v3.0 event writer |
| `interface` | `CliSolver::new(executable)` | Create CLI subprocess solver |

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
pub enum FormFactor { PointLike, Monopole, Dipole, Exponential, Scripted { script } }

// Diagrams
pub struct FeynmanDiagram { id, nodes, edges, channels, loop_order, symmetry_factor }
pub struct TopologySet { reaction_id, max_loop_order, diagrams, count }

// Amplitudes & Kinematics
pub struct AmplitudeResult { diagram_id, expression, couplings, momenta_labels }
pub struct KinematicsReport { threshold, is_allowed, phase_space, mandelstam_boundaries }
pub struct PhaseSpacePoint { momenta: Vec<SpacetimeVector>, weight: f64 }

// Integration
pub struct IntegrationResult { value, error, events_evaluated, efficiency, relative_error }
pub struct CrossSectionResult { cross_section, uncertainty, ..._pb, cms_energy }
pub struct HadronicCrossSectionResult { cross_section, uncertainty, ..._pb, beam_energy_sq }
```

---

## Scripting Engine (`scripting`)

The embedded Rhai engine exposes physics types to user scripts:

| Entry Point | Description |
|---|---|
| `SpireScriptEngine::new()` | Create engine with registered types |
| `engine.compile_observable(script)` | Compile script → `RhaiObservable` |
| `engine.compile_cut(script)` | Compile script → `RhaiCut` |
| `engine.compile_form_factor(script)` | Compile script → `CompiledFormFactor` |
| `engine.validate_script(script)` | Syntax-check without creating wrapper |

### Traits

```rust
/// A computed physical quantity evaluated on each Monte Carlo event.
pub trait Observable: Send + Sync {
    fn evaluate(&self, event: &PhaseSpacePoint) -> f64;
    fn name(&self) -> &str;
}

/// A boolean predicate for event selection.
pub trait KinematicCut: Send + Sync {
    fn is_passed(&self, event: &PhaseSpacePoint) -> bool;
    fn name(&self) -> &str;
}
```

See the [Scripting Guide](../guide/scripting.md) for usage examples.

---

## Event I/O (`io`)

### `EventWriter` Trait

```rust
pub trait EventWriter: Send {
    fn write_header(&mut self, config: &InitConfig) -> SpireResult<()>;
    fn write_event(&mut self, event: &EventRecord) -> SpireResult<()>;
    fn finish(&mut self) -> SpireResult<()>;
}
```

### Data Structures

| Type | Description |
|---|---|
| `EventParticle` | Single particle: PDG ID, status, mothers, colour, 4-momentum, lifetime, spin |
| `EventRecord` | Complete event: particles, weight, scale, $\alpha_{\text{QED}}$, $\alpha_{\text{QCD}}$ |
| `InitConfig` | File header: beam IDs/energies, PDF info, process list, generator name |
| `ProcessInfo` | Subprocess entry: cross-section, error, max weight, process ID |

### `LheWriter<W: Write>`

Concrete implementation writing LHE v3.0 format:

```rust
use spire_kernel::io::lhe::LheWriter;
use spire_kernel::io::EventWriter;

let mut writer = LheWriter::new(std::io::stdout());
writer.write_header(&config)?;
writer.write_event(&event)?;
writer.finish()?;
```

### Helper Function

`event_record_from_phase_space(ps, pdg_ids, beam_ids, beam_energy, process_id)` -
converts a `PhaseSpacePoint` and metadata into an `EventRecord` suitable
for the `EventWriter` interface.

See the [Interop Guide](../guide/interop.md) for full examples.

---

## External Solver Interface (`interface`)

### `ExternalSolver` Trait

```rust
pub trait ExternalSolver: Send + Sync {
    fn name(&self) -> &str;
    fn solve(&self, input: &str) -> SpireResult<String>;
}
```

### `CliSolver`

Builder-pattern subprocess wrapper:

```rust
use spire_kernel::interface::CliSolver;

let solver = CliSolver::new("fire")
    .args(&["-auto"])
    .timeout_secs(60)
    .working_dir("/tmp")
    .env("FIRE_THREADS", "4");

let result = solver.solve("input data")?;
```

| Method | Description |
|---|---|
| `new(executable)` | Create solver for given binary |
| `.args(slice)` | Append command-line arguments |
| `.timeout_secs(n)` | Set process timeout (default: 30 s) |
| `.working_dir(path)` | Set working directory |
| `.env(key, value)` | Add environment variable |
| `.solve(input)` | Execute and return stdout |

---

## Tauri IPC Commands

The desktop application exposes twelve `#[tauri::command]` functions accessible
via the `invoke()` bridge:

| Command | Parameters | Returns |
|---|---|---|
| `load_theoretical_model` | `particles_toml`, `vertices_toml`, `model_name?` | `TheoreticalModel` |
| `validate_and_reconstruct_reaction` | `initial_ids`, `final_ids?`, `cms_energy`, `model` | `Reaction` or `ReconstructedFinalState[]` |
| `generate_feynman_diagrams` | `reaction`, `model`, `max_loop_order?` | `TopologySet` |
| `derive_amplitude` | `diagram` | `AmplitudeResult` |
| `derive_amplitude_steps` | `diagram`, `dimension?` | `DerivationStep[]` |
| `compute_kinematics` | `final_masses`, `cms_energy`, `target_mass?`, ... | `KinematicsReport` |
| `compute_dalitz_data` | `parent_mass`, `child_masses`, `n_points` | `DalitzPlotData` |
| `export_amplitude_latex` | `amplitude` | `String` (LaTeX) |
| `export_model_ufo` | `model` | `HashMap<String, String>` |
| `validate_script` | `script` | `()` or error message |
| `test_observable_script` | `script` | `f64` |
| `test_cut_script` | `script` | `bool` |

All commands are stateless - the full model and reaction objects are passed
in each call as serialized JSON.

---

## TypeScript Client (`$lib/api.ts`)

Typed wrapper functions around Tauri's `invoke()`:

```typescript
import {
    loadModel, constructReaction, reconstructReaction,
    generateDiagrams, deriveAmplitude, deriveAmplitudeSteps,
    computeKinematics, computeDalitzData,
    exportAmplitudeLatex, exportModelUfo,
    validateScript, testObservableScript, testCutScript,
} from '$lib/api';

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

// Step-by-step derivation
const steps = await deriveAmplitudeSteps(diagrams.diagrams[0]);

// Compute kinematics
const kinematics = await computeKinematics([0.10566, 0.10566], 10.0);

// Validate a user script
const ok = await validateScript("event.momenta[0].pt()");
```

---

## Python Bindings (`spire-hep`)

Feature-gated behind `python` in `Cargo.toml`. Built with PyO3 and Maturin:

```python
import spire_hep

# Load model
model_json = spire_hep.load_model(particles_toml, vertices_toml, "SM")

# Construct reaction
reaction = spire_hep.construct_reaction(
    ["e-", "e+"], ["mu-", "mu+"], 10.0, model_json
)

# Cross-section computation
result = spire_hep.calculate_cross_section(91.2, [0.10566, 0.10566], 50000)
print(f"σ = {result.cross_section_pb:.4f} pb")

# Zero-copy NumPy event generation
momenta, weights = spire_hep.generate_phase_space_events(
    cms_energy=91.2,
    final_masses=[0.10566, 0.10566],
    num_events=100_000,
    seed=42,
)
# momenta.shape == (100000, 2, 4)  - [event, particle, (E, px, py, pz)]
# weights.shape == (100000,)
```

See the [Interop Guide](../guide/interop.md) for detailed Python examples.

---

## WASM Bindings

For browser-based usage via `wasm-bindgen`:

```javascript
import init, {
    loadModel, constructReaction, reconstructReaction,
    generateDiagrams, deriveAmplitude, computeKinematics,
    calculateCrossSection, calculateHadronicCrossSection,
    computeDalitzData, exportAmplitudeLatex, exportModelUfo,
} from 'spire_bindings';

await init();
const model = loadModel(particlesToml, verticesToml, "Standard Model");
```

---

*For the full Rust API with doc-comments, run `cargo doc --workspace --no-deps --open`.*
