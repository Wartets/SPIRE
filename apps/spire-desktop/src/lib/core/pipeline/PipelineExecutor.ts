import {
  constructReaction,
  loadModel,
  runAnalysis,
  testObservableScript,
} from "$lib/api";
import { executeWorkerTask } from "$lib/workers/executeWorkerTask";
import type {
  AnalysisResult,
  Reaction,
  TheoreticalModel,
} from "$lib/types/spire";
import type {
  PipelineEdge,
  PipelineGraph,
  PipelineNode,
  PipelineNodeType,
  PipelinePort,
} from "$lib/core/pipeline/graph";
import {
  TopologyError,
  markStale,
  topologicalSortNodeIdsOrThrow,
} from "$lib/core/pipeline/graphUtils";

export type PipelineNodeExecutionStatus =
  | "idle"
  | "pending"
  | "running"
  | "completed"
  | "error"
  | "stale";

export type PipelineEdgeExecutionStatus =
  | "idle"
  | "running"
  | "completed"
  | "error"
  | "stale";

export interface NodeExecutionState {
  status: PipelineNodeExecutionStatus;
  startedAt?: string;
  completedAt?: string;
  durationMs?: number;
  error?: string;
  cached?: boolean;
}

export interface EdgeExecutionState {
  status: PipelineEdgeExecutionStatus;
  updatedAt: string;
  error?: string;
}

export interface PipelineRunSnapshot {
  runId: string;
  isRunning: boolean;
  startedAt: string | null;
  completedAt: string | null;
  nodeStates: Record<string, NodeExecutionState>;
  edgeStates: Record<string, EdgeExecutionState>;
  nodeOutputCache: Record<string, Record<string, unknown>>;
  edgePayloads: Record<string, unknown>;
  staleNodeIds: string[];
  lastError?: string;
}

export interface PipelineBaseInputs {
  particlesToml: string;
  verticesToml: string;
  modelName: string;
}

export interface PipelineExecutorContext {
  graph: PipelineGraph;
  node: PipelineNode;
  inputsByPortKey: Record<string, unknown>;
  inputsByPortId: Record<string, unknown>;
  baseInputs: PipelineBaseInputs;
  previousNodeOutputs: Record<string, unknown>;
}

export interface PipelineNodeExecutionResult {
  outputs: Record<string, unknown>;
  summary?: string;
}

export type PipelineNodeHandler = (
  context: PipelineExecutorContext,
) => Promise<PipelineNodeExecutionResult>;

export type PipelineNodeHandlerRegistry = Partial<
  Record<PipelineNodeType, PipelineNodeHandler>
>;

export interface PipelineExecutorApi {
  loadModel: typeof loadModel;
  constructReaction: typeof constructReaction;
  runAnalysis: typeof runAnalysis;
  testObservableScript: typeof testObservableScript;
}

export interface PipelineExecutorOptions {
  handlers?: PipelineNodeHandlerRegistry;
  baseInputsProvider?: () => PipelineBaseInputs;
  api?: PipelineExecutorApi;
}

function nowIso(): string {
  return new Date().toISOString();
}

function makeRunId(): string {
  return `pipe-run-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 9)}`;
}

function safeJsonClone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function parseCsvList(value: unknown): string[] {
  if (typeof value !== "string") return [];
  return value
    .split(",")
    .map((part) => part.trim())
    .filter((part) => part.length > 0);
}

function toNumber(value: unknown, fallback: number): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function toLheEventFragments(payload: unknown): string[] {
  if (!payload) return [];

  if (isRecord(payload) && Array.isArray(payload.lhe_events)) {
    return payload.lhe_events.map((entry) => String(entry));
  }

  if (Array.isArray(payload)) {
    return payload.map((entry) => String(entry));
  }

  if (isRecord(payload) && Array.isArray(payload.events)) {
    return payload.events.map((entry) => {
      if (typeof entry === "string") return entry;
      return `<event>${JSON.stringify(entry)}</event>`;
    });
  }

  return [String(payload)];
}

function reactionCmsEnergy(reaction: Reaction): number {
  const invariantMassSq = reaction.initial.invariant_mass_sq;
  if (Number.isFinite(invariantMassSq) && invariantMassSq > 0) {
    return Math.sqrt(invariantMassSq);
  }
  return 91.2;
}

function reactionFinalMasses(reaction: Reaction): number[] {
  return reaction.final_state.states.map((state) => state.particle.field.mass);
}

function defaultBaseInputs(): PipelineBaseInputs {
  return {
    particlesToml: "",
    verticesToml: "",
    modelName: "Standard Model",
  };
}

function initialNodeState(): NodeExecutionState {
  return { status: "idle" };
}

function initialEdgeState(): EdgeExecutionState {
  return {
    status: "idle",
    updatedAt: nowIso(),
  };
}

function cloneSnapshot(snapshot: PipelineRunSnapshot): PipelineRunSnapshot {
  return safeJsonClone(snapshot);
}

function deriveEdgePayloadForPort(nodeOutputs: Record<string, unknown>, sourcePort: PipelinePort): unknown {
  if (sourcePort.id in nodeOutputs) {
    return nodeOutputs[sourcePort.id];
  }
  if (sourcePort.key in nodeOutputs) {
    return nodeOutputs[sourcePort.key];
  }
  return undefined;
}

export class PipelineExecutor {
  private readonly listeners = new Set<(snapshot: PipelineRunSnapshot) => void>();
  private readonly handlers: Record<PipelineNodeType, PipelineNodeHandler>;
  private readonly baseInputsProvider: () => PipelineBaseInputs;
  private readonly api: PipelineExecutorApi;
  private snapshot: PipelineRunSnapshot;

  public constructor(options?: PipelineExecutorOptions) {
    this.api = options?.api ?? {
      loadModel,
      constructReaction,
      runAnalysis,
      testObservableScript,
    };

    this.baseInputsProvider = options?.baseInputsProvider ?? defaultBaseInputs;

    this.handlers = {
      ...this.createDefaultHandlers(),
      ...(options?.handlers ?? {}),
    } as Record<PipelineNodeType, PipelineNodeHandler>;

    this.snapshot = {
      runId: makeRunId(),
      isRunning: false,
      startedAt: null,
      completedAt: null,
      nodeStates: {},
      edgeStates: {},
      nodeOutputCache: {},
      edgePayloads: {},
      staleNodeIds: [],
    };
  }

  public subscribe(listener: (snapshot: PipelineRunSnapshot) => void): () => void {
    this.listeners.add(listener);
    listener(this.getSnapshot());
    return () => {
      this.listeners.delete(listener);
    };
  }

  public getSnapshot(): PipelineRunSnapshot {
    return cloneSnapshot(this.snapshot);
  }

  public clearRuntimeState(): PipelineRunSnapshot {
    this.snapshot = {
      ...this.snapshot,
      nodeStates: {},
      edgeStates: {},
      edgePayloads: {},
      nodeOutputCache: {},
      staleNodeIds: [],
      isRunning: false,
      startedAt: null,
      completedAt: null,
      lastError: undefined,
      runId: makeRunId(),
    };
    this.emit();
    return this.getSnapshot();
  }

  public markNodeStale(graph: PipelineGraph, dirtyNodeId: string): PipelineRunSnapshot {
    const staleResult = markStale(graph, dirtyNodeId, {
      staleNodeIds: new Set(this.snapshot.staleNodeIds),
      edgePayloads: this.snapshot.edgePayloads,
      nodeOutputs: this.snapshot.nodeOutputCache,
    });

    const nextNodeStates = { ...this.snapshot.nodeStates };
    for (const nodeId of staleResult.staleNodeIds) {
      nextNodeStates[nodeId] = {
        status: "stale",
      };
    }

    const nextEdgeStates = { ...this.snapshot.edgeStates };
    for (const edgeId of staleResult.clearedEdgeIds) {
      nextEdgeStates[edgeId] = {
        status: "stale",
        updatedAt: nowIso(),
      };
    }

    this.snapshot = {
      ...this.snapshot,
      staleNodeIds: Array.from(staleResult.staleNodeIds).sort((a, b) => a.localeCompare(b)),
      nodeOutputCache: staleResult.nodeOutputs,
      edgePayloads: staleResult.edgePayloads,
      nodeStates: nextNodeStates,
      edgeStates: nextEdgeStates,
    };
    this.emit();
    return this.getSnapshot();
  }

  public async executePipeline(graph: PipelineGraph): Promise<PipelineRunSnapshot> {
    const runId = makeRunId();
    const startedAt = nowIso();

    const nextNodeStates: Record<string, NodeExecutionState> = { ...this.snapshot.nodeStates };
    const nextEdgeStates: Record<string, EdgeExecutionState> = { ...this.snapshot.edgeStates };

    for (const nodeId of Object.keys(graph.nodes)) {
      if (!nextNodeStates[nodeId] || nextNodeStates[nodeId].status === "idle") {
        nextNodeStates[nodeId] = initialNodeState();
      }
    }

    for (const edgeId of Object.keys(graph.edges)) {
      if (!nextEdgeStates[edgeId]) {
        nextEdgeStates[edgeId] = initialEdgeState();
      }
    }

    this.snapshot = {
      ...this.snapshot,
      runId,
      isRunning: true,
      startedAt,
      completedAt: null,
      lastError: undefined,
      nodeStates: nextNodeStates,
      edgeStates: nextEdgeStates,
    };
    this.emit();

    let order: string[] = [];
    try {
      order = topologicalSortNodeIdsOrThrow(graph);
    } catch (error) {
      const message = error instanceof TopologyError
        ? `${error.message} (${error.cycleNodeIds.join(", ")})`
        : String(error);
      this.snapshot = {
        ...this.snapshot,
        isRunning: false,
        completedAt: nowIso(),
        lastError: message,
      };
      this.emit();
      throw error;
    }

    const staleSet = new Set(this.snapshot.staleNodeIds);

    for (const nodeId of order) {
      const node = graph.nodes[nodeId];
      if (!node) continue;

      const incomingEdges = Object.values(graph.edges).filter((edge) => edge.targetNodeId === node.id);

      const hasCached = Boolean(this.snapshot.nodeOutputCache[node.id]);
      const shouldExecute = staleSet.has(node.id) || !hasCached;

      if (!shouldExecute) {
        this.snapshot = {
          ...this.snapshot,
          nodeStates: {
            ...this.snapshot.nodeStates,
            [node.id]: {
              status: "completed",
              cached: true,
              completedAt: nowIso(),
            },
          },
        };
        this.emit();
        continue;
      }

      this.snapshot = {
        ...this.snapshot,
        nodeStates: {
          ...this.snapshot.nodeStates,
          [node.id]: {
            status: "running",
            startedAt: nowIso(),
            cached: false,
          },
        },
      };

      if (incomingEdges.length > 0) {
        const edgeStates = { ...this.snapshot.edgeStates };
        const tick = nowIso();
        for (const edge of incomingEdges) {
          edgeStates[edge.id] = {
            status: "running",
            updatedAt: tick,
          };
        }
        this.snapshot = {
          ...this.snapshot,
          edgeStates,
        };
      }
      this.emit();

      const startedAtMs = Date.now();

      try {
        const context = this.buildExecutionContext(graph, node);
        const handler = this.handlers[node.type];
        if (!handler) {
          throw new Error(`No pipeline handler registered for node type '${node.type}'.`);
        }

        const result = await handler(context);
        const nodeOutputs = this.mapNodeOutputs(node, result.outputs);
        const edgePayloads = this.updateEdgePayloadsForNode(graph, node, nodeOutputs);

        staleSet.delete(node.id);

        const edgeStates = { ...this.snapshot.edgeStates };
        const tick = nowIso();
        for (const edge of incomingEdges) {
          edgeStates[edge.id] = {
            status: "completed",
            updatedAt: tick,
          };
        }

        this.snapshot = {
          ...this.snapshot,
          nodeOutputCache: {
            ...this.snapshot.nodeOutputCache,
            [node.id]: nodeOutputs,
          },
          edgePayloads,
          edgeStates,
          nodeStates: {
            ...this.snapshot.nodeStates,
            [node.id]: {
              status: "completed",
              startedAt: this.snapshot.nodeStates[node.id]?.startedAt,
              completedAt: tick,
              durationMs: Date.now() - startedAtMs,
              cached: false,
            },
          },
          staleNodeIds: Array.from(staleSet).sort((a, b) => a.localeCompare(b)),
        };
        this.emit();
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);

        const edgeStates = { ...this.snapshot.edgeStates };
        const tick = nowIso();
        for (const edge of incomingEdges) {
          edgeStates[edge.id] = {
            status: "error",
            updatedAt: tick,
            error: message,
          };
        }

        this.snapshot = {
          ...this.snapshot,
          isRunning: false,
          completedAt: tick,
          lastError: message,
          edgeStates,
          nodeStates: {
            ...this.snapshot.nodeStates,
            [node.id]: {
              status: "error",
              startedAt: this.snapshot.nodeStates[node.id]?.startedAt,
              completedAt: tick,
              durationMs: Date.now() - startedAtMs,
              error: message,
              cached: false,
            },
          },
        };
        this.emit();
        throw error;
      }
    }

    this.snapshot = {
      ...this.snapshot,
      isRunning: false,
      completedAt: nowIso(),
      staleNodeIds: Array.from(staleSet).sort((a, b) => a.localeCompare(b)),
    };
    this.emit();

    return this.getSnapshot();
  }

  private buildExecutionContext(graph: PipelineGraph, node: PipelineNode): PipelineExecutorContext {
    const incomingEdges = Object.values(graph.edges).filter((edge) => edge.targetNodeId === node.id);

    const inputsByPortId: Record<string, unknown> = {};
    const inputsByPortKey: Record<string, unknown> = {};

    for (const edge of incomingEdges) {
      const sourceNodeOutputs = this.snapshot.nodeOutputCache[edge.sourceNodeId] ?? {};
      const sourceNode = graph.nodes[edge.sourceNodeId];
      const sourcePort = sourceNode
        ? [...sourceNode.inputs, ...sourceNode.outputs].find((port) => port.id === edge.sourcePortId)
        : null;

      const payload = sourcePort
        ? deriveEdgePayloadForPort(sourceNodeOutputs, sourcePort)
        : sourceNodeOutputs[edge.sourcePortId];

      const targetPort = [...node.inputs, ...node.outputs].find((port) => port.id === edge.targetPortId);
      if (targetPort) {
        inputsByPortId[targetPort.id] = payload;
        inputsByPortKey[targetPort.key] = payload;
      }
    }

    return {
      graph,
      node,
      inputsByPortId,
      inputsByPortKey,
      baseInputs: this.baseInputsProvider(),
      previousNodeOutputs: this.snapshot.nodeOutputCache[node.id] ?? {},
    };
  }

  private mapNodeOutputs(node: PipelineNode, outputs: Record<string, unknown>): Record<string, unknown> {
    const mapped: Record<string, unknown> = {};

    for (const port of node.outputs) {
      if (port.id in outputs) {
        mapped[port.id] = outputs[port.id];
      } else if (port.key in outputs) {
        mapped[port.id] = outputs[port.key];
      } else {
        mapped[port.id] = null;
      }
      mapped[port.key] = mapped[port.id];
    }

    return mapped;
  }

  private updateEdgePayloadsForNode(
    graph: PipelineGraph,
    node: PipelineNode,
    nodeOutputs: Record<string, unknown>,
  ): Record<string, unknown> {
    const edgePayloads = { ...this.snapshot.edgePayloads };
    for (const edge of Object.values(graph.edges)) {
      if (edge.sourceNodeId !== node.id) continue;

      const sourcePort = node.outputs.find((port) => port.id === edge.sourcePortId);
      if (!sourcePort) continue;

      const payload = deriveEdgePayloadForPort(nodeOutputs, sourcePort);
      edgePayloads[edge.id] = payload;
    }
    return edgePayloads;
  }

  private createDefaultHandlers(): PipelineNodeHandlerRegistry {
    return {
      model_source: async ({ node, baseInputs }) => {
        const modelName = String(node.parameters.modelName ?? baseInputs.modelName).trim() || "Standard Model";
        const model = await this.api.loadModel(
          baseInputs.particlesToml,
          baseInputs.verticesToml,
          modelName,
        );

        return {
          outputs: {
            theory: model,
          },
          summary: `Loaded model ${model.name}`,
        };
      },

      reaction_builder: async ({ node, inputsByPortKey }) => {
        const model = inputsByPortKey.theory_in as TheoreticalModel | undefined;
        if (!model) {
          throw new Error("Reaction Builder requires Theory Input.");
        }

        const initialState = parseCsvList(node.parameters.initialState);
        const finalState = parseCsvList(node.parameters.finalState);
        const cmsEnergy = toNumber(node.parameters.cmsEnergy, 91.2);

        if (initialState.length === 0 || finalState.length === 0) {
          throw new Error("Reaction Builder requires non-empty initial and final states.");
        }

        const reaction = await this.api.constructReaction(initialState, finalState, cmsEnergy, model);

        return {
          outputs: {
            reaction_out: reaction,
          },
          summary: "Reaction validated and reconstructed.",
        };
      },

      monte_carlo_generator: async ({ node, inputsByPortKey }) => {
        const reaction = inputsByPortKey.reaction_in as Reaction | undefined;
        if (!reaction) {
          throw new Error("Monte Carlo Generator requires a Reaction input.");
        }

        const numEvents = Math.max(1, Math.floor(toNumber(node.parameters.numEvents, 10_000)));
        const seed = Math.max(0, Math.floor(toNumber(node.parameters.seed, 42)));
        const weighted = Boolean(node.parameters.weighted ?? true);

        const analysis = await this.api.runAnalysis({
          plots: [
            {
              name: "Pipeline pT",
              observable_script: "event.momenta[0].pt()",
              n_bins: 36,
              min: 0,
              max: 180,
            },
          ],
          plots_2d: null,
          cut_scripts: [],
          num_events: numEvents,
          cms_energy: reactionCmsEnergy(reaction),
          final_masses: reactionFinalMasses(reaction),
          detector_preset: null,
          particle_kinds: null,
        });

        const kinematicsSummary = {
          events_generated: analysis.events_generated,
          events_passed: analysis.events_passed,
          cross_section: analysis.cross_section,
          cross_section_error: analysis.cross_section_error,
          weighted,
          seed,
        };

        return {
          outputs: {
            events_out: analysis,
            kinematics_out: kinematicsSummary,
          },
          summary: "Monte Carlo integration complete.",
        };
      },

      detector_simulation: async ({ node, inputsByPortKey }) => {
        const eventBatch = inputsByPortKey.events_in as AnalysisResult | undefined;
        if (!eventBatch) {
          throw new Error("Detector Simulation requires Event Batch input.");
        }

        return {
          outputs: {
            detector_events_out: {
              ...eventBatch,
              detector_preset: String(node.parameters.preset ?? "lhc_like"),
              smearing: Boolean(node.parameters.smearing ?? true),
            },
          },
        };
      },

      observable_script: async ({ node, inputsByPortKey }) => {
        const sourceEvents = inputsByPortKey.events_in;
        if (!sourceEvents) {
          throw new Error("Observable Script requires event input.");
        }

        const script = String(node.parameters.script ?? "").trim();
        const name = String(node.parameters.name ?? "Observable").trim() || "Observable";
        if (script.length === 0) {
          throw new Error("Observable Script requires a non-empty Rhai expression.");
        }

        const probe = await this.api.testObservableScript(script);

        return {
          outputs: {
            observable_out: {
              name,
              script,
              probe,
              source: sourceEvents,
            },
          },
        };
      },

      histogram_filler: async ({ node, inputsByPortKey }) => {
        const observable = inputsByPortKey.observable_in as
          | { name?: string; script?: string; source?: AnalysisResult }
          | undefined;

        if (!observable) {
          throw new Error("Histogram Filler requires observable input.");
        }

        const bins = Math.max(1, Math.floor(toNumber(node.parameters.bins, 40)));
        const xMin = toNumber(node.parameters.xMin, 0);
        const xMax = toNumber(node.parameters.xMax, 200);

        const sourceHistogram = observable.source?.histograms?.[0] ?? null;

        return {
          outputs: {
            histogram_out: {
              bins,
              xMin,
              xMax,
              logY: Boolean(node.parameters.logY ?? false),
              sourceHistogram,
              observable,
            },
          },
        };
      },

      export_sink: async ({ node, inputsByPortKey }) => {
        const histogram = inputsByPortKey.histogram_in;
        const events = inputsByPortKey.events_in;
        const format = String(node.parameters.format ?? "json");
        const fileName = String(node.parameters.fileName ?? "spire_output");

        let content: string | null = null;
        if (format === "lhe") {
          const fragments = toLheEventFragments(events);
          content = await executeWorkerTask<
            { header?: string; init?: string; events: string[]; footer?: string },
            string
          >(
            new URL("$lib/workers/lhe_formatter.worker.ts", import.meta.url),
            {
              header: '<LesHouchesEvents version="3.0">',
              events: fragments,
              footer: "</LesHouchesEvents>",
            },
          );
        }

        return {
          outputs: {
            payload_out: {
              format,
              fileName,
              includeProvenance: Boolean(node.parameters.includeProvenance ?? true),
              histogram,
              events,
              content,
            },
          },
        };
      },

      custom_rhai_script: async ({ node, inputsByPortKey }) => {
        const script = String(node.parameters.script ?? "").trim();
        if (script.length === 0) {
          throw new Error("Custom Rhai Script requires a non-empty script.");
        }

        await this.api.testObservableScript(script);

        return {
          outputs: {
            script_out: script,
          },
        };
      },
    };
  }

  private emit(): void {
    const snapshot = this.getSnapshot();
    for (const listener of this.listeners) {
      listener(snapshot);
    }
  }
}
