/**
 * SPIRE Pipeline Graph Engine
 *
 * Rendering-agnostic graph primitives for the visual automation layer.
 * This module owns type validation, edge legality checks, acyclicity,
 * topological sorting, reducer actions, and JSON serialisation.
 */

// ===========================================================================
// Type Registry
// ===========================================================================

export type PipelineDataType =
  | "TheoreticalModel"
  | "Reaction"
  | "EventBatch"
  | "DetectorEventBatch"
  | "ObservableSeries"
  | "HistogramData"
  | "ExportPayload"
  | "RhaiScript"
  | "Kinematics"
  | "String"
  | "Number"
  | "Boolean"
  | "Any";

export type PipelineNodeType =
  | "model_source"
  | "reaction_builder"
  | "monte_carlo_generator"
  | "detector_simulation"
  | "observable_script"
  | "histogram_filler"
  | "export_sink"
  | "custom_rhai_script";

export type PortDirection = "input" | "output";

export interface WorldPoint {
  x: number;
  y: number;
}

export interface NodeSize {
  width: number;
  height: number;
}

export interface PipelinePort {
  id: string;
  key: string;
  label: string;
  direction: PortDirection;
  dataType: PipelineDataType;
  multiConnection: boolean;
}

export type ParameterControlType = "number" | "slider" | "text" | "textarea" | "select" | "toggle";

export interface PipelineParameterSchema {
  key: string;
  label: string;
  control: ParameterControlType;
  defaultValue: string | number | boolean;
  min?: number;
  max?: number;
  step?: number;
  options?: Array<{ label: string; value: string }>;
  help?: string;
}

export type PipelineParameterValue = string | number | boolean | string[];

export interface PipelineNodeDefinition {
  type: PipelineNodeType;
  label: string;
  description: string;
  size: NodeSize;
  inputs: Array<Omit<PipelinePort, "id">>;
  outputs: Array<Omit<PipelinePort, "id">>;
  parameterSchema: PipelineParameterSchema[];
}

export interface PipelineNode {
  id: string;
  type: PipelineNodeType;
  label: string;
  position: WorldPoint;
  size: NodeSize;
  inputs: PipelinePort[];
  outputs: PipelinePort[];
  parameters: Record<string, PipelineParameterValue>;
}

export interface PipelineEdge {
  id: string;
  sourceNodeId: string;
  sourcePortId: string;
  targetNodeId: string;
  targetPortId: string;
}

export interface PipelineGraph {
  nodes: Record<string, PipelineNode>;
  edges: Record<string, PipelineEdge>;
  selectedNodeId: string | null;
  updatedAt: string;
}

export interface ConnectionValidation {
  ok: boolean;
  reason?: string;
}

export interface ResolvedPort {
  node: PipelineNode;
  port: PipelinePort;
}

const TYPE_COMPATIBILITY: Record<PipelineDataType, ReadonlySet<PipelineDataType>> = {
  TheoreticalModel: new Set(["TheoreticalModel", "Any"]),
  Reaction: new Set(["Reaction", "Any"]),
  EventBatch: new Set(["EventBatch", "Any"]),
  DetectorEventBatch: new Set(["DetectorEventBatch", "EventBatch", "Any"]),
  ObservableSeries: new Set(["ObservableSeries", "Any"]),
  HistogramData: new Set(["HistogramData", "Any"]),
  ExportPayload: new Set(["ExportPayload", "Any"]),
  RhaiScript: new Set(["RhaiScript", "String", "Any"]),
  Kinematics: new Set(["Kinematics", "Any"]),
  String: new Set(["String", "Any"]),
  Number: new Set(["Number", "Any"]),
  Boolean: new Set(["Boolean", "Any"]),
  Any: new Set(["Any"]),
};

let _pipelineCounter = 0;

function makeId(prefix: "node" | "edge"): string {
  _pipelineCounter += 1;
  return `pipe-${prefix}-${_pipelineCounter}-${Date.now().toString(36)}`;
}

function makePortId(nodeId: string, direction: PortDirection, key: string): string {
  return `${nodeId}:${direction}:${key}`;
}

function nowIso(): string {
  return new Date().toISOString();
}

export const NODE_DEFINITIONS: Record<PipelineNodeType, PipelineNodeDefinition> = {
  model_source: {
    type: "model_source",
    label: "Model Source",
    description: "Loads or references a theoretical model for downstream nodes.",
    size: { width: 248, height: 132 },
    inputs: [],
    outputs: [
      {
        key: "theory",
        label: "Theory Output",
        direction: "output",
        dataType: "TheoreticalModel",
        multiConnection: true,
      },
    ],
    parameterSchema: [
      {
        key: "modelName",
        label: "Model",
        control: "select",
        defaultValue: "Standard Model",
        options: [{ label: "Standard Model", value: "Standard Model" }],
        help: "Select the loaded model exposed to the graph.",
      },
      {
        key: "autoLoad",
        label: "Auto Load",
        control: "toggle",
        defaultValue: true,
      },
    ],
  },
  reaction_builder: {
    type: "reaction_builder",
    label: "Reaction Builder",
    description: "Builds or reconstructs reactions from model + initial/final states.",
    size: { width: 272, height: 168 },
    inputs: [
      {
        key: "theory_in",
        label: "Theory Input",
        direction: "input",
        dataType: "TheoreticalModel",
        multiConnection: false,
      },
      {
        key: "label_in",
        label: "Label",
        direction: "input",
        dataType: "String",
        multiConnection: false,
      },
    ],
    outputs: [
      {
        key: "reaction_out",
        label: "Reaction",
        direction: "output",
        dataType: "Reaction",
        multiConnection: true,
      },
    ],
    parameterSchema: [
      { key: "initialState", label: "Initial IDs", control: "text", defaultValue: "e-,e+" },
      { key: "finalState", label: "Final IDs", control: "text", defaultValue: "mu-,mu+" },
      { key: "cmsEnergy", label: "√s (GeV)", control: "number", defaultValue: 91.2, min: 0.1, step: 0.1 },
      { key: "reconstructOnly", label: "Reconstruct Only", control: "toggle", defaultValue: false },
      { key: "maxLoopOrder", label: "Max Loop", control: "slider", defaultValue: 0, min: 0, max: 3, step: 1 },
    ],
  },
  monte_carlo_generator: {
    type: "monte_carlo_generator",
    label: "Monte Carlo Generator",
    description: "Samples phase space and produces event batches.",
    size: { width: 272, height: 154 },
    inputs: [
      {
        key: "reaction_in",
        label: "Reaction",
        direction: "input",
        dataType: "Reaction",
        multiConnection: false,
      },
    ],
    outputs: [
      {
        key: "events_out",
        label: "Event Batch",
        direction: "output",
        dataType: "EventBatch",
        multiConnection: true,
      },
      {
        key: "kinematics_out",
        label: "Kinematics",
        direction: "output",
        dataType: "Kinematics",
        multiConnection: true,
      },
    ],
    parameterSchema: [
      { key: "numEvents", label: "Events", control: "number", defaultValue: 10000, min: 1, step: 100 },
      { key: "seed", label: "Seed", control: "number", defaultValue: 42, min: 0, step: 1 },
      {
        key: "weighted",
        label: "Weighted Events",
        control: "toggle",
        defaultValue: true,
      },
    ],
  },
  detector_simulation: {
    type: "detector_simulation",
    label: "Detector Simulation",
    description: "Applies detector effects and reconstruction pipeline.",
    size: { width: 256, height: 146 },
    inputs: [
      {
        key: "events_in",
        label: "Event Batch",
        direction: "input",
        dataType: "EventBatch",
        multiConnection: false,
      },
    ],
    outputs: [
      {
        key: "detector_events_out",
        label: "Reco Events",
        direction: "output",
        dataType: "DetectorEventBatch",
        multiConnection: true,
      },
    ],
    parameterSchema: [
      {
        key: "preset",
        label: "Preset",
        control: "select",
        defaultValue: "lhc_like",
        options: [
          { label: "LHC-like", value: "lhc_like" },
          { label: "ILC-like", value: "ilc_like" },
          { label: "Perfect", value: "perfect" },
        ],
      },
      { key: "smearing", label: "Smearing", control: "toggle", defaultValue: true },
    ],
  },
  observable_script: {
    type: "observable_script",
    label: "Observable Script",
    description: "Compiles Rhai observable definitions against event streams.",
    size: { width: 300, height: 202 },
    inputs: [
      {
        key: "events_in",
        label: "Events",
        direction: "input",
        dataType: "DetectorEventBatch",
        multiConnection: false,
      },
      {
        key: "script_in",
        label: "Script Input",
        direction: "input",
        dataType: "RhaiScript",
        multiConnection: false,
      },
    ],
    outputs: [
      {
        key: "observable_out",
        label: "Observable",
        direction: "output",
        dataType: "ObservableSeries",
        multiConnection: true,
      },
    ],
    parameterSchema: [
      { key: "name", label: "Name", control: "text", defaultValue: "Leading pT" },
      {
        key: "script",
        label: "Rhai",
        control: "textarea",
        defaultValue: "event.momenta[0].pt()",
      },
    ],
  },
  histogram_filler: {
    type: "histogram_filler",
    label: "Histogram Filler",
    description: "Bins observable outputs into 1D histograms.",
    size: { width: 248, height: 158 },
    inputs: [
      {
        key: "observable_in",
        label: "Observable",
        direction: "input",
        dataType: "ObservableSeries",
        multiConnection: false,
      },
    ],
    outputs: [
      {
        key: "histogram_out",
        label: "Histogram",
        direction: "output",
        dataType: "HistogramData",
        multiConnection: true,
      },
    ],
    parameterSchema: [
      { key: "bins", label: "Bins", control: "number", defaultValue: 40, min: 1, step: 1 },
      { key: "xMin", label: "X Min", control: "number", defaultValue: 0, step: 0.1 },
      { key: "xMax", label: "X Max", control: "number", defaultValue: 200, step: 0.1 },
      { key: "logY", label: "Log Y", control: "toggle", defaultValue: false },
    ],
  },
  export_sink: {
    type: "export_sink",
    label: "Export Sink",
    description: "Serialises the pipeline output into external artefacts.",
    size: { width: 256, height: 170 },
    inputs: [
      {
        key: "histogram_in",
        label: "Histogram",
        direction: "input",
        dataType: "HistogramData",
        multiConnection: false,
      },
      {
        key: "events_in",
        label: "Events",
        direction: "input",
        dataType: "DetectorEventBatch",
        multiConnection: false,
      },
    ],
    outputs: [
      {
        key: "payload_out",
        label: "Export Payload",
        direction: "output",
        dataType: "ExportPayload",
        multiConnection: true,
      },
    ],
    parameterSchema: [
      {
        key: "format",
        label: "Format",
        control: "select",
        defaultValue: "json",
        options: [
          { label: "JSON", value: "json" },
          { label: "LHE", value: "lhe" },
          { label: "LaTeX", value: "latex" },
          { label: "CSV", value: "csv" },
        ],
      },
      { key: "fileName", label: "File Name", control: "text", defaultValue: "spire_output" },
      { key: "includeProvenance", label: "Provenance", control: "toggle", defaultValue: true },
    ],
  },
  custom_rhai_script: {
    type: "custom_rhai_script",
    label: "Custom Rhai Script",
    description: "General-purpose transformation script node.",
    size: { width: 292, height: 190 },
    inputs: [
      {
        key: "payload_in",
        label: "Payload",
        direction: "input",
        dataType: "Any",
        multiConnection: false,
      },
      {
        key: "label_in",
        label: "String Label",
        direction: "input",
        dataType: "String",
        multiConnection: false,
      },
    ],
    outputs: [
      {
        key: "script_out",
        label: "Rhai Script",
        direction: "output",
        dataType: "RhaiScript",
        multiConnection: true,
      },
    ],
    parameterSchema: [
      { key: "entry", label: "Entry Function", control: "text", defaultValue: "transform" },
      {
        key: "script",
        label: "Rhai",
        control: "textarea",
        defaultValue: "fn transform(payload) { payload }",
      },
    ],
  },
};

function buildDefaultParameters(definition: PipelineNodeDefinition): Record<string, PipelineParameterValue> {
  const result: Record<string, PipelineParameterValue> = {};
  for (const schema of definition.parameterSchema) {
    result[schema.key] = schema.defaultValue;
  }
  return result;
}

function cloneGraph(graph: PipelineGraph): PipelineGraph {
  return JSON.parse(JSON.stringify(graph)) as PipelineGraph;
}

function listOutgoingEdges(graph: PipelineGraph, nodeId: string): PipelineEdge[] {
  return Object.values(graph.edges).filter((edge) => edge.sourceNodeId === nodeId);
}

function computeInDegrees(graph: PipelineGraph): Map<string, number> {
  const inDegree = new Map<string, number>();
  for (const nodeId of Object.keys(graph.nodes)) {
    inDegree.set(nodeId, 0);
  }
  for (const edge of Object.values(graph.edges)) {
    inDegree.set(edge.targetNodeId, (inDegree.get(edge.targetNodeId) ?? 0) + 1);
  }
  return inDegree;
}

function sortNodeIdsStable(a: string, b: string): number {
  return a.localeCompare(b);
}

function touchingNodeIds(graph: PipelineGraph, nodeId: string): Set<string> {
  const touches = new Set<string>();
  for (const edge of Object.values(graph.edges)) {
    if (edge.sourceNodeId === nodeId) {
      touches.add(edge.targetNodeId);
    }
    if (edge.targetNodeId === nodeId) {
      touches.add(edge.sourceNodeId);
    }
  }
  return touches;
}

function wouldCreateCycle(graph: PipelineGraph, sourceNodeId: string, targetNodeId: string): boolean {
  if (sourceNodeId === targetNodeId) {
    return true;
  }
  const stack: string[] = [targetNodeId];
  const visited = new Set<string>();

  while (stack.length > 0) {
    const current = stack.pop();
    if (!current || visited.has(current)) {
      continue;
    }
    visited.add(current);
    if (current === sourceNodeId) {
      return true;
    }
    const out = listOutgoingEdges(graph, current);
    for (const edge of out) {
      stack.push(edge.targetNodeId);
    }
  }
  return false;
}

function resolvePortById(graph: PipelineGraph, nodeId: string, portId: string): ResolvedPort | null {
  const node = graph.nodes[nodeId];
  if (!node) return null;

  const port = [...node.inputs, ...node.outputs].find((candidate) => candidate.id === portId);
  if (!port) return null;

  return { node, port };
}

function hasConnectionOnPort(graph: PipelineGraph, portId: string): boolean {
  return Object.values(graph.edges).some(
    (edge) => edge.sourcePortId === portId || edge.targetPortId === portId,
  );
}

export function createEmptyPipelineGraph(): PipelineGraph {
  return {
    nodes: {},
    edges: {},
    selectedNodeId: null,
    updatedAt: nowIso(),
  };
}

export function createPipelineNode(nodeType: PipelineNodeType, position: WorldPoint): PipelineNode {
  const definition = NODE_DEFINITIONS[nodeType];
  const nodeId = makeId("node");

  const inputs = definition.inputs.map((port) => ({
    ...port,
    id: makePortId(nodeId, "input", port.key),
  }));

  const outputs = definition.outputs.map((port) => ({
    ...port,
    id: makePortId(nodeId, "output", port.key),
  }));

  return {
    id: nodeId,
    type: nodeType,
    label: definition.label,
    position,
    size: { ...definition.size },
    inputs,
    outputs,
    parameters: buildDefaultParameters(definition),
  };
}

export function canConnect(outputPort: PipelinePort, inputPort: PipelinePort): ConnectionValidation {
  if (outputPort.direction !== "output") {
    return { ok: false, reason: "Source port is not an output." };
  }
  if (inputPort.direction !== "input") {
    return { ok: false, reason: "Target port is not an input." };
  }

  const allowedTargets = TYPE_COMPATIBILITY[outputPort.dataType] ?? new Set<PipelineDataType>();
  if (!allowedTargets.has(inputPort.dataType)) {
    return {
      ok: false,
      reason: `Incompatible types: ${outputPort.dataType} → ${inputPort.dataType}.`,
    };
  }

  return { ok: true };
}

export function validateEdge(graph: PipelineGraph, edge: PipelineEdge): ConnectionValidation {
  const source = resolvePortById(graph, edge.sourceNodeId, edge.sourcePortId);
  const target = resolvePortById(graph, edge.targetNodeId, edge.targetPortId);

  if (!source || !target) {
    return { ok: false, reason: "Source or target port does not exist." };
  }

  if (source.node.id === target.node.id) {
    return { ok: false, reason: "Self-connections are not allowed." };
  }

  const typeValidation = canConnect(source.port, target.port);
  if (!typeValidation.ok) {
    return typeValidation;
  }

  const duplicate = Object.values(graph.edges).some(
    (existing) =>
      existing.sourcePortId === edge.sourcePortId &&
      existing.targetPortId === edge.targetPortId &&
      existing.id !== edge.id,
  );
  if (duplicate) {
    return { ok: false, reason: "Duplicate edge." };
  }

  if (!source.port.multiConnection && hasConnectionOnPort(graph, source.port.id)) {
    return { ok: false, reason: "Source port accepts only one connection." };
  }
  if (!target.port.multiConnection && hasConnectionOnPort(graph, target.port.id)) {
    return { ok: false, reason: "Target port accepts only one connection." };
  }

  if (wouldCreateCycle(graph, source.node.id, target.node.id)) {
    return { ok: false, reason: "Connection introduces a cycle." };
  }

  return { ok: true };
}

export function topologicalSortNodeIds(graph: PipelineGraph): string[] {
  const inDegree = computeInDegrees(graph);
  const queue = Object.keys(graph.nodes)
    .filter((id) => (inDegree.get(id) ?? 0) === 0)
    .sort(sortNodeIdsStable);
  const sorted: string[] = [];

  while (queue.length > 0) {
    const nodeId = queue.shift();
    if (!nodeId) break;
    sorted.push(nodeId);

    for (const edge of listOutgoingEdges(graph, nodeId)) {
      const nextDegree = (inDegree.get(edge.targetNodeId) ?? 0) - 1;
      inDegree.set(edge.targetNodeId, nextDegree);
      if (nextDegree === 0) {
        queue.push(edge.targetNodeId);
        queue.sort(sortNodeIdsStable);
      }
    }
  }

  if (sorted.length !== Object.keys(graph.nodes).length) {
    return [];
  }

  return sorted;
}

export function topologicalSort(graph: PipelineGraph): PipelineNode[] {
  return topologicalSortNodeIds(graph)
    .map((id) => graph.nodes[id])
    .filter((node): node is PipelineNode => Boolean(node));
}

function setUpdatedTimestamp(graph: PipelineGraph): PipelineGraph {
  return {
    ...graph,
    updatedAt: nowIso(),
  };
}

export function addNode(graph: PipelineGraph, node: PipelineNode): PipelineGraph {
  const next = cloneGraph(graph);
  next.nodes[node.id] = node;
  next.selectedNodeId = node.id;
  return setUpdatedTimestamp(next);
}

export function removeNode(graph: PipelineGraph, nodeId: string): PipelineGraph {
  const next = cloneGraph(graph);
  delete next.nodes[nodeId];

  for (const edgeId of Object.keys(next.edges)) {
    const edge = next.edges[edgeId];
    if (edge.sourceNodeId === nodeId || edge.targetNodeId === nodeId) {
      delete next.edges[edgeId];
    }
  }

  if (next.selectedNodeId === nodeId) {
    const remainingNodeIds = Object.keys(next.nodes).sort(sortNodeIdsStable);
    next.selectedNodeId = remainingNodeIds[0] ?? null;
  }

  return setUpdatedTimestamp(next);
}

export function updateNodePosition(graph: PipelineGraph, nodeId: string, position: WorldPoint): PipelineGraph {
  const next = cloneGraph(graph);
  const node = next.nodes[nodeId];
  if (!node) return graph;
  node.position = position;
  return setUpdatedTimestamp(next);
}

export function updateNodeParameters(
  graph: PipelineGraph,
  nodeId: string,
  patch: Record<string, PipelineParameterValue>,
): PipelineGraph {
  const next = cloneGraph(graph);
  const node = next.nodes[nodeId];
  if (!node) return graph;
  node.parameters = {
    ...node.parameters,
    ...patch,
  };
  return setUpdatedTimestamp(next);
}

export function selectNode(graph: PipelineGraph, nodeId: string | null): PipelineGraph {
  if (nodeId !== null && !graph.nodes[nodeId]) {
    return graph;
  }
  return setUpdatedTimestamp({
    ...graph,
    selectedNodeId: nodeId,
  });
}

export function addEdge(
  graph: PipelineGraph,
  edge: Omit<PipelineEdge, "id">,
): { graph: PipelineGraph; validation: ConnectionValidation } {
  const next = cloneGraph(graph);
  const candidate: PipelineEdge = {
    ...edge,
    id: makeId("edge"),
  };

  const validation = validateEdge(next, candidate);
  if (!validation.ok) {
    return { graph, validation };
  }

  next.edges[candidate.id] = candidate;
  return {
    graph: setUpdatedTimestamp(next),
    validation,
  };
}

export function removeEdge(graph: PipelineGraph, edgeId: string): PipelineGraph {
  const next = cloneGraph(graph);
  delete next.edges[edgeId];
  return setUpdatedTimestamp(next);
}

export function listNodeIds(graph: PipelineGraph): string[] {
  return Object.keys(graph.nodes).sort(sortNodeIdsStable);
}

export function listEdgeIds(graph: PipelineGraph): string[] {
  return Object.keys(graph.edges).sort(sortNodeIdsStable);
}

export function getConnectedNodeIds(graph: PipelineGraph, nodeId: string): string[] {
  return Array.from(touchingNodeIds(graph, nodeId)).sort(sortNodeIdsStable);
}

export function serializeGraph(graph: PipelineGraph): string {
  return JSON.stringify(graph, null, 2);
}

export function deserializeGraph(json: string): PipelineGraph {
  const raw = JSON.parse(json) as Partial<PipelineGraph>;
  if (
    !raw ||
    typeof raw !== "object" ||
    typeof raw.nodes !== "object" ||
    raw.nodes === null ||
    typeof raw.edges !== "object" ||
    raw.edges === null
  ) {
    throw new Error("Invalid pipeline graph payload.");
  }

  return {
    nodes: raw.nodes as Record<string, PipelineNode>,
    edges: raw.edges as Record<string, PipelineEdge>,
    selectedNodeId: typeof raw.selectedNodeId === "string" ? raw.selectedNodeId : null,
    updatedAt: typeof raw.updatedAt === "string" ? raw.updatedAt : nowIso(),
  };
}

export type PipelineGraphAction =
  | { type: "ADD_NODE"; nodeType: PipelineNodeType; position: WorldPoint }
  | { type: "REMOVE_NODE"; nodeId: string }
  | { type: "MOVE_NODE"; nodeId: string; position: WorldPoint }
  | { type: "SET_NODE_PARAMS"; nodeId: string; patch: Record<string, PipelineParameterValue> }
  | { type: "ADD_EDGE"; sourceNodeId: string; sourcePortId: string; targetNodeId: string; targetPortId: string }
  | { type: "REMOVE_EDGE"; edgeId: string }
  | { type: "SELECT_NODE"; nodeId: string | null }
  | { type: "REPLACE_GRAPH"; graph: PipelineGraph }
  | { type: "RESET" };

export function reducePipelineGraph(
  state: PipelineGraph,
  action: PipelineGraphAction,
): { state: PipelineGraph; validation?: ConnectionValidation } {
  switch (action.type) {
    case "ADD_NODE": {
      const node = createPipelineNode(action.nodeType, action.position);
      return { state: addNode(state, node) };
    }
    case "REMOVE_NODE":
      return { state: removeNode(state, action.nodeId) };
    case "MOVE_NODE":
      return { state: updateNodePosition(state, action.nodeId, action.position) };
    case "SET_NODE_PARAMS":
      return {
        state: updateNodeParameters(state, action.nodeId, action.patch),
      };
    case "ADD_EDGE": {
      const result = addEdge(state, {
        sourceNodeId: action.sourceNodeId,
        sourcePortId: action.sourcePortId,
        targetNodeId: action.targetNodeId,
        targetPortId: action.targetPortId,
      });
      return { state: result.graph, validation: result.validation };
    }
    case "REMOVE_EDGE":
      return { state: removeEdge(state, action.edgeId) };
    case "SELECT_NODE":
      return { state: selectNode(state, action.nodeId) };
    case "REPLACE_GRAPH":
      return { state: setUpdatedTimestamp(cloneGraph(action.graph)) };
    case "RESET":
      return { state: createEmptyPipelineGraph() };
    default:
      return { state };
  }
}

export interface PipelineGraphStoreSnapshot {
  graph: PipelineGraph;
  lastValidation: ConnectionValidation | null;
}

export class PipelineGraphStore {
  private snapshot: PipelineGraphStoreSnapshot;
  private listeners = new Set<(snapshot: PipelineGraphStoreSnapshot) => void>();

  public constructor(initialGraph?: PipelineGraph) {
    this.snapshot = {
      graph: initialGraph ? cloneGraph(initialGraph) : createEmptyPipelineGraph(),
      lastValidation: null,
    };
  }

  public getSnapshot(): PipelineGraphStoreSnapshot {
    return cloneGraph({
      ...this.snapshot.graph,
    })
      ? {
          graph: cloneGraph(this.snapshot.graph),
          lastValidation: this.snapshot.lastValidation
            ? { ...this.snapshot.lastValidation }
            : null,
        }
      : this.snapshot;
  }

  public subscribe(
    listener: (snapshot: PipelineGraphStoreSnapshot) => void,
  ): () => void {
    this.listeners.add(listener);
    listener(this.getSnapshot());
    return () => {
      this.listeners.delete(listener);
    };
  }

  public dispatch(action: PipelineGraphAction): PipelineGraphStoreSnapshot {
    const result = reducePipelineGraph(this.snapshot.graph, action);
    this.snapshot = {
      graph: result.state,
      lastValidation: result.validation ?? this.snapshot.lastValidation,
    };

    for (const listener of this.listeners) {
      listener(this.getSnapshot());
    }

    return this.getSnapshot();
  }

  public replace(graph: PipelineGraph): PipelineGraphStoreSnapshot {
    this.snapshot = {
      graph: cloneGraph(graph),
      lastValidation: null,
    };
    for (const listener of this.listeners) {
      listener(this.getSnapshot());
    }
    return this.getSnapshot();
  }
}
