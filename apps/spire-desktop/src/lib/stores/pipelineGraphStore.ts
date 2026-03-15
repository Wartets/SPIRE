import { derived, writable, get } from "svelte/store";
import {
  NODE_DEFINITIONS,
  PipelineGraphStore,
  addNode,
  addEdge,
  createEmptyPipelineGraph,
  createPipelineNode,
  removeEdge,
  removeNode,
  selectNode,
  serializeGraph,
  deserializeGraph,
  type ConnectionValidation,
  type PipelineDataType,
  type PipelineGraph,
  type PipelineNode,
  type PipelineNodeType,
  type PipelineParameterValue,
  type PipelinePort,
  type WorldPoint,
  type PipelineGraphAction,
  type PipelineGraphStoreSnapshot,
} from "$lib/core/pipeline/graph";
import { applyAutoLayoutToGraph } from "$lib/core/pipeline/layout";
import {
  PipelineExecutor,
  type PipelineRunSnapshot,
  type NodeExecutionState,
  type EdgeExecutionState,
} from "$lib/core/pipeline/PipelineExecutor";
import type { CanvasItem } from "$lib/stores/layoutStore";
import {
  particlesTomlInput,
  verticesTomlInput,
  modelNameInput,
} from "$lib/stores/workspaceInputsStore";

interface PipelineGraphUiState {
  graph: PipelineGraph;
  lastValidation: ConnectionValidation | null;
}

interface PipelineExecutionUiState {
  run: PipelineRunSnapshot;
  selectedEdgeId: string | null;
}

const engine = new PipelineGraphStore(createEmptyPipelineGraph());
const executor = new PipelineExecutor({
  baseInputsProvider: () => ({
    particlesToml: get(particlesTomlInput),
    verticesToml: get(verticesTomlInput),
    modelName: get(modelNameInput),
  }),
});

const _graphState = writable<PipelineGraphUiState>({
  graph: createEmptyPipelineGraph(),
  lastValidation: null,
});

const _executionState = writable<PipelineExecutionUiState>({
  run: executor.getSnapshot(),
  selectedEdgeId: null,
});

engine.subscribe((snapshot) => {
  _graphState.set({
    graph: snapshot.graph,
    lastValidation: snapshot.lastValidation,
  });
});

executor.subscribe((snapshot) => {
  _executionState.update((state) => ({
    ...state,
    run: snapshot,
  }));
});

export const pipelineGraphState = {
  subscribe: _graphState.subscribe,
};

export const pipelineGraph = derived(_graphState, ($state) => $state.graph);

export const pipelineNodes = derived(pipelineGraph, ($graph) =>
  Object.values($graph.nodes).sort((a, b) => a.id.localeCompare(b.id)),
);

export const pipelineEdges = derived(pipelineGraph, ($graph) =>
  Object.values($graph.edges).sort((a, b) => a.id.localeCompare(b.id)),
);

export const selectedPipelineNode = derived(_graphState, ($state) => {
  const selectedId = $state.graph.selectedNodeId;
  if (!selectedId) return null;
  return $state.graph.nodes[selectedId] ?? null;
});

export const pipelineLastValidation = derived(
  _graphState,
  ($state) => $state.lastValidation,
);

export const pipelineRunState = derived(_executionState, ($state) => $state.run);

export const pipelineNodeExecutionStates = derived(
  pipelineRunState,
  ($run) => $run.nodeStates,
);

export const pipelineEdgeExecutionStates = derived(
  pipelineRunState,
  ($run) => $run.edgeStates,
);

export const pipelineEdgePayloads = derived(
  pipelineRunState,
  ($run) => $run.edgePayloads,
);

export const selectedPipelineEdgeId = derived(
  _executionState,
  ($state) => $state.selectedEdgeId,
);

export const selectedPipelineEdgePayload = derived(
  [_executionState],
  ([$state]) => {
    const edgeId = $state.selectedEdgeId;
    if (!edgeId) return null;
    return {
      edgeId,
      payload: $state.run.edgePayloads[edgeId],
      edgeState: $state.run.edgeStates[edgeId] ?? null,
    };
  },
);

function dispatch(action: PipelineGraphAction): PipelineGraphStoreSnapshot {
  return engine.dispatch(action);
}

export function addPipelineNode(nodeType: PipelineNodeType, position: WorldPoint): void {
  dispatch({
    type: "ADD_NODE",
    nodeType,
    position,
  });
}

export function removePipelineNode(nodeId: string): void {
  dispatch({
    type: "REMOVE_NODE",
    nodeId,
  });
  executor.clearRuntimeState();
}

export function movePipelineNode(nodeId: string, position: WorldPoint): void {
  dispatch({
    type: "MOVE_NODE",
    nodeId,
    position,
  });
}

export function selectPipelineNode(nodeId: string | null): void {
  dispatch({
    type: "SELECT_NODE",
    nodeId,
  });
}

export function updatePipelineNodeParameters(
  nodeId: string,
  patch: Record<string, PipelineParameterValue>,
): void {
  const snapshot = dispatch({
    type: "SET_NODE_PARAMS",
    nodeId,
    patch,
  });
  executor.markNodeStale(snapshot.graph, nodeId);
}

export function addPipelineEdge(
  sourceNodeId: string,
  sourcePortId: string,
  targetNodeId: string,
  targetPortId: string,
): ConnectionValidation {
  const result = engine.dispatch({
    type: "ADD_EDGE",
    sourceNodeId,
    sourcePortId,
    targetNodeId,
    targetPortId,
  });
  if (result.lastValidation?.ok) {
    executor.markNodeStale(result.graph, targetNodeId);
  }
  return result.lastValidation ?? { ok: true };
}

export function removePipelineEdge(edgeId: string): void {
  dispatch({
    type: "REMOVE_EDGE",
    edgeId,
  });
  executor.clearRuntimeState();
}

export function clearPipelineGraph(): void {
  dispatch({ type: "RESET" });
  executor.clearRuntimeState();
}

export function replacePipelineGraph(graph: PipelineGraph): void {
  dispatch({
    type: "REPLACE_GRAPH",
    graph,
  });
  executor.clearRuntimeState();
}

export function runPipelineAutoLayout(obstacles: CanvasItem[]): void {
  const snapshot = engine.getSnapshot();
  const formatted = applyAutoLayoutToGraph(snapshot.graph, obstacles);
  replacePipelineGraph(formatted);
}

export function exportPipelineGraphJson(): string {
  return serializeGraph(engine.getSnapshot().graph);
}

export function importPipelineGraphJson(json: string): void {
  const graph = deserializeGraph(json);
  replacePipelineGraph(graph);
}

export async function runPipelineExecution(): Promise<PipelineRunSnapshot> {
  return executor.executePipeline(get(pipelineGraph));
}

export function clearPipelineRuntimeState(): void {
  executor.clearRuntimeState();
}

export function markPipelineNodeStale(nodeId: string): void {
  executor.markNodeStale(get(pipelineGraph), nodeId);
}

export function selectPipelineEdge(edgeId: string | null): void {
  _executionState.update((state) => ({
    ...state,
    selectedEdgeId: edgeId,
  }));
}

export const PIPELINE_NODE_TYPES: PipelineNodeType[] = [
  "model_source",
  "reaction_builder",
  "monte_carlo_generator",
  "detector_simulation",
  "observable_script",
  "histogram_filler",
  "export_sink",
  "custom_rhai_script",
];

export function getPipelineNodeDefinition(nodeType: PipelineNodeType) {
  return NODE_DEFINITIONS[nodeType];
}

export function seedPipelineGraph(): void {
  const graph = createEmptyPipelineGraph();

  const modelNode = createPipelineNode("model_source", { x: 120, y: 140 });
  graph.nodes[modelNode.id] = modelNode;

  const reactionNode = createPipelineNode("reaction_builder", { x: 480, y: 140 });
  graph.nodes[reactionNode.id] = reactionNode;

  const stringNode = createPipelineNode("custom_rhai_script", { x: 480, y: 360 });
  graph.nodes[stringNode.id] = stringNode;

  const modelOutputPort = modelNode.outputs.find((port) => port.key === "theory");
  const reactionInputPort = reactionNode.inputs.find((port) => port.key === "theory_in");

  if (modelOutputPort && reactionInputPort) {
    const edgeResult = addEdge(graph, {
      sourceNodeId: modelNode.id,
      sourcePortId: modelOutputPort.id,
      targetNodeId: reactionNode.id,
      targetPortId: reactionInputPort.id,
    });
    replacePipelineGraph(edgeResult.graph);
    return;
  }

  replacePipelineGraph(graph);
}

export type {
  PipelineNode,
  PipelinePort,
  PipelineDataType,
  PipelineNodeType,
  PipelineGraph,
};
