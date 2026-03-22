/**
 * SPIRE - Mock Backend (Simulation Mode)
 *
 * Implements the `SpireBackend` interface with static, hardcoded
 * responses that keep the UI functional when neither the Tauri native
 * process nor a WASM module is available.
 *
 * ## Purpose
 *
 * - **UI / UX development**: Designers and frontend engineers can iterate
 *   on the interface without compiling Rust or building WASM artefacts.
 * - **Graceful degradation**: If the user opens the built web assets in
 *   a plain browser, the application displays "Simulation Mode" instead
 *   of crashing with an unhandled IPC exception.
 * - **Testing harness**: Unit tests for Svelte components can inject
 *   this backend to avoid real computation.
 *
 * All returned data is physically plausible but minimal - enough to
 * populate the UI widgets without misleading the user into thinking
 * real computation occurred.  Every response carries a clear signal
 * (e.g., model name "Mock Standard Model") indicating simulation mode.
 */

import type { SpireBackend } from "./BackendInterface";
import { BackendError } from "./BackendInterface";
import type { BackendKind } from "./BackendInterface";

import type {
  Reaction,
  ReconstructedFinalState,
  TopologySet,
  AmplitudeResult,
  KinematicsReport,
  DalitzPlotData,
  TheoreticalModel,
  FeynmanDiagram,
  FeynmanNode,
  FeynmanEdge,
  UfoExportResult,
  DerivationStep,
  SpacetimeDimension,
  ObservableKind,
  SimplifiedExpressionResult,
  DimensionalCheckReport,
  AnalysisConfig,
  AnalysisResult,
  EventDisplayData,
  LagrangianExpr,
  ExternalField,
  DerivedVertexRule,
  ValidationResult,
  FieldSpin,
  GaugeSymmetry,
  FieldGaugeInfo,
  RgeFlowConfig,
  RgeFlowResult,
  SlhaDocument,
  UfoFileContents,
  UfoModel,
  CountertermResult,
  ScanConfig1D,
  ScanResult1D,
  ScanConfig2D,
  ScanResult2D,
  CalcDecayTable,
  NloConfig,
  ShowerToggleConfig,
  RelicConfig,
  RelicDensityReport,
  LatticeInputs,
  WilsonCoefficients,
  BMixingResult,
  FlavorObservableReport,
  GoodnessOfFitResult,
  ObservableFitInput,
  GlobalObservableFitResult,
} from "$lib/types/spire";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Simulate a short async delay to mimic real computation latency. */
function simulateLatency(ms: number = 50): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// ---------------------------------------------------------------------------
// MockBackend
// ---------------------------------------------------------------------------

export class MockBackend implements SpireBackend {
  public readonly kind: BackendKind = "mock";

  // ── Model Loading ──────────────────────────────────────────────────────

  async loadModel(
    _particlesToml: string,
    _verticesToml: string,
    modelName?: string,
  ): Promise<TheoreticalModel> {
    await simulateLatency();
    return {
      name: modelName ?? "Mock Standard Model",
      description: "Simulation-mode placeholder model - no real computation backend is connected.",
      fields: [
        {
          id: "e-",
          name: "Electron",
          symbol: "e⁻",
          mass: 0.000511,
          width: 0,
          quantum_numbers: {
            electric_charge: -3,
            weak_isospin: -1,
            hypercharge: -3,
            baryon_number: 0,
            lepton_numbers: { electron: 1, muon: 0, tau: 0 },
            spin: 1,
            parity: "Even",
            charge_conjugation: "Undefined",
            color: "Singlet",
            weak_multiplet: "DoubletDown",
          },
          interactions: ["Electromagnetic", "WeakCC", "WeakNC"],
        },
        {
          id: "gamma",
          name: "Photon",
          symbol: "γ",
          mass: 0,
          width: 0,
          quantum_numbers: {
            electric_charge: 0,
            weak_isospin: 0,
            hypercharge: 0,
            baryon_number: 0,
            lepton_numbers: { electron: 0, muon: 0, tau: 0 },
            spin: 2,
            parity: "Odd",
            charge_conjugation: "Odd",
            color: "Singlet",
            weak_multiplet: "Singlet",
          },
          interactions: ["Electromagnetic"],
        },
      ],
      terms: [],
      vertex_factors: [],
      propagators: [],
    };
  }

  // ── Reaction Construction ──────────────────────────────────────────────

  async constructReaction(
    initialIds: string[],
    finalIds: string[],
    cmsEnergy: number,
    _model: TheoreticalModel,
  ): Promise<Reaction> {
    await simulateLatency();
    const makeDummyState = (ids: string[]) => ({
      states: ids.map((id) => ({
        particle: {
          field: {
            id,
            name: id,
            symbol: id,
            mass: 0,
            width: 0,
            quantum_numbers: {
              electric_charge: 0,
              weak_isospin: 0,
              hypercharge: 0,
              baryon_number: 0,
              lepton_numbers: { electron: 0, muon: 0, tau: 0 },
              spin: 1,
              parity: "Even" as const,
              charge_conjugation: "Undefined" as const,
              color: "Singlet" as const,
              weak_multiplet: "Singlet" as const,
            },
            interactions: [],
          },
          shell: "OnShell" as const,
          helicity: null,
          chirality: null,
          is_anti: false,
        },
        four_momentum: [cmsEnergy / ids.length, 0, 0, 0] as [number, number, number, number],
        normalization: 1,
      })),
      invariant_mass_sq: cmsEnergy * cmsEnergy,
    });

    return {
      initial: makeDummyState(initialIds),
      final_state: makeDummyState(finalIds),
      is_valid: true,
      violation_diagnostics: [],
      interaction_types: ["Electromagnetic"],
      mediating_bosons: [],
      perturbative_order: 0,
    };
  }

  async reconstructReaction(
    _initialIds: string[],
    _cmsEnergy: number,
    _model: TheoreticalModel,
  ): Promise<ReconstructedFinalState[]> {
    await simulateLatency();
    return [];
  }

  // ── Feynman Diagrams ───────────────────────────────────────────────────

  async generateDiagrams(
    reaction: Reaction,
    _model: TheoreticalModel,
    _maxLoopOrder?: number,
  ): Promise<TopologySet> {
    await simulateLatency();

    const initial = reaction.initial.states;
    const final_ = reaction.final_state.states;

    const makeField = (id: string, symbol: string, mass: number, spin: number) => ({
      id, name: id, symbol, mass, width: 0,
      quantum_numbers: {
        electric_charge: 0, weak_isospin: 0, hypercharge: 0,
        baryon_number: 0, lepton_numbers: { electron: 0, muon: 0, tau: 0 },
        spin, parity: "Even" as const, charge_conjugation: "Undefined" as const,
        color: "Singlet" as const, weak_multiplet: "Singlet" as const,
      },
      interactions: ["Electromagnetic" as const],
    });

    const mediatorField = makeField("gamma", "γ", 0, 2);
    const neutralMediatorField = makeField("Z0", "Z⁰", 91.1876, 2);
    const scalarMediatorField = makeField("H", "H", 125.0, 0);

    const diagrams: FeynmanDiagram[] = [];
    let nextDiagramId = 0;

    const isAntiParticle = (fieldId: string): boolean => {
      const norm = fieldId.toLowerCase();
      return norm.includes("_bar") || norm.endsWith("+") || norm.includes("anti") || norm.includes("bar");
    };

    const edgeFor = (field: FeynmanEdge["field"], momentumLabel: string, isExternal: boolean): FeynmanEdge => {
      const normalizedId = isAntiParticle(field.id) && !field.id.toLowerCase().includes("_bar")
        ? `${field.id}_bar`
        : field.id;
      return {
        field: {
          ...field,
          id: normalizedId,
        },
        propagator: null,
        momentum_label: momentumLabel,
        is_external: isExternal,
      };
    };

    const propagatorEdge = (
      field: FeynmanEdge["field"],
      momentumLabel: string,
      expression: string,
      form: "MasslessVector" | "MassiveVector" | "Scalar" = "MasslessVector",
    ): FeynmanEdge => ({
      field,
      propagator: {
        field_id: field.id,
        spin: form === "Scalar" ? 0 : 2,
        mass: field.mass,
        width: field.width,
        expression,
        gauge_parameter: null,
        form,
      },
      momentum_label: momentumLabel,
      is_external: false,
    });

    // ── S-channel backbone (works for any 2→N) ──
    const sNodes: FeynmanNode[] = [
      ...initial.map((s, i) => ({
        id: i,
        kind: { ExternalIncoming: s.particle },
        position: [0, i * 2] as [number, number],
      })),
      {
        id: initial.length,
        kind: { InternalVertex: {
          term_id: "mock-vertex-prod",
          field_ids: [...initial.map((s) => s.particle.field.id), mediatorField.id] as string[],
          expression: "-ieγ^μ",
          coupling_value: 0.3028,
          n_legs: initial.length + 1,
        }},
        position: [2, 1] as [number, number],
      },
      {
        id: initial.length + 1,
        kind: { InternalVertex: {
          term_id: "mock-vertex-dec",
          field_ids: [mediatorField.id, ...final_.map((s) => s.particle.field.id)] as string[],
          expression: "-ieγ^μ",
          coupling_value: 0.3028,
          n_legs: final_.length + 1,
        }},
        position: [4, 1] as [number, number],
      },
      ...final_.map((s, i) => ({
        id: initial.length + 2 + i,
        kind: { ExternalOutgoing: s.particle },
        position: [6, i * 2] as [number, number],
      })),
    ];

    const vProd = initial.length;
    const vDec = initial.length + 1;

    const sEdges: [number, number, FeynmanEdge][] = [
      ...initial.map((s, i) => [
        i,
        vProd,
        edgeFor(s.particle.field, `p${i + 1}`, true),
      ] as [number, number, FeynmanEdge]),
      [
        vProd,
        vDec,
        {
          field: mediatorField,
          propagator: {
            field_id: mediatorField.id,
            spin: 2,
            mass: 0,
            width: 0,
            expression: "-ig_{μν}/q²",
            gauge_parameter: null,
            form: "MasslessVector" as const,
          },
          momentum_label: "q",
          is_external: false,
        },
      ],
      ...final_.map((s, i) => [
        vDec,
        initial.length + 2 + i,
        edgeFor(s.particle.field, `k${i + 1}`, true),
      ] as [number, number, FeynmanEdge]),
    ];

    diagrams.push({
      id: nextDiagramId++,
      nodes: sNodes,
      edges: sEdges,
      channels: ["S"],
      loop_order: "Tree",
      symmetry_factor: 1,
      is_connected: true,
      is_one_particle_irreducible: true,
    });

    // ── Richer 2→2 set: add T and U channels ──
    if (initial.length === 2 && final_.length === 2) {
      const [i1, i2] = initial;
      const [f1, f2] = final_;

      const buildTwoByTwoChannel = (
        idBase: string,
        channels: Array<"T" | "U">,
        outgoingOrder: [typeof f1, typeof f2],
      ): FeynmanDiagram => {
        const nodes: FeynmanNode[] = [
          { id: 0, kind: { ExternalIncoming: i1.particle }, position: [0, 0] as [number, number] },
          { id: 1, kind: { ExternalIncoming: i2.particle }, position: [0, 4] as [number, number] },
          {
            id: 2,
            kind: {
              InternalVertex: {
                term_id: `${idBase}-v1`,
                field_ids: [i1.particle.field.id, outgoingOrder[0].particle.field.id, mediatorField.id],
                expression: "-ieγ^μ",
                coupling_value: 0.3028,
                n_legs: 3,
              },
            },
            position: [3, 0] as [number, number],
          },
          {
            id: 3,
            kind: {
              InternalVertex: {
                term_id: `${idBase}-v2`,
                field_ids: [i2.particle.field.id, outgoingOrder[1].particle.field.id, mediatorField.id],
                expression: "-ieγ^μ",
                coupling_value: 0.3028,
                n_legs: 3,
              },
            },
            position: [3, 4] as [number, number],
          },
          { id: 4, kind: { ExternalOutgoing: outgoingOrder[0].particle }, position: [6, 0] as [number, number] },
          { id: 5, kind: { ExternalOutgoing: outgoingOrder[1].particle }, position: [6, 4] as [number, number] },
        ];

        const exchange = channels[0] === "T" ? "t" : "u";
        const edges: [number, number, FeynmanEdge][] = [
          [0, 2, edgeFor(i1.particle.field, "p1", true)],
          [1, 3, edgeFor(i2.particle.field, "p2", true)],
          [2, 3, {
            field: mediatorField,
            propagator: {
              field_id: mediatorField.id,
              spin: 2,
              mass: 0,
              width: 0,
              expression: `-ig_{μν}/${exchange}`,
              gauge_parameter: null,
              form: "MasslessVector" as const,
            },
            momentum_label: exchange,
            is_external: false,
          }],
          [2, 4, edgeFor(outgoingOrder[0].particle.field, "k1", true)],
          [3, 5, edgeFor(outgoingOrder[1].particle.field, "k2", true)],
        ];

        return {
          id: nextDiagramId++,
          nodes,
          edges,
          channels,
          loop_order: "Tree",
          symmetry_factor: 1,
          is_connected: true,
          is_one_particle_irreducible: true,
        };
      };

      diagrams.push(buildTwoByTwoChannel("mock-t", ["T"], [f1, f2]));
      diagrams.push(buildTwoByTwoChannel("mock-u", ["U"], [f2, f1]));
    }

    // ── For larger multiplicity, add a ladder-like multi-peripheral topology ──
    if (final_.length >= 3) {
      const ladderNodes: FeynmanNode[] = [];
      const ladderEdges: [number, number, FeynmanEdge][] = [];

      let nodeId = 0;
      const inNodeIds = initial.map((s, idx) => {
        const id = nodeId++;
        ladderNodes.push({
          id,
          kind: { ExternalIncoming: s.particle },
          position: [0, idx * 3] as [number, number],
        });
        return id;
      });

      const firstVertexId = nodeId++;
      ladderNodes.push({
        id: firstVertexId,
        kind: {
          InternalVertex: {
            term_id: "mock-ladder-v0",
            field_ids: [...initial.map((s) => s.particle.field.id), mediatorField.id],
            expression: "-ieγ^μ",
            coupling_value: 0.3028,
            n_legs: initial.length + 1,
          },
        },
        position: [2.2, 1.5] as [number, number],
      });

      inNodeIds.forEach((inId, idx) => {
        ladderEdges.push([inId, firstVertexId, edgeFor(initial[idx].particle.field, `p${idx + 1}`, true)]);
      });

      let prevVertexId = firstVertexId;

      final_.forEach((state, idx) => {
        const outNodeId = nodeId++;
        ladderNodes.push({
          id: outNodeId,
          kind: { ExternalOutgoing: state.particle },
          position: [7.2, idx * 2.2] as [number, number],
        });

        if (idx < final_.length - 1) {
          const nextVertexId = nodeId++;
          ladderNodes.push({
            id: nextVertexId,
            kind: {
              InternalVertex: {
                term_id: `mock-ladder-v${idx + 1}`,
                field_ids: [state.particle.field.id, mediatorField.id, final_[idx + 1].particle.field.id],
                expression: "-ieγ^μ",
                coupling_value: 0.3028,
                n_legs: 3,
              },
            },
            position: [3.4 + idx * 1.1, 1.1 + idx * 0.7] as [number, number],
          });

          ladderEdges.push([prevVertexId, outNodeId, edgeFor(state.particle.field, `k${idx + 1}`, true)]);
          ladderEdges.push([
            prevVertexId,
            nextVertexId,
            {
              field: mediatorField,
              propagator: {
                field_id: mediatorField.id,
                spin: 2,
                mass: 0,
                width: 0,
                expression: `-ig_{μν}/q_${idx + 1}²`,
                gauge_parameter: null,
                form: "MasslessVector" as const,
              },
              momentum_label: `q${idx + 1}`,
              is_external: false,
            },
          ]);

          prevVertexId = nextVertexId;
        } else {
          ladderEdges.push([prevVertexId, outNodeId, edgeFor(state.particle.field, `k${idx + 1}`, true)]);
        }
      });

      diagrams.push({
        id: nextDiagramId++,
        nodes: ladderNodes,
        edges: ladderEdges,
        channels: ["S", "T"],
        loop_order: "Tree",
        symmetry_factor: 1,
        is_connected: true,
        is_one_particle_irreducible: false,
      });
    }

    // ── Multiplicity >= 4: add split-decay tree with explicit sub-interactions ──
    if (final_.length >= 4) {
      const treeNodes: FeynmanNode[] = [];
      const treeEdges: [number, number, FeynmanEdge][] = [];

      let nodeId = 0;
      const inNodeIds = initial.map((state, idx) => {
        const id = nodeId++;
        treeNodes.push({
          id,
          kind: { ExternalIncoming: state.particle },
          position: [0.0, 1.1 + idx * 2.3] as [number, number],
        });
        return id;
      });

      const vProd = nodeId++;
      const vBranchA = nodeId++;
      const vBranchB = nodeId++;

      treeNodes.push(
        {
          id: vProd,
          kind: {
            InternalVertex: {
              term_id: "mock-tree-prod",
              field_ids: [...initial.map((s) => s.particle.field.id), mediatorField.id],
              expression: "-ieγ^μ",
              coupling_value: 0.3028,
              n_legs: initial.length + 1,
            },
          },
          position: [2.1, 2.2] as [number, number],
        },
        {
          id: vBranchA,
          kind: {
            InternalVertex: {
              term_id: "mock-tree-branch-a",
              field_ids: [mediatorField.id, neutralMediatorField.id, final_[0].particle.field.id],
              expression: "-ig_Zγ",
              coupling_value: 0.21,
              n_legs: 3,
            },
          },
          position: [4.0, 1.1] as [number, number],
        },
        {
          id: vBranchB,
          kind: {
            InternalVertex: {
              term_id: "mock-tree-branch-b",
              field_ids: [mediatorField.id, scalarMediatorField.id, final_[final_.length - 1].particle.field.id],
              expression: "-iy",
              coupling_value: 0.15,
              n_legs: 3,
            },
          },
          position: [4.0, 3.3] as [number, number],
        },
      );

      inNodeIds.forEach((inId, idx) => {
        treeEdges.push([inId, vProd, edgeFor(initial[idx].particle.field, `p${idx + 1}`, true)]);
      });

      treeEdges.push(
        [vProd, vBranchA, propagatorEdge(mediatorField, "q_a", "-ig_{μν}/q_a²", "MasslessVector")],
        [vProd, vBranchB, propagatorEdge(neutralMediatorField, "q_b", "-ig_{μν}/(q_b²-m_Z²)", "MassiveVector")],
        [vBranchA, vBranchB, propagatorEdge(scalarMediatorField, "q_h", "i/(q_h²-m_H²)", "Scalar")],
      );

      const splitIndex = Math.ceil(final_.length / 2);
      const branchAFinal = final_.slice(0, splitIndex);
      const branchBFinal = final_.slice(splitIndex);

      branchAFinal.forEach((state, idx) => {
        const outId = nodeId++;
        treeNodes.push({
          id: outId,
          kind: { ExternalOutgoing: state.particle },
          position: [7.1, 0.4 + idx * 1.1] as [number, number],
        });
        treeEdges.push([vBranchA, outId, edgeFor(state.particle.field, `k${idx + 1}`, true)]);
      });

      branchBFinal.forEach((state, idx) => {
        const outId = nodeId++;
        treeNodes.push({
          id: outId,
          kind: { ExternalOutgoing: state.particle },
          position: [7.1, 2.7 + idx * 1.1] as [number, number],
        });
        treeEdges.push([vBranchB, outId, edgeFor(state.particle.field, `k${splitIndex + idx + 1}`, true)]);
      });

      diagrams.push({
        id: nextDiagramId++,
        nodes: treeNodes,
        edges: treeEdges,
        channels: ["S", "T", "U"],
        loop_order: "Tree",
        symmetry_factor: 1,
        is_connected: true,
        is_one_particle_irreducible: false,
      });

      // Effective loop-like topology to expose non-trivial substructure.
      const loopNodes: FeynmanNode[] = [];
      const loopEdges: [number, number, FeynmanEdge][] = [];
      nodeId = 0;

      const loopInIds = initial.map((state, idx) => {
        const id = nodeId++;
        loopNodes.push({
          id,
          kind: { ExternalIncoming: state.particle },
          position: [0.0, 1.2 + idx * 2.2] as [number, number],
        });
        return id;
      });

      const vIn = nodeId++;
      const vTop = nodeId++;
      const vBottom = nodeId++;
      const vLoopA = nodeId++;
      const vLoopB = nodeId++;

      loopNodes.push(
        {
          id: vIn,
          kind: {
            InternalVertex: {
              term_id: "mock-loop-in",
              field_ids: [...initial.map((s) => s.particle.field.id), mediatorField.id],
              expression: "-ieγ^μ",
              coupling_value: 0.3028,
              n_legs: initial.length + 1,
            },
          },
          position: [2.0, 2.2] as [number, number],
        },
        {
          id: vTop,
          kind: {
            InternalVertex: {
              term_id: "mock-loop-top",
              field_ids: [mediatorField.id, neutralMediatorField.id, final_[0].particle.field.id],
              expression: "-ig_Zγ",
              coupling_value: 0.22,
              n_legs: 3,
            },
          },
          position: [4.0, 1.0] as [number, number],
        },
        {
          id: vBottom,
          kind: {
            InternalVertex: {
              term_id: "mock-loop-bottom",
              field_ids: [mediatorField.id, scalarMediatorField.id, final_[final_.length - 1].particle.field.id],
              expression: "-iy",
              coupling_value: 0.14,
              n_legs: 3,
            },
          },
          position: [4.0, 3.4] as [number, number],
        },
        {
          id: vLoopA,
          kind: {
            InternalVertex: {
              term_id: "mock-loop-a",
              field_ids: [neutralMediatorField.id, scalarMediatorField.id, mediatorField.id],
              expression: "-iλ",
              coupling_value: 0.11,
              n_legs: 3,
            },
          },
          position: [5.3, 1.8] as [number, number],
        },
        {
          id: vLoopB,
          kind: {
            InternalVertex: {
              term_id: "mock-loop-b",
              field_ids: [neutralMediatorField.id, scalarMediatorField.id, mediatorField.id],
              expression: "-iλ",
              coupling_value: 0.11,
              n_legs: 3,
            },
          },
          position: [5.3, 2.6] as [number, number],
        },
      );

      loopInIds.forEach((inId, idx) => {
        loopEdges.push([inId, vIn, edgeFor(initial[idx].particle.field, `p${idx + 1}`, true)]);
      });

      loopEdges.push(
        [vIn, vTop, propagatorEdge(mediatorField, "q_t", "-ig_{μν}/q_t²", "MasslessVector")],
        [vIn, vBottom, propagatorEdge(neutralMediatorField, "q_b", "-ig_{μν}/(q_b²-m_Z²)", "MassiveVector")],
        [vTop, vLoopA, propagatorEdge(neutralMediatorField, "ℓ_1", "-ig_{μν}/(ℓ_1²-m_Z²)", "MassiveVector")],
        [vLoopA, vBottom, propagatorEdge(scalarMediatorField, "ℓ_2", "i/(ℓ_2²-m_H²)", "Scalar")],
        [vBottom, vLoopB, propagatorEdge(neutralMediatorField, "ℓ_3", "-ig_{μν}/(ℓ_3²-m_Z²)", "MassiveVector")],
        [vLoopB, vTop, propagatorEdge(scalarMediatorField, "ℓ_4", "i/(ℓ_4²-m_H²)", "Scalar")],
      );

      final_.forEach((state, idx) => {
        const outId = nodeId++;
        const attachTop = idx % 2 === 0;
        loopNodes.push({
          id: outId,
          kind: { ExternalOutgoing: state.particle },
          position: [7.2, 0.4 + idx * 0.95] as [number, number],
        });
        loopEdges.push([
          attachTop ? vTop : vBottom,
          outId,
          edgeFor(state.particle.field, `k${idx + 1}`, true),
        ]);
      });

      diagrams.push({
        id: nextDiagramId++,
        nodes: loopNodes,
        edges: loopEdges,
        channels: ["S", "T"],
        loop_order: "OneLoop",
        symmetry_factor: 1,
        is_connected: true,
        is_one_particle_irreducible: true,
      });
    }

    // ── Multiplicity >= 5: dense web/bridge topology (high-complexity mock) ──
    if (final_.length >= 5) {
      const webNodes: FeynmanNode[] = [];
      const webEdges: [number, number, FeynmanEdge][] = [];

      let nodeId = 0;
      const inIds = initial.map((state, idx) => {
        const id = nodeId++;
        webNodes.push({
          id,
          kind: { ExternalIncoming: state.particle },
          position: [0.0, 1.0 + idx * 2.6] as [number, number],
        });
        return id;
      });

      const v0 = nodeId++;
      const v1 = nodeId++;
      const v2 = nodeId++;
      const v3 = nodeId++;
      const v4 = nodeId++;
      const v5 = nodeId++;

      webNodes.push(
        {
          id: v0,
          kind: {
            InternalVertex: {
              term_id: "mock-web-v0",
              field_ids: [...initial.map((s) => s.particle.field.id), mediatorField.id],
              expression: "-ieγ^μ",
              coupling_value: 0.3028,
              n_legs: initial.length + 1,
            },
          },
          position: [1.9, 2.2] as [number, number],
        },
        {
          id: v1,
          kind: {
            InternalVertex: {
              term_id: "mock-web-v1",
              field_ids: [mediatorField.id, neutralMediatorField.id, scalarMediatorField.id],
              expression: "-ig_{mix}",
              coupling_value: 0.23,
              n_legs: 3,
            },
          },
          position: [3.0, 1.1] as [number, number],
        },
        {
          id: v2,
          kind: {
            InternalVertex: {
              term_id: "mock-web-v2",
              field_ids: [mediatorField.id, neutralMediatorField.id, scalarMediatorField.id],
              expression: "-ig_{mix}",
              coupling_value: 0.21,
              n_legs: 3,
            },
          },
          position: [3.0, 3.3] as [number, number],
        },
        {
          id: v3,
          kind: {
            InternalVertex: {
              term_id: "mock-web-v3",
              field_ids: [neutralMediatorField.id, scalarMediatorField.id, mediatorField.id],
              expression: "-iλ_1",
              coupling_value: 0.12,
              n_legs: 3,
            },
          },
          position: [4.4, 0.7] as [number, number],
        },
        {
          id: v4,
          kind: {
            InternalVertex: {
              term_id: "mock-web-v4",
              field_ids: [neutralMediatorField.id, scalarMediatorField.id, mediatorField.id],
              expression: "-iλ_2",
              coupling_value: 0.10,
              n_legs: 3,
            },
          },
          position: [4.4, 2.2] as [number, number],
        },
        {
          id: v5,
          kind: {
            InternalVertex: {
              term_id: "mock-web-v5",
              field_ids: [neutralMediatorField.id, scalarMediatorField.id, mediatorField.id],
              expression: "-iλ_3",
              coupling_value: 0.11,
              n_legs: 3,
            },
          },
          position: [4.4, 3.7] as [number, number],
        },
      );

      inIds.forEach((inId, idx) => {
        webEdges.push([inId, v0, edgeFor(initial[idx].particle.field, `p${idx + 1}`, true)]);
      });

      webEdges.push(
        [v0, v1, propagatorEdge(mediatorField, "q1", "-ig_{μν}/q1²", "MasslessVector")],
        [v0, v2, propagatorEdge(neutralMediatorField, "q2", "-ig_{μν}/(q2²-m_Z²)", "MassiveVector")],
        [v1, v3, propagatorEdge(neutralMediatorField, "ℓ1", "-ig_{μν}/(ℓ1²-m_Z²)", "MassiveVector")],
        [v1, v4, propagatorEdge(scalarMediatorField, "ℓ2", "i/(ℓ2²-m_H²)", "Scalar")],
        [v2, v4, propagatorEdge(mediatorField, "ℓ3", "-ig_{μν}/ℓ3²", "MasslessVector")],
        [v2, v5, propagatorEdge(scalarMediatorField, "ℓ4", "i/(ℓ4²-m_H²)", "Scalar")],
        [v3, v4, propagatorEdge(mediatorField, "b1", "-ig_{μν}/b1²", "MasslessVector")],
        [v4, v5, propagatorEdge(neutralMediatorField, "b2", "-ig_{μν}/(b2²-m_Z²)", "MassiveVector")],
        [v3, v5, propagatorEdge(scalarMediatorField, "b3", "i/(b3²-m_H²)", "Scalar")],
      );

      final_.forEach((state, idx) => {
        const outId = nodeId++;
        const y = 0.35 + idx * 0.9;
        webNodes.push({
          id: outId,
          kind: { ExternalOutgoing: state.particle },
          position: [7.2, y] as [number, number],
        });

        const attach = idx % 3 === 0 ? v3 : (idx % 3 === 1 ? v4 : v5);
        webEdges.push([attach, outId, edgeFor(state.particle.field, `k${idx + 1}`, true)]);
      });

      diagrams.push({
        id: nextDiagramId++,
        nodes: webNodes,
        edges: webEdges,
        channels: ["S", "T", "U"],
        loop_order: "OneLoop",
        symmetry_factor: 1,
        is_connected: true,
        is_one_particle_irreducible: true,
      });
    }

    return {
      reaction_id: "mock-reaction",
      max_loop_order: "Tree",
      diagrams,
      count: diagrams.length,
    };
  }

  // ── Amplitude ──────────────────────────────────────────────────────────

  async deriveAmplitude(diagram: FeynmanDiagram): Promise<AmplitudeResult> {
    await simulateLatency();

    // Generate a plausible LaTeX amplitude based on the diagram channel.
    const channel = diagram.channels[0] ?? "S";
    const expressions: Record<string, string> = {
      S: "i\\mathcal{M}_s = \\frac{-ie^2}{s} \\bar{u}(k_1)\\gamma^\\mu v(k_2) \\bar{v}(p_2)\\gamma_\\mu u(p_1)",
      T: "i\\mathcal{M}_t = \\frac{-ie^2}{t} \\bar{u}(k_1)\\gamma^\\mu u(p_1) \\bar{v}(p_2)\\gamma_\\mu v(k_2)",
      U: "i\\mathcal{M}_u = \\frac{-ie^2}{u} \\bar{u}(k_2)\\gamma^\\mu u(p_1) \\bar{v}(p_2)\\gamma_\\mu v(k_1)",
    };

    return {
      diagram_id: diagram.id,
      expression: expressions[channel] ?? expressions.S,
      couplings: ["e"],
      momenta_labels: ["p1", "p2", "k1", "k2"],
    };
  }

  async deriveAmplitudeSteps(
    _diagram: FeynmanDiagram,
    _dim?: SpacetimeDimension,
  ): Promise<DerivationStep[]> {
    await simulateLatency();
    return [
      {
        label: "Mock Step",
        description: "Placeholder derivation step - no computation backend connected.",
        expression: { Scalar: 0 },
        latex: "\\mathcal{M} = 0",
      },
    ];
  }

  async simplifyExpression(
    _diagram: FeynmanDiagram,
    _dim?: SpacetimeDimension,
    observable: ObservableKind = "Amplitude",
  ): Promise<SimplifiedExpressionResult> {
    await simulateLatency();
    const dimension_check: DimensionalCheckReport = {
      observable,
      expected_mass_dimension: observable === "CrossSection" ? -2 : (observable === "DecayWidth" ? 1 : 0),
      inferred_mass_dimension: 0,
      is_consistent: observable !== "CrossSection" && observable !== "DecayWidth",
      message: "Mock dimensional check (simulation mode).",
      diagnostics: ["No real CAS backend connected; result is illustrative."],
    };
    return {
      original_latex: "\\mathcal{M}_{\\text{mock}}",
      simplified_expression: { Scalar: 0 },
      simplified_latex: "0",
      applied_rules: ["CollectTermsRule"],
      dimension_check,
    };
  }

  async verifyDimensions(
    _diagram: FeynmanDiagram,
    _dim?: SpacetimeDimension,
    observable: ObservableKind = "Amplitude",
  ): Promise<DimensionalCheckReport> {
    await simulateLatency();
    const expected = observable === "CrossSection" ? -2 : (observable === "DecayWidth" ? 1 : 0);
    return {
      observable,
      expected_mass_dimension: expected,
      inferred_mass_dimension: 0,
      is_consistent: expected === 0,
      message: "Mock dimensional check (simulation mode).",
      diagnostics: ["No real CAS backend connected; result is illustrative."],
    };
  }

  async exportAmplitudeLatex(_diagram: FeynmanDiagram): Promise<string> {
    await simulateLatency();
    return "i\\mathcal{M} = 0 \\quad \\text{(simulation mode)}";
  }

  // ── Kinematics ─────────────────────────────────────────────────────────

  async computeKinematics(
    finalMasses: number[],
    cmsEnergy: number,
    _targetMass?: number,
    _externalMasses?: [number, number, number, number],
  ): Promise<KinematicsReport> {
    await simulateLatency();
    const totalMass = finalMasses.reduce((a, b) => a + b, 0);
    return {
      threshold: {
        threshold_energy: totalMass,
        lab_energy: null,
        final_masses: finalMasses,
      },
      is_allowed: cmsEnergy >= totalMass,
      phase_space: {
        n_final: finalMasses.length,
        n_variables: 3 * finalMasses.length - 4,
        total_energy_cm: cmsEnergy,
        final_masses: finalMasses,
        measure_expression: "dΦ (mock)",
      },
      mandelstam_boundaries: null,
    };
  }

  async computeDalitzData(
    motherMass: number,
    mA: number,
    mB: number,
    mC: number,
    _nPoints: number = 3000,
  ): Promise<DalitzPlotData> {
    await simulateLatency();
    const mABmin = (mA + mB) ** 2;
    const mABmax = (motherMass - mC) ** 2;
    const mBCmin = (mB + mC) ** 2;
    const mBCmax = (motherMass - mA) ** 2;
    return {
      boundaries: {
        mother_mass: motherMass,
        daughter_masses: [mA, mB, mC],
        m_ab_sq_min: mABmin,
        m_ab_sq_max: mABmax,
        m_bc_sq_min: mBCmin,
        m_bc_sq_max: mBCmax,
      },
      points: [],
      n_grid: 0,
    };
  }

  // ── Export ─────────────────────────────────────────────────────────────

  async exportModelUfo(_model: TheoreticalModel): Promise<UfoExportResult> {
    await simulateLatency();
    return {
      "__init__.py": "# Mock UFO model (simulation mode)\n",
      "particles.py": "# No particles - simulation mode\n",
      "parameters.py": "# No parameters - simulation mode\n",
    };
  }

  // ── Analysis Pipeline ──────────────────────────────────────────────────

  async runAnalysis(config: AnalysisConfig): Promise<AnalysisResult> {
    await simulateLatency(100);
    return {
      histograms: config.plots.map((p) => ({
        name: p.name,
        bin_edges: Array.from({ length: p.n_bins + 1 }, (_, i) =>
          p.min + (i * (p.max - p.min)) / p.n_bins,
        ),
        bin_contents: new Array(p.n_bins).fill(0),
        bin_errors: new Array(p.n_bins).fill(0),
        underflow: 0,
        overflow: 0,
        entries: 0,
        mean: 0,
      })),
      histograms_2d: (config.plots_2d ?? []).map((p) => ({
        name: p.name,
        x_bin_edges: Array.from({ length: p.nx + 1 }, (_, i) =>
          p.x_min + (i * (p.x_max - p.x_min)) / p.nx,
        ),
        y_bin_edges: Array.from({ length: p.ny + 1 }, (_, i) =>
          p.y_min + (i * (p.y_max - p.y_min)) / p.ny,
        ),
        bin_contents: new Array(p.nx * p.ny).fill(0),
        nx: p.nx,
        ny: p.ny,
        entries: 0,
        total_weight: 0,
      })),
      cross_section: 0,
      cross_section_error: 0,
      events_generated: 0,
      events_passed: 0,
    };
  }

  async computeChiSquare(
    _theoryBinContents: number[],
    _theoryBinEdges: number[],
    _expCsv: string,
    _expLabel: string,
  ): Promise<GoodnessOfFitResult> {
    await simulateLatency();
    return {
      chi_square: 0,
      ndf: 0,
      chi_square_reduced: 0,
      p_value: 1,
      pulls: [],
    };
  }

  async computeGlobalFit(
    observables: ObservableFitInput[],
    nParams: number,
  ): Promise<GlobalObservableFitResult> {
    await simulateLatency();
    return {
      observables: observables.map((o) => ({
        observable: o.observable,
        result: {
          chi_square: 0,
          ndf: 0,
          chi_square_reduced: 0,
          p_value: 1,
          pulls: [],
        },
      })),
      chi_square_global: 0,
      ndf_total: 0,
      n_params: nParams,
      chi_square_reduced: 0,
      p_value: 1,
    };
  }

  // ── Scripting ──────────────────────────────────────────────────────────

  async validateScript(_script: string): Promise<void> {
    await simulateLatency();
    // Always passes in mock mode.
  }

  async testObservableScript(_script: string): Promise<number> {
    await simulateLatency();
    return 0.0;
  }

  async testCutScript(_script: string): Promise<boolean> {
    await simulateLatency();
    return true;
  }

  // ── 3D Event Display ───────────────────────────────────────────────────

  async generateDisplayEvent(
    cmsEnergy: number,
    _finalMasses: number[],
    _detectorPreset: string,
    _particleKinds?: string[] | null,
  ): Promise<EventDisplayData> {
    await simulateLatency();
    return {
      jets: [],
      electrons: [],
      muons: [],
      photons: [],
      met: { direction: { x: 0, y: 0, z: 0 }, magnitude: 0 },
      cms_energy: cmsEnergy,
    };
  }

  async generateDisplayBatch(
    cmsEnergy: number,
    _finalMasses: number[],
    _detectorPreset: string,
    _batchSize: number,
    _particleKinds?: string[] | null,
  ): Promise<EventDisplayData[]> {
    await simulateLatency();
    return [
      {
        jets: [],
        electrons: [],
        muons: [],
        photons: [],
        met: { direction: { x: 0, y: 0, z: 0 }, magnitude: 0 },
        cms_energy: cmsEnergy,
      },
    ];
  }

  // ── Lagrangian Workbench ───────────────────────────────────────────────

  async parseLagrangianTerm(
    _input: string,
    _knownFields: Record<string, FieldSpin>,
  ): Promise<LagrangianExpr> {
    await simulateLatency();
    return { RealConstant: 0 };
  }

  async deriveVertexRuleFromAst(
    _input: string,
    _knownFields: Record<string, FieldSpin>,
    externalFields: ExternalField[],
  ): Promise<DerivedVertexRule> {
    await simulateLatency();
    return {
      external_fields: externalFields,
      residual_expr: { RealConstant: 0 },
      latex: "0 \\quad \\text{(mock)}",
      symmetry_factor: 1,
      grassmann_sign: 1,
      n_legs: externalFields.length,
    };
  }

  async validateLagrangianTerm(
    _input: string,
    _knownFields: Record<string, FieldSpin>,
    _gaugeSymmetry?: GaugeSymmetry | null,
    _fieldGaugeInfo?: Record<string, FieldGaugeInfo>,
  ): Promise<ValidationResult> {
    await simulateLatency();
    return {
      is_lorentz_scalar: true,
      is_gauge_singlet: true,
      is_hermitian: true,
      is_renormalisable: true,
      mass_dimension: 4,
      messages: [
        {
          severity: "info",
          check: "simulation",
          message: "Mock validation - no computation backend connected.",
        },
      ],
    };
  }

  async runRgeFlow(config: RgeFlowConfig): Promise<RgeFlowResult> {
    await simulateLatency();
    const n = config.n_points;
    const muVals = Array.from({ length: n }, (_, i) =>
      config.mu_min + (i * (config.mu_max - config.mu_min)) / (n - 1),
    );
    return {
      coupling_name: config.coupling_name,
      mu_values: muVals,
      coupling_values: muVals.map(() => config.initial_value),
    };
  }

  // ── External Theory Bridge ─────────────────────────────────────────────

  async importSlhaString(_slhaText: string): Promise<SlhaDocument> {
    await simulateLatency();
    return { blocks: {}, decays: {} };
  }

  async importUfoModel(
    _fileContents: UfoFileContents,
    modelName: string,
  ): Promise<[UfoModel, TheoreticalModel]> {
    await simulateLatency();
    const ufoModel: UfoModel = {
      particles: [],
      vertices: [],
      couplings: [],
      parameters: [],
      lorentz_structures: [],
    };
    const thModel: TheoreticalModel = {
      name: `${modelName} (mock)`,
      description: "Imported via simulation mode - no computation backend.",
      fields: [],
      terms: [],
      vertex_factors: [],
      propagators: [],
    };
    return [ufoModel, thModel];
  }

  async deriveCounterterms(
    _input: string,
    _knownFields: Record<string, FieldSpin>,
    _externalFields: ExternalField[],
  ): Promise<CountertermResult> {
    await simulateLatency();
    return {
      tree_level_expr: { RealConstant: 0 },
      counterterms: [],
      counterterm_rules: [],
      renorm_constants: [],
    };
  }

  async runParameterScan1D(config: ScanConfig1D): Promise<ScanResult1D> {
    await simulateLatency();
    const n = config.variable.steps;
    const xs = Array.from({ length: n }, (_, i) =>
      config.variable.min + (i / (n - 1)) * (config.variable.max - config.variable.min),
    );
    return {
      variable: config.variable,
      x_values: xs,
      y_values: xs.map((x) => 100.0 / (x * x)),
      y_errors: xs.map((x) => 10.0 / (x * x)),
    };
  }

  async runParameterScan2D(config: ScanConfig2D): Promise<ScanResult2D> {
    await simulateLatency();
    const nx = config.variable_x.steps;
    const ny = config.variable_y.steps;
    const xs = Array.from({ length: nx }, (_, i) =>
      config.variable_x.min + (i / (nx - 1)) * (config.variable_x.max - config.variable_x.min),
    );
    const ys = Array.from({ length: ny }, (_, j) =>
      config.variable_y.min + (j / (ny - 1)) * (config.variable_y.max - config.variable_y.min),
    );
    const zs = xs.flatMap((x) => ys.map((y) => 100.0 / (x + y)));
    return {
      variable_x: config.variable_x,
      variable_y: config.variable_y,
      x_values: xs,
      y_values: ys,
      z_values: zs,
      z_errors: zs.map((z) => z * 0.1),
    };
  }

  async calculateDecayTable(_model: TheoreticalModel, particleId: string): Promise<CalcDecayTable> {
    await simulateLatency();
    return {
      parent_id: particleId,
      parent_name: particleId === "Z0" ? "Z boson" : particleId,
      parent_mass: particleId === "Z0" ? 91.1876 : 125.1,
      total_width: particleId === "Z0" ? 2.4952 : 0.00407,
      lifetime_seconds: particleId === "Z0" ? 2.6379e-25 : 1.617e-22,
      channels: [
        {
          final_state: ["e-", "e-"],
          final_state_names: ["electron", "electron"],
          partial_width: 0.08391,
          branching_ratio: 0.03363,
          vertex_id: "nc_eez",
        },
        {
          final_state: ["mu-", "mu-"],
          final_state_names: ["muon", "muon"],
          partial_width: 0.08391,
          branching_ratio: 0.03366,
          vertex_id: "nc_mmz",
        },
      ],
    };
  }

  async exportDecaySlha(_model: TheoreticalModel, particleId: string, pdgCode: number): Promise<string> {
    await simulateLatency();
    return `DECAY ${pdgCode} 2.4952E+00 # ${particleId}\n#  BR         NDA  ID1  ID2\n   3.363E-02  2    11   -11  # e- e+\n   3.366E-02  2    13   -13  # mu- mu+\n`;
  }

  async configureNlo(_config: NloConfig): Promise<void> {
    await simulateLatency();
    // Mock: NLO configuration accepted (no-op in simulation mode)
  }

  async configureShower(_config: ShowerToggleConfig): Promise<void> {
    await simulateLatency();
    // Mock: Shower configuration accepted (no-op in simulation mode)
  }

  async generateMathematicalProof(
    _diagram: FeynmanDiagram,
    processLabel: string,
    _dim: SpacetimeDimension,
  ): Promise<string> {
    await simulateLatency();
    return `\\documentclass[aps,prd,twocolumn,superscriptaddress]{revtex4-2}
\\usepackage{amsmath}
\\begin{document}
\\title{Mock Proof: ${processLabel}}
\\author{SPIRE}
\\maketitle
\\section{Placeholder}
This is a mock proof document for ${processLabel}.
\\end{document}
`;
  }

  async computeProvenanceHash(
    _model: TheoreticalModel,
    _reaction: Reaction | null,
    _cmsEnergy: number,
    _numEvents: number,
    _seed: number,
  ): Promise<{ sha256: string; payload: string }> {
    await simulateLatency();
    return {
      sha256: "0".repeat(64),
      payload: JSON.stringify({ mock: true, version: "0.1.0" }),
    };
  }

  async loadProvenanceState(
    payload: string,
  ): Promise<Record<string, unknown>> {
    await simulateLatency();
    return JSON.parse(payload);
  }

  async validateSessionIntegrity(
    payload: string,
  ): Promise<import("$lib/types/spire").SessionIntegrityValidationResult> {
    await simulateLatency();
    return { ok: true, state: JSON.parse(payload) };
  }

  async calculateRelicDensity(config: RelicConfig): Promise<RelicDensityReport> {
    await simulateLatency();
    const evolution = [];
    for (let i = 0; i < 100; i++) {
      const x = Math.exp(Math.log(config.x_start || 1) + (i / 99) * (Math.log(config.x_end || 1000) - Math.log(config.x_start || 1)));
      const y_eq = 0.003 * Math.exp(-x);
      const y = x < 22 ? y_eq : 4e-11;
      evolution.push({ x, y, y_eq });
    }
    return {
      omega_h2: 0.118,
      x_freeze_out: 22.0,
      y_infinity: 4e-11,
      evolution,
      planck_omega_h2: 0.120,
      classification: "compatible",
      dm_mass: config.dm_mass,
      sigma_v: config.sigma_v_a,
    };
  }

  async calculateBMixing(_lattice: LatticeInputs): Promise<BMixingResult> {
    await simulateLatency();
    return {
      delta_m_d: 0.51,
      delta_m_s: 17.8,
      exp_delta_m_d: 0.5065,
      exp_delta_m_s: 17.765,
    };
  }

  async calculateBToKll(
    q2Min: number,
    q2Max: number,
    _wilsonCoeffs: WilsonCoefficients,
    _lattice: LatticeInputs,
    nPoints?: number,
  ): Promise<FlavorObservableReport> {
    await simulateLatency();
    const n = nPoints ?? 100;
    const spectrum = [];
    for (let i = 0; i < n; i++) {
      const q2 = q2Min + (i / (n - 1)) * (q2Max - q2Min);
      // Mock: bell-shaped curve peaking around q² ≈ 4 GeV²
      const dgamma = 1e-19 * Math.exp(-0.1 * (q2 - 4) * (q2 - 4));
      spectrum.push({ q2, dgamma_dq2: dgamma });
    }
    return {
      delta_m_d: 0.51,
      delta_m_s: 17.8,
      branching_ratio: 4.7e-7,
      differential_spectrum: spectrum,
      q2_min: q2Min,
      q2_max: q2Max,
      exp_delta_m_d: 0.5065,
      exp_delta_m_s: 17.765,
    };
  }

  async queryHardwareBackends(): Promise<{
    gpu_feature_compiled: boolean;
    gpu_adapter_available: boolean;
    adapter_name: string;
    cpu_backend: string;
    gpu_backend: string;
  }> {
    await simulateLatency();
    return {
      gpu_feature_compiled: false,
      gpu_adapter_available: false,
      adapter_name: "",
      cpu_backend: "CPU (Rayon thread-parallel)",
      gpu_backend: "GPU (WebGPU compute shader)",
    };
  }

  async loadPlugin(path: string): Promise<{
    name: string;
    version: string;
    description: string;
    author: string;
    capabilities: string[];
    enabled: boolean;
  }> {
    await simulateLatency();
    return {
      name: path.split("/").pop()?.replace(".wasm", "") ?? "mock-plugin",
      version: "0.1.0",
      description: "Mock plugin for development",
      author: "SPIRE Mock",
      capabilities: ["KinematicCut"],
      enabled: true,
    };
  }

  async listActivePlugins(): Promise<Array<{
    name: string;
    version: string;
    description: string;
    author: string;
    capabilities: string[];
    enabled: boolean;
  }>> {
    await simulateLatency();
    return [];
  }

  async unloadPlugin(_name: string): Promise<void> {
    await simulateLatency();
  }

  async startMcmcFit(_request: import("$lib/types/spire").McmcFitRequest): Promise<void> {
    await simulateLatency();
  }

  async getMcmcStatus(_includeSamples: boolean): Promise<import("$lib/types/spire").McmcFitStatus> {
    await simulateLatency();
    return {
      status: {
        current_step: 0,
        total_steps: 0,
        acceptance_fraction: 0.0,
        running: false,
        stopped: false,
      },
      flat_samples: null,
      param_names: [],
    };
  }

  async stopMcmcFit(): Promise<void> {
    await simulateLatency();
  }

  // ────────────────────────────────────────────────────────────────────
  // PDG Integration (Phase 73)
  // ────────────────────────────────────────────────────────────────────

  async pdgGetMetadata(): Promise<import("$lib/types/spire").PdgMetadata> {
    await simulateLatency();
    return {
      edition: "PDG 2025",
      version: "0.2.2 (Mock)",
      timestamp: "2025-03-20T00:00:00Z",
      source_files: ["mock-database.sqlite"],
    };
  }

  async pdgGetCacheDiagnostics(): Promise<import("$lib/types/spire").PdgCacheDiagnostics> {
    await simulateLatency();
    return {
      particle_records: { hits: 0, misses: 0, evictions: 0, size: 0, capacity: 500, hit_rate: 0 },
      decay_tables: { hits: 0, misses: 0, evictions: 0, size: 0, capacity: 200, hit_rate: 0 },
      id_resolution: { hits: 0, misses: 0, evictions: 0, size: 0, capacity: 2000, hit_rate: 0 },
      total_hits: 0,
      total_misses: 0,
      total_evictions: 0,
      total_entries: 0,
      db_queries: 0,
      db_average_latency_us: 0,
      db_last_latency_us: 0,
    };
  }

  async pdgGetLiveApiSettings(): Promise<import("$lib/types/spire").PdgLiveApiSettings> {
    await simulateLatency();
    return {
      enabled: false,
      base_url: "https://pdgapi.lbl.gov",
      requests_per_second: 5,
      burst_capacity: 2,
      max_retries: 3,
      request_timeout_ms: 2500,
    };
  }

  async pdgSetLiveApiSettings(
    settings: import("$lib/types/spire").PdgLiveApiSettings,
  ): Promise<import("$lib/types/spire").PdgLiveApiSettings> {
    await simulateLatency();
    return {
      enabled: settings.enabled,
      base_url: settings.base_url ?? "https://pdgapi.lbl.gov",
      requests_per_second: settings.requests_per_second ?? 5,
      burst_capacity: settings.burst_capacity ?? 2,
      max_retries: settings.max_retries ?? 3,
      request_timeout_ms: settings.request_timeout_ms ?? 2500,
    };
  }

  async pdgGetNetworkDiagnostics(): Promise<import("$lib/types/spire").PdgNetworkDiagnostics> {
    await simulateLatency();
    return {
      total_requests: 0,
      throttled_requests: 0,
      responses_429: 0,
      responses_503: 0,
      retries: 0,
      queue_depth: 0,
      queue_depth_peak: 0,
    };
  }

  async pdgLookupParticleByMcid(mcid: number): Promise<import("$lib/types/spire").PdgParticleRecord> {
    await simulateLatency();
    if (mcid === 11) {
      return {
        pdg_id: 11,
        label: "e-",
        mass: { kind: "exact", value: 0.000510999 },
        width: undefined,
        lifetime: undefined,
        branching_fractions: [],
        quantum_numbers: { charge: -1, spin_j: "1/2", parity: "Odd" },
        provenance: {
          edition: "PDG 2025",
          source_id: "mock",
          origin: undefined,
          fingerprint: "mock-v1",
        },
      };
    }
    throw new Error(`Mock: particle MCID ${mcid} not in mock database`);
  }

  async pdgLookupParticleByPdgid(pdgid: string): Promise<import("$lib/types/spire").PdgParticleRecord> {
    if (pdgid.toLowerCase().includes("electron") || pdgid === "e-") {
      return this.pdgLookupParticleByMcid(11);
    }
    throw new Error(`Mock: particle "${pdgid}" not in mock database`);
  }

  async pdgGetParticleProperties(mcid: number): Promise<import("$lib/types/spire").PdgParticleRecord> {
    return this.pdgLookupParticleByMcid(mcid);
  }

  async pdgGetDecayTable(
    mcid: number,
    _policy: import("$lib/types/spire").PdgExtractionPolicy,
  ): Promise<import("$lib/types/spire").PdgDecayTable> {
    await simulateLatency();
    if (mcid === 11) {
      return { parent_pdg_id: 11, channels: [], edition: "PDG 2025" };
    }
    throw new Error(`Mock: no decay table for MCID ${mcid}`);
  }

  async pdgSyncModel(
    model: TheoreticalModel,
    _options?: import("$lib/types/spire").PdgSyncOptions,
  ): Promise<TheoreticalModel> {
    await simulateLatency();
    return model;
  }

  async pdgBeginCatalogStream(): Promise<string> {
    await simulateLatency();
    return `mock-stream-${Date.now()}`;
  }

  async pdgCancelCatalogStream(_requestId: string): Promise<void> {
    await simulateLatency();
  }

  async pdgSearchIdentifiersChunked(
    query: string,
    offset: number,
    limit: number,
    requestId?: string | null,
  ): Promise<import("$lib/types/spire").PdgCatalogChunk> {
    const all = await this.pdgSearchIdentifiers(query);
    const records = all.slice(offset, offset + limit);
    return {
      request_id: requestId ?? null,
      offset,
      limit,
      total: all.length,
      done: offset + records.length >= all.length,
      cancelled: false,
      records,
    };
  }

  async pdgSearchIdentifiers(query: string): Promise<import("$lib/types/spire").PdgParticleRecord[]> {
    await simulateLatency();
    if (query.toLowerCase().includes("electron") || query === "e-") {
      return [await this.pdgLookupParticleByMcid(11)];
    }
    return [];
  }
}
