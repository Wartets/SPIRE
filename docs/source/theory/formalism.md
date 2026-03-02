# Theoretical Formalism

This document summarises the theoretical framework that underpins SPIRE's
computational engine. Each section corresponds to a core module in the Rust
kernel (`spire-kernel`).

---

## 1. Formal Ontology of Physical Entities

Elementary fields are modelled as irreducible representations of the Poincaré
group and the Standard Model gauge group:

$$
G_{\text{SM}} = SU(3)_C \times SU(2)_L \times U(1)_Y
$$

Each field is defined by its invariant mass $m$, spin $J$ (stored as $2J$ to
avoid fractions), and a vector of quantum numbers:

| Quantum Number | Storage Convention | Example |
|---|---|---|
| Electric charge $Q$ | $3Q$ (integer) | $e^-$: $-3$ |
| Weak isospin $T_3$ | $2T_3$ | $e^-$: $-1$ |
| Hypercharge $Y$ | $3Y$ | $e^-$: $-3$ |
| Baryon number $B$ | $3B$ | $u$: $1$ |
| Lepton numbers | $[L_e, L_\mu, L_\tau]$ | $e^-$: $[1,0,0]$ |

The **Gell-Mann–Nishijima relation** is used as a consistency check:

$$
Q = T_3 + \frac{Y}{2} \quad\Longleftrightarrow\quad 3Y = 2 \cdot 3Q - 3 \cdot 2T_3
$$

### On-Shell vs Off-Shell

SPIRE distinguishes between:

- **On-shell** external states (asymptotic particles satisfying $p^2 = m^2$)
- **Off-shell** internal propagators (virtual particles with $p^2 \neq m^2$)

## 2. Symmetry Groups and Conservation Laws

### Continuous Symmetries

For each reaction $A + B \to C + D$, SPIRE verifies conservation of:

- **Energy-momentum**: 4-vector conservation $p_A^\mu + p_B^\mu = p_C^\mu + p_D^\mu$
- **Electric charge**: $\sum Q_i = \sum Q_f$
- **Baryon number**: $\sum B_i = \sum B_f$
- **Lepton family numbers**: $\sum L_{\ell,i} = \sum L_{\ell,f}$ for $\ell \in \{e, \mu, \tau\}$
- **Weak isospin / Hypercharge**: checked for electromagnetic and strong interactions

### Discrete Symmetries

The engine also checks C, P, and T symmetries depending on the interaction type:

| Interaction | C | P | CP | T |
|---|---|---|---|---|
| Strong | ✓ | ✓ | ✓ | ✓ |
| Electromagnetic | ✓ | ✓ | ✓ | ✓ |
| Weak (CC) | ✗ | ✗ | ≈✓ | ≈✓ |
| Weak (NC) | ✗ | ✗ | ✗ (small) | ✗ (small) |

## 3. S-Matrix and Reaction Construction

A physical process is a transition between asymptotic states in the S-matrix:

$$
\langle f | S | i \rangle = \delta_{fi} + i (2\pi)^4 \delta^{(4)}(p_f - p_i) \, \mathcal{M}_{fi}
$$

### Reaction Validation Pipeline

1. **Kinematic check**: Is $\sqrt{s} \geq \sum m_f$ (threshold)?
2. **Conservation laws**: Are all quantum numbers conserved?
3. **Interaction identification**: Which gauge couplings mediate the process?
4. **Mediator identification**: Which boson(s) carry the quantum number difference?

### Final-State Reconstruction

Given only an initial state $|i\rangle$ and $\sqrt{s}$, SPIRE enumerates all
kinematically and dynamically allowed two-body final states by:

1. Iterating over all particle pairs $(f_1, f_2)$ from the model
2. Applying threshold and conservation filters
3. Ranking by interaction weight

## 4. Lagrangian Density and Feynman Rules

The theoretical model is specified by a set of fields and interaction terms in
the Lagrangian density $\mathcal{L}$. SPIRE parses these from TOML definitions
and derives:

- **Vertex factors** $-i g V_\mu^{abc\ldots}$: one per interaction term
- **Propagators** $\Delta_F(p)$: determined by spin and mass

| Spin | Propagator Form |
|---|---|
| 0 (scalar) | $\frac{i}{p^2 - m^2 + i\epsilon}$ |
| ½ (fermion) | $\frac{i(\not{p} + m)}{p^2 - m^2 + i\epsilon}$ |
| 1 (massive vector) | $\frac{-i(g_{\mu\nu} - p_\mu p_\nu / m^2)}{p^2 - m^2 + i\epsilon}$ |
| 1 (massless vector) | $\frac{-i g_{\mu\nu}}{p^2 + i\epsilon}$ (Feynman gauge) |

## 5. Topological Construction (Feynman Diagrams)

Diagrams are generated as directed graphs using `petgraph`:

- **Nodes**: External incoming, external outgoing, or interaction vertices
- **Edges**: Propagator lines carrying particle identity and momentum label

The generator produces all topologically distinct diagrams at a given loop order
by:

1. Enumerating valid vertex connections from the model's vertex factors
2. Filtering for momentum conservation at each vertex
3. Classifying channels ($s$, $t$, $u$) based on momentum routing
4. Computing symmetry factors for identical-particle permutations

## 6. Symbolic Amplitude Construction

For each diagram, the invariant amplitude $\mathcal{M}$ is constructed by
applying Feynman rules:

$$
\mathcal{M} = \prod_{\text{vertices}} (-ig V) \times \prod_{\text{propagators}} \Delta_F(p) \times \prod_{\text{external}} \text{(spinor/polarization)}
$$

The algebra module handles:

- **Dirac traces**: $\text{Tr}[\gamma^\mu \gamma^\nu \cdots]$
- **Lorentz contractions**: $g_{\mu\nu} T^{\mu\nu}$
- **Spinor chains**: $\bar{u}(p') \Gamma u(p)$

## 7. Kinematics

### Mandelstam Variables

For $2 \to 2$ scattering with external momenta $p_1, p_2, p_3, p_4$:

$$
s = (p_1 + p_2)^2, \quad t = (p_1 - p_3)^2, \quad u = (p_1 - p_4)^2
$$

with the constraint $s + t + u = \sum m_i^2$.

### Phase Space

The $n$-body phase space measure is:

$$
d\Phi_n = \prod_{i=1}^{n} \frac{d^3 p_i}{(2\pi)^3 \, 2E_i} \; (2\pi)^4 \delta^{(4)}\!\left(P - \sum p_i\right)
$$

SPIRE computes threshold energies, Mandelstam boundaries, and (for 3-body
decays) Dalitz plot limits.

---

*This document corresponds to the physics implemented in `spire-kernel` modules:
`ontology`, `groups`, `s_matrix`, `lagrangian`, `graph`, `algebra`, `kinematics`.*
