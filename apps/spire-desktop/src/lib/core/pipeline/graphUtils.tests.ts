import {
  TopologyError,
  collectDownstreamNodeIds,
  markStale,
  topologicalSortNodeIdsOrThrow,
} from "$lib/core/pipeline/graphUtils";
import {
  addEdge,
  createEmptyPipelineGraph,
  createPipelineNode,
  type PipelineGraph,
} from "$lib/core/pipeline/graph";

function assert(condition: boolean, message: string): void {
  if (!condition) {
    throw new Error(`graphUtils test failed: ${message}`);
  }
}

function makeSimpleGraph(): PipelineGraph {
  const graph = createEmptyPipelineGraph();

  const model = createPipelineNode("model_source", { x: 0, y: 0 });
  const reaction = createPipelineNode("reaction_builder", { x: 320, y: 0 });
  const mc = createPipelineNode("monte_carlo_generator", { x: 640, y: 0 });

  graph.nodes[model.id] = model;
  graph.nodes[reaction.id] = reaction;
  graph.nodes[mc.id] = mc;

  const edge1 = addEdge(graph, {
    sourceNodeId: model.id,
    sourcePortId: model.outputs[0].id,
    targetNodeId: reaction.id,
    targetPortId: reaction.inputs[0].id,
  });

  const edge2 = addEdge(edge1.graph, {
    sourceNodeId: reaction.id,
    sourcePortId: reaction.outputs[0].id,
    targetNodeId: mc.id,
    targetPortId: mc.inputs[0].id,
  });

  return edge2.graph;
}

export function runGraphUtilsUnitTests(): void {
  const dag = makeSimpleGraph();

  const order = topologicalSortNodeIdsOrThrow(dag);
  assert(order.length === 3, "Topological sort should include all nodes");

  const downstream = collectDownstreamNodeIds(dag, order[0]);
  assert(downstream.size === 2, "Downstream collection should include descendants");

  const edgeIds = Object.keys(dag.edges);
  const staleResult = markStale(dag, order[0], {
    staleNodeIds: new Set<string>(),
    nodeOutputs: {
      [order[0]]: { ok: true },
      [order[1]]: { ok: true },
      [order[2]]: { ok: true },
    },
    edgePayloads: {
      [edgeIds[0]]: { payload: 1 },
      [edgeIds[1]]: { payload: 2 },
    },
  });

  assert(staleResult.staleNodeIds.size === 3, "markStale should mark source and all downstream nodes");
  assert(Object.keys(staleResult.nodeOutputs).length === 0, "markStale should clear cached node outputs");
  assert(Object.keys(staleResult.edgePayloads).length === 0, "markStale should clear edge payloads");

  let cycleDetected = false;
  try {
    const graphWithCycle = makeSimpleGraph();
    const nodes = Object.values(graphWithCycle.nodes);
    const source = nodes[2];
    const target = nodes[0];
    const maybe = addEdge(graphWithCycle, {
      sourceNodeId: source.id,
      sourcePortId: source.outputs[0].id,
      targetNodeId: target.id,
      targetPortId: target.inputs[0].id,
    });

    const cyclicGraph = maybe.validation.ok ? maybe.graph : graphWithCycle;
    if (maybe.validation.ok) {
      topologicalSortNodeIdsOrThrow(cyclicGraph);
    } else {
      cycleDetected = true;
    }
  } catch (error) {
    cycleDetected = error instanceof TopologyError;
  }

  assert(cycleDetected, "Cycle should be detected by validation or topological sort");
}
