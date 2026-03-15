import { derived, writable } from "svelte/store";
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
} from "$lib/core/pipeline/graph";
import { applyAutoLayoutToGraph } from "$lib/core/pipeline/layout";
import type { CanvasItem } from "$lib/stores/layoutStore";

interface PipelineGraphUiState {
  graph: PipelineGraph;
  lastValidation: ConnectionValidation | null;
}

const engine = new PipelineGraphStore(createEmptyPipelineGraph());

const _graphState = writable<PipelineGraphUiState>({
  graph: createEmptyPipelineGraph(),
  lastValidation: null,
});

engine.subscribe((snapshot) => {
  _graphState.set({
    graph: snapshot.graph,
    lastValidation: snapshot.lastValidation,
  });
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

function dispatch(action: PipelineGraphAction): void {
  engine.dispatch(action);
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
  dispatch({
    type: "SET_NODE_PARAMS",
    nodeId,
    patch,
  });
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
  return result.lastValidation ?? { ok: true };
}

export function removePipelineEdge(edgeId: string): void {
  dispatch({
    type: "REMOVE_EDGE",
    edgeId,
  });
}

export function clearPipelineGraph(): void {
  dispatch({ type: "RESET" });
}

export function replacePipelineGraph(graph: PipelineGraph): void {
  dispatch({
    type: "REPLACE_GRAPH",
    graph,
  });
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
