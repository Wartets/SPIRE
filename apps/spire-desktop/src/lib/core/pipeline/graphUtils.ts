import type { PipelineEdge, PipelineGraph } from "$lib/core/pipeline/graph";

export class TopologyError extends Error {
  public readonly cycleNodeIds: string[];

  public constructor(message: string, cycleNodeIds: string[] = []) {
    super(message);
    this.name = "TopologyError";
    this.cycleNodeIds = cycleNodeIds;
  }
}

export interface GraphAdjacency {
  outgoing: Map<string, Set<string>>;
  incoming: Map<string, Set<string>>;
}

export function buildAdjacency(graph: PipelineGraph): GraphAdjacency {
  const outgoing = new Map<string, Set<string>>();
  const incoming = new Map<string, Set<string>>();

  for (const nodeId of Object.keys(graph.nodes)) {
    outgoing.set(nodeId, new Set<string>());
    incoming.set(nodeId, new Set<string>());
  }

  for (const edge of Object.values(graph.edges)) {
    if (!graph.nodes[edge.sourceNodeId] || !graph.nodes[edge.targetNodeId]) {
      continue;
    }
    outgoing.get(edge.sourceNodeId)?.add(edge.targetNodeId);
    incoming.get(edge.targetNodeId)?.add(edge.sourceNodeId);
  }

  return { outgoing, incoming };
}

export function topologicalSortNodeIdsOrThrow(graph: PipelineGraph): string[] {
  const { outgoing, incoming } = buildAdjacency(graph);
  const inDegree = new Map<string, number>();

  for (const [nodeId, incomingSet] of incoming.entries()) {
    inDegree.set(nodeId, incomingSet.size);
  }

  const queue = Array.from(inDegree.entries())
    .filter(([, degree]) => degree === 0)
    .map(([nodeId]) => nodeId)
    .sort((a, b) => a.localeCompare(b));

  const order: string[] = [];

  while (queue.length > 0) {
    const nodeId = queue.shift();
    if (!nodeId) break;

    order.push(nodeId);

    const targets = Array.from(outgoing.get(nodeId) ?? []).sort((a, b) => a.localeCompare(b));
    for (const targetId of targets) {
      const nextDegree = (inDegree.get(targetId) ?? 0) - 1;
      inDegree.set(targetId, nextDegree);
      if (nextDegree === 0) {
        queue.push(targetId);
        queue.sort((a, b) => a.localeCompare(b));
      }
    }
  }

  const totalNodes = Object.keys(graph.nodes).length;
  if (order.length !== totalNodes) {
    const unresolved = Array.from(inDegree.entries())
      .filter(([, degree]) => degree > 0)
      .map(([nodeId]) => nodeId)
      .sort((a, b) => a.localeCompare(b));
    throw new TopologyError("Pipeline graph contains a dependency cycle.", unresolved);
  }

  return order;
}

export function collectDownstreamNodeIds(graph: PipelineGraph, nodeId: string): Set<string> {
  const { outgoing } = buildAdjacency(graph);
  const downstream = new Set<string>();
  const stack = [nodeId];

  while (stack.length > 0) {
    const current = stack.pop();
    if (!current) continue;
    const targets = outgoing.get(current);
    if (!targets) continue;

    for (const nextId of targets) {
      if (downstream.has(nextId)) {
        continue;
      }
      downstream.add(nextId);
      stack.push(nextId);
    }
  }

  return downstream;
}

export interface PipelineStaleState {
  staleNodeIds: Set<string>;
  edgePayloads: Record<string, unknown>;
  nodeOutputs: Record<string, Record<string, unknown>>;
}

export interface MarkStaleResult {
  staleNodeIds: Set<string>;
  clearedNodeIds: string[];
  clearedEdgeIds: string[];
  edgePayloads: Record<string, unknown>;
  nodeOutputs: Record<string, Record<string, unknown>>;
}

function edgeTargetsNode(edge: PipelineEdge, nodeIdSet: Set<string>): boolean {
  return nodeIdSet.has(edge.sourceNodeId) || nodeIdSet.has(edge.targetNodeId);
}

export function markStale(
  graph: PipelineGraph,
  dirtyNodeId: string,
  state: PipelineStaleState,
): MarkStaleResult {
  if (!graph.nodes[dirtyNodeId]) {
    return {
      staleNodeIds: new Set(state.staleNodeIds),
      clearedNodeIds: [],
      clearedEdgeIds: [],
      edgePayloads: { ...state.edgePayloads },
      nodeOutputs: { ...state.nodeOutputs },
    };
  }

  const impacted = collectDownstreamNodeIds(graph, dirtyNodeId);
  impacted.add(dirtyNodeId);

  const staleNodeIds = new Set(state.staleNodeIds);
  for (const nodeId of impacted) {
    staleNodeIds.add(nodeId);
  }

  const nodeOutputs: Record<string, Record<string, unknown>> = { ...state.nodeOutputs };
  const clearedNodeIds: string[] = [];
  for (const nodeId of impacted) {
    if (nodeId in nodeOutputs) {
      delete nodeOutputs[nodeId];
      clearedNodeIds.push(nodeId);
    }
  }

  const edgePayloads: Record<string, unknown> = { ...state.edgePayloads };
  const clearedEdgeIds: string[] = [];

  for (const edge of Object.values(graph.edges)) {
    if (!edgeTargetsNode(edge, impacted)) {
      continue;
    }
    if (edge.id in edgePayloads) {
      delete edgePayloads[edge.id];
      clearedEdgeIds.push(edge.id);
    }
  }

  return {
    staleNodeIds,
    clearedNodeIds: clearedNodeIds.sort((a, b) => a.localeCompare(b)),
    clearedEdgeIds: clearedEdgeIds.sort((a, b) => a.localeCompare(b)),
    edgePayloads,
    nodeOutputs,
  };
}
