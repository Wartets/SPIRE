# Quick Start Guide

This guide walks you through a complete SPIRE workflow: loading a theoretical
model, constructing a reaction, generating Feynman diagrams, and inspecting
amplitudes and kinematics.

---

## Prerequisites

- **Rust** toolchain (≥ 1.70) for the kernel
- **Node.js** (≥ 18) and **npm** for the frontend
- The SPIRE repository cloned locally

## Launching the Application

SPIRE runs as a Tauri desktop application:

```bash
cd apps/spire-desktop
npm install --legacy-peer-deps
npm run dev          # launches the Tauri dev server + SvelteKit frontend
```

The application window opens with a three-column dashboard.

---

## Step 1: Load a Model

The **Model Loader** panel (left sidebar, top) comes pre-populated with the
Standard Model definition - 22 particles and 14 interaction vertices.

1. Optionally select a **Framework** from the dropdown (Standard Model, QED,
   QCD, Electroweak, or BSM).
2. Click **"Load Model"**.
3. A green status badge confirms the model is loaded:
   `✓ Standard Model - 22 fields, 14 vertices`

!!! tip "Custom Models"
    Click **"▸ Edit TOML Data"** to reveal the TOML editors. You can modify
    particle properties or add entirely new fields and vertices, then reload.

## Step 2: Define a Reaction

The **Reaction Workspace** panel (left sidebar, bottom) lets you specify
initial and final states.

1. The default reaction is **e⁻ e⁺ → μ⁻ μ⁺** at **√s = 10 GeV**.
2. Add or remove particles using the dropdown selector and **+ Add** / **×** buttons.
3. Adjust the **√s (GeV)** centre-of-mass energy.
4. Set **Max Loops** (0 = tree-level).

## Step 3: Run the Pipeline

Click **"▶ Run Full Pipeline"** to execute the complete analysis chain:

| Step | Description | Store Updated |
|---|---|---|
| **Construct** | Validates quantum numbers and conservation laws | `activeReaction` |
| **Diagrams** | Generates all topologically distinct Feynman diagrams | `generatedDiagrams` |
| **Amplitudes** | Derives symbolic amplitude $\mathcal{M}$ for each diagram | `amplitudeResults` |
| **Kinematics** | Computes threshold, phase space, Mandelstam boundaries | `kinematics` |

You can also run each step individually using the smaller buttons below.

## Step 4: Inspect Feynman Diagrams

The centre panel shows the **Diagram Visualizer**:

- **Graph View** (default): Mermaid.js renders a left-to-right flowchart.
    - Green stadium nodes = incoming particles
    - Red stadium nodes = outgoing particles
    - Blue circles = interaction vertices
    - Arrows labelled with particle symbol and momentum
- **Text View**: Toggle with the **"≡ Text View"** button for an ASCII
  representation.
- **Edge Table**: Always visible below, listing source → target with particle
  and momentum labels.

Select different diagrams using the numbered tabs.

## Step 5: Analyse Amplitudes

The **Amplitude Panel** (right sidebar, top) shows:

- The full symbolic expression for the selected amplitude.
- Clickable cards for each diagram's amplitude with coupling constants and
  momentum labels.

## Step 6: Review Kinematics

The **Kinematics View** (right sidebar, bottom) displays:

- **Threshold energy** $\sqrt{s}_{\text{thr}}$ and whether the reaction is
  kinematically allowed.
- **Phase space** parameters ($n_{\text{final}}$, $n_{\text{variables}}$, $d\Phi$).
- **Mandelstam boundaries** ($s_{\min}$, $t_{\min}$, $t_{\max}$).

## Step 7: Monitor the Console

The **Log Console** (bottom bar) records all system messages:

- Model load confirmations
- Reaction validation results (✓ / ✗)
- Diagram and amplitude counts
- Error messages (highlighted in red)

Click **Clear** to reset the log.

---

## Reconstruction Mode

Instead of specifying a complete reaction, you can explore what final states
are allowed:

1. Define only the initial state (e.g., e⁻ e⁺).
2. Clear the final state particles.
3. Click **"Reconstruct"**.
4. SPIRE enumerates all kinematically and dynamically allowed two-body final
   states with their interaction weights.

---

## Example: Bhabha Scattering

| Parameter | Value |
|---|---|
| Initial state | e⁻, e⁺ |
| Final state | e⁻, e⁺ |
| √s | 10 GeV |
| Max Loops | 0 |

This generates both the **s-channel** (annihilation via γ/Z) and **t-channel**
(scattering via γ/Z) diagrams with their distinct amplitude expressions.

---

*For API-level documentation, see the [API Reference](../api/reference.md).*
