import {
  createEmptyPipelineGraph,
  createPipelineNode,
  deserializeGraph,
  serializeGraph,
  type PipelineGraph,
  type PipelineNode,
} from "$lib/core/pipeline/graph";

const PIPELINE_FILE_EXTENSION = ".spire-pipeline";

export interface PipelineFileEnvelope {
  schema: "spire.pipeline.v1";
  exportedAt: string;
  graph: PipelineGraph;
}

export interface PipelineImportOptions {
  regenerateIds?: boolean;
}

export function exportPipeline(graph: PipelineGraph): string {
  const envelope: PipelineFileEnvelope = {
    schema: "spire.pipeline.v1",
    exportedAt: new Date().toISOString(),
    graph,
  };
  return JSON.stringify(envelope, null, 2);
}

function cloneNodeWithNewId(node: PipelineNode): PipelineNode {
  const regenerated = createPipelineNode(node.type, { ...node.position });

  const inputMap = new Map<string, string>();
  for (const input of node.inputs) {
    const candidate = regenerated.inputs.find((item) => item.key === input.key);
    if (candidate) {
      inputMap.set(input.id, candidate.id);
    }
  }

  const outputMap = new Map<string, string>();
  for (const output of node.outputs) {
    const candidate = regenerated.outputs.find((item) => item.key === output.key);
    if (candidate) {
      outputMap.set(output.id, candidate.id);
    }
  }

  regenerated.label = node.label;
  regenerated.size = { ...node.size };
  regenerated.parameters = { ...node.parameters };

  // Preserve predictable port ordering and metadata.
  regenerated.inputs = node.inputs.map((input) => {
    const replacement = regenerated.inputs.find((item) => item.key === input.key);
    return {
      ...input,
      id: replacement?.id ?? input.id,
    };
  });

  regenerated.outputs = node.outputs.map((output) => {
    const replacement = regenerated.outputs.find((item) => item.key === output.key);
    return {
      ...output,
      id: replacement?.id ?? output.id,
    };
  });

  return {
    ...regenerated,
    inputs: regenerated.inputs,
    outputs: regenerated.outputs,
  };
}

export function importPipeline(
  jsonString: string,
  options?: PipelineImportOptions,
): PipelineGraph {
  const parsed = JSON.parse(jsonString) as Partial<PipelineFileEnvelope> | PipelineGraph;

  const rawGraph = (() => {
    if (
      parsed &&
      typeof parsed === "object" &&
      "schema" in parsed &&
      (parsed as PipelineFileEnvelope).schema === "spire.pipeline.v1" &&
      "graph" in parsed
    ) {
      return (parsed as PipelineFileEnvelope).graph;
    }

    if (parsed && typeof parsed === "object" && "nodes" in parsed && "edges" in parsed) {
      return parsed as PipelineGraph;
    }

    throw new Error("Invalid pipeline file format.");
  })();

  const normalised = deserializeGraph(serializeGraph(rawGraph));
  if (options?.regenerateIds === false) {
    return normalised;
  }

  const rebuilt = createEmptyPipelineGraph();
  const nodeIdMap = new Map<string, string>();
  const portIdMap = new Map<string, string>();

  const nodeEntries = Object.values(normalised.nodes).sort((a, b) => a.id.localeCompare(b.id));
  for (const node of nodeEntries) {
    const cloned = cloneNodeWithNewId(node);
    rebuilt.nodes[cloned.id] = cloned;
    nodeIdMap.set(node.id, cloned.id);

    for (const idx of node.inputs.keys()) {
      portIdMap.set(node.inputs[idx].id, cloned.inputs[idx].id);
    }
    for (const idx of node.outputs.keys()) {
      portIdMap.set(node.outputs[idx].id, cloned.outputs[idx].id);
    }
  }

  const edgeEntries = Object.values(normalised.edges).sort((a, b) => a.id.localeCompare(b.id));
  for (const edge of edgeEntries) {
    const sourceNodeId = nodeIdMap.get(edge.sourceNodeId);
    const targetNodeId = nodeIdMap.get(edge.targetNodeId);
    const sourcePortId = portIdMap.get(edge.sourcePortId);
    const targetPortId = portIdMap.get(edge.targetPortId);

    if (!sourceNodeId || !targetNodeId || !sourcePortId || !targetPortId) {
      continue;
    }

    const nextEdgeId = `pipe-edge-import-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
    rebuilt.edges[nextEdgeId] = {
      id: nextEdgeId,
      sourceNodeId,
      sourcePortId,
      targetNodeId,
      targetPortId,
    };
  }

  rebuilt.selectedNodeId = null;
  rebuilt.updatedAt = new Date().toISOString();

  return rebuilt;
}

export function downloadPipelineToFile(graph: PipelineGraph, suggestedName = "pipeline"): void {
  const safeName = suggestedName.replace(/[^a-zA-Z0-9_-]/g, "_");
  const blob = new Blob([exportPipeline(graph)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `${safeName}${PIPELINE_FILE_EXTENSION}`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

export async function loadPipelineFromFile(file: File): Promise<PipelineGraph> {
  const content = await file.text();
  return importPipeline(content);
}

export function openPipelineFilePicker(): Promise<PipelineGraph | null> {
  return new Promise((resolve, reject) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = `${PIPELINE_FILE_EXTENSION},application/json`;
    input.onchange = async () => {
      try {
        const file = input.files?.[0];
        if (!file) {
          resolve(null);
          return;
        }
        const graph = await loadPipelineFromFile(file);
        resolve(graph);
      } catch (error) {
        reject(error);
      }
    };
    input.click();
  });
}
