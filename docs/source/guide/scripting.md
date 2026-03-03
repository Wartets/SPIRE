# Scripting Engine

SPIRE embeds the [Rhai](https://rhai.rs) scripting language, enabling
researchers to define **custom observables**, **kinematic cuts**, and
**momentum-dependent form factors** at runtime — without recompiling
the kernel or writing Rust code.

---

## Overview

The scripting subsystem lives in the `scripting` module of `spire-kernel`.
Its architecture follows three principles:

1. **Compile once, evaluate millions of times.** Every user script is parsed
   into an Abstract Syntax Tree (AST) before the Monte Carlo loop begins.
   No string parsing occurs inside the hot loop.
2. **Full access to kinematics.** Scripts receive a `PhaseSpacePoint` object
   and can call any registered method on its constituent 4-vectors.
3. **Object-safe traits.** Custom observables and cuts implement the same
   Rust traits as native implementations, so they participate seamlessly
   in integration pipelines.

---

## Registered Types

### `SpacetimeVector`

Each 4-momentum in an event is exposed as a `SpacetimeVector` with the
following methods:

| Method | Returns | Description |
|---|---|---|
| `.e()` | `f64` | Energy component $v^0$ |
| `.px()` | `f64` | Spatial $x$-component $v^1$ |
| `.py()` | `f64` | Spatial $y$-component $v^2$ |
| `.pz()` | `f64` | Spatial $z$-component $v^3$ |
| `.pt()` | `f64` | Transverse momentum $p_T = \sqrt{p_x^2 + p_y^2}$ |
| `.eta()` | `f64` | Pseudorapidity $\eta = \mathrm{atanh}(p_z / |\vec{p}|)$ |
| `.phi()` | `f64` | Azimuthal angle $\phi = \mathrm{atan2}(p_y, p_x)$ |
| `.m()` | `f64` | Invariant mass $m = \sqrt{|E^2 - |\vec{p}|^2|}$ |
| `.rapidity()` | `f64` | Rapidity $y = \tfrac{1}{2}\ln\tfrac{E + p_z}{E - p_z}$ |

Arithmetic operators `+`, `-`, and scalar `*` are also registered, so
vector algebra works naturally inside scripts.

### Constructor: `vec4(e, px, py, pz)`

Scripts can construct new 4-vectors directly:

```rhai
let total = vec4(0.0, 0.0, 0.0, 0.0);
```

### `PhaseSpacePoint`

The top-level object passed to every script. Properties:

| Property | Type | Description |
|---|---|---|
| `event.momenta` | Array of `SpacetimeVector` | Final-state particle 4-momenta |
| `event.weight` | `f64` | Phase-space integration weight |

---

## Writing Observables

An **observable** is a script that maps a phase-space event to a scalar
value. The script must evaluate to a numeric (`f64`) result.

### Example: Leading Particle Transverse Momentum

```rhai
let p = event.momenta[0];
p.pt()
```

### Example: Invariant Mass of a Particle Pair

$$
M_{12} = \sqrt{(p_1 + p_2)^2}
$$

```rhai
let p1 = event.momenta[0];
let p2 = event.momenta[1];
let total = p1 + p2;
total.m()
```

### Example: Transverse Mass

The transverse mass of the leading lepton and missing transverse energy,
commonly used in $W$ boson analyses:

$$
M_T = \sqrt{2\,p_{T,1}\,p_{T,2}\,\bigl(1 - \cos\Delta\phi\bigr)}
$$

```rhai
let p1 = event.momenta[0];
let p2 = event.momenta[1];
let dphi = p1.phi() - p2.phi();
let mt_sq = 2.0 * p1.pt() * p2.pt() * (1.0 - dphi.cos());
mt_sq.sqrt()
```

### Example: Total Visible Energy

```rhai
let total_e = 0.0;
for p in event.momenta {
    total_e += p.e();
}
total_e
```

---

## Writing Kinematic Cuts

A **kinematic cut** is a script that returns a boolean. Events for which
the script evaluates to `false` are discarded from the Monte Carlo
integration.

### Example: Transverse Momentum Threshold

Require the leading particle to have $p_T > 50\,\text{GeV}$:

```rhai
let p = event.momenta[0];
p.pt() > 50.0
```

### Example: Central Pseudorapidity

Require all particles to satisfy $|\eta| < 2.5$ (typical detector
acceptance):

```rhai
let pass = true;
for p in event.momenta {
    if p.eta().abs() > 2.5 {
        pass = false;
    }
}
pass
```

### Example: Combined $p_T$ and $\eta$ Cut

```rhai
let p = event.momenta[0];
p.pt() > 25.0 && p.eta().abs() < 2.4
```

### Example: Invariant Mass Window

Select events where the di-particle invariant mass lies near the $Z$
boson peak ($80 < M_{12} < 100\,\text{GeV}$):

```rhai
let total = event.momenta[0] + event.momenta[1];
let m = total.m();
m > 80.0 && m < 100.0
```

---

## Writing Form Factors

A **form factor** script receives the squared momentum transfer as the
variable `q2` (a plain `f64`) and must return a dimensionless suppression
factor $F(Q^2) \in [0, 1]$.

### Example: Dipole Form Factor

$$
F(Q^2) = \left(1 + \frac{Q^2}{\Lambda^2}\right)^{-2}
\quad\text{with}\quad \Lambda^2 = 0.71\;\text{GeV}^2
$$

```rhai
let lambda_sq = 0.71;
let x = 1.0 + q2 / lambda_sq;
1.0 / (x * x)
```

### Example: Exponential Form Factor

$$
F(Q^2) = \exp\!\left(-\frac{Q^2}{\Lambda^2}\right)
$$

```rhai
let lambda_sq = 1.0;
(-q2 / lambda_sq).exp()
```

Form factors defined via the `FormFactor::Scripted` variant in the
Lagrangian module use this mechanism internally.

---

## Compilation and Validation

All scripts are compiled before use. There are two compilation pathways:

### Programmatic (Rust API)

```rust
use spire_kernel::scripting::SpireScriptEngine;

let engine = SpireScriptEngine::new();

// Compile an observable
let obs = engine.compile_observable("event.momenta[0].pt()")
    .unwrap()
    .with_name("leading_pt");

// Compile a cut
let cut = engine.compile_cut("event.momenta[0].pt() > 50.0")
    .unwrap()
    .with_name("pt_cut_50");

// Validate without creating a wrapper
let result = engine.validate_script("let x = 1 + ");
assert!(result.is_err());
```

### GUI (Desktop Application)

The **Reaction Workspace** panel includes an "Observables & Cuts" section
where scripts can be entered in a text area. The application validates
scripts in real-time as you type and reports compilation errors inline.

---

## Integration Pipeline

Custom observables and cuts integrate naturally with the Monte Carlo
engine:

```rust
use spire_kernel::integration::UniformIntegrator;
use spire_kernel::scripting::SpireScriptEngine;

let engine = SpireScriptEngine::new();
let cuts: Vec<Box<dyn spire_kernel::scripting::KinematicCut>> = vec![
    Box::new(
        engine.compile_cut("event.momenta[0].pt() > 25.0").unwrap()
    ),
];

let cut_refs: Vec<&dyn spire_kernel::scripting::KinematicCut> =
    cuts.iter().map(|c| c.as_ref()).collect();

let integrator = UniformIntegrator;
let result = integrator.integrate_with_cuts(
    |ps| 1.0,       // amplitude squared
    &mut generator,
    cms_energy,
    &final_masses,
    num_events,
    &cut_refs,
).unwrap();
```

The `integrate_with_cuts()` and `integrate_with_cuts_parallel()` methods
accept any collection of `&dyn KinematicCut`, allowing arbitrary
combinations of native Rust cuts and Rhai-scripted cuts.

---

## Performance Considerations

| Aspect | Detail |
|---|---|
| Compilation | One-time cost (~microseconds per script) |
| Evaluation | ~100 ns per event per script (Rhai interpreter overhead) |
| Memory | Each compiled AST is `Send + Sync`, shared via `Arc<Engine>` |
| Thread safety | Enabled by the `rhai/sync` feature flag |

For the highest-throughput applications (>10⁹ events), consider
implementing custom observables directly in Rust via the `Observable`
and `KinematicCut` traits. For typical physics studies (~10⁶ events),
Rhai scripts impose negligible overhead.

---

*See also: [API Reference](../api/reference.md) · [Quick Start](quickstart.md)*
