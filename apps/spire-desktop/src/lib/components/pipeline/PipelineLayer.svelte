<!--
  @component
  Pipeline graph interaction layer rendered in canvas world-space coordinates.

  @param {HTMLDivElement} canvasElement Canvas host used for client↔world coordinate transforms.
  @param {number} panX Current canvas X pan in screen pixels.
  @param {number} panY Current canvas Y pan in screen pixels.
  @param {number} zoom Current canvas zoom factor.

  @reads pipelineGraph, pipelineNodes, pipelineEdges, pipelineRunState, pipelineNodeExecutionStates, pipelineEdgeExecutionStates, selectedPipelineNode, selectedPipelineEdgePayload
  @writes pipelineGraph (via add/move/remove/select/update actions), pipeline runtime state (via run/clear actions), workspace input stores and observableScripts (through syncGraphToPhysicsStores)
-->
<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { get } from "svelte/store";
  import PipelineNodeCard from "$lib/components/pipeline/PipelineNode.svelte";
  import PipelineInspector from "$lib/components/pipeline/PipelineInspector.svelte";
  import {
    PIPELINE_NODE_TYPES,
    addPipelineEdge,
    addPipelineNode,
    clearPipelineRuntimeState,
    movePipelineNode,
    pipelineEdges,
    pipelineEdgeExecutionStates,
    pipelineGraph,
    pipelineLastValidation,
    pipelineNodes,
    pipelineNodeExecutionStates,
    pipelineRunState,
    removePipelineEdge,
    removePipelineNode,
    runPipelineAutoLayout,
    runPipelineExecution,
    selectPipelineEdge,
    selectPipelineNode,
    selectedPipelineEdgeId,
    selectedPipelineEdgePayload,
    selectedPipelineNode,
    updatePipelineNodeParameters,
    getPipelineNodeDefinition,
    type PipelineNode,
    type PipelineNodeType,
    type PipelinePort,
  } from "$lib/stores/pipelineGraphStore";
  import { canvasItems } from "$lib/stores/layoutStore";
  import { modelNameInput, initialIdsInput, finalIdsInput, cmsEnergyInput, maxLoopOrderInput } from "$lib/stores/workspaceInputsStore";
  import { theoreticalModel, observableScripts } from "$lib/stores/physicsStore";

  export let canvasElement: HTMLDivElement;
  export let panX = 0;
  export let panY = 0;
  export let zoom = 1;

  interface NodeDragState {
    nodeId: string;
    offsetX: number;
    offsetY: number;
  }

  interface PortDragState {
    nodeId: string;
    portId: string;
    direction: "input" | "output";
    worldX: number;
    worldY: number;
    pointerX: number;
    pointerY: number;
  }

  let nodeDrag: NodeDragState | null = null;
  let portDrag: PortDragState | null = null;
  let invalidFlash = false;
  let invalidFlashTimer: ReturnType<typeof setTimeout> | null = null;

  const EDGE_PROXIMITY_RADIUS = 16;
  const PORT_INSET = 6;

  function worldFromClient(clientX: number, clientY: number): { x: number; y: number } {
    const rect = canvasElement.getBoundingClientRect();
    return {
      x: (clientX - rect.left - panX) / zoom,
      y: (clientY - rect.top - panY) / zoom,
    };
  }

  function nodePortTop(node: PipelineNode, index: number, count: number): number {
    const safeCount = Math.max(1, count);
    const topPadding = 38;
    const usableHeight = Math.max(42, node.size.height - 52);
    return node.position.y + topPadding + (usableHeight * (index + 0.5)) / safeCount;
  }

  function resolvePortAnchor(node: PipelineNode, port: PipelinePort): { x: number; y: number } {
    if (port.direction === "input") {
      const index = node.inputs.findIndex((candidate) => candidate.id === port.id);
      const y = nodePortTop(node, index < 0 ? 0 : index, node.inputs.length);
      return { x: node.position.x + PORT_INSET, y };
    }

    const index = node.outputs.findIndex((candidate) => candidate.id === port.id);
    const y = nodePortTop(node, index < 0 ? 0 : index, node.outputs.length);
    return { x: node.position.x + node.size.width - PORT_INSET, y };
  }

  function drawBezier(x1: number, y1: number, x2: number, y2: number): string {
    const dx = Math.abs(x2 - x1);
    const offset = Math.max(48, Math.min(240, dx * 0.46));
    const c1x = x1 + offset;
    const c2x = x2 - offset;
    return `M ${x1} ${y1} C ${c1x} ${y1}, ${c2x} ${y2}, ${x2} ${y2}`;
  }

  function findNode(nodeId: string): PipelineNode | null {
    return get(pipelineGraph).nodes[nodeId] ?? null;
  }

  function findPort(nodeId: string, portId: string): PipelinePort | null {
    const node = findNode(nodeId);
    if (!node) return null;
    return [...node.inputs, ...node.outputs].find((port) => port.id === portId) ?? null;
  }

  function findNearestTargetPort(
    pointerX: number,
    pointerY: number,
    expectedDirection: "input" | "output",
  ): { node: PipelineNode; port: PipelinePort; distance: number } | null {
    let best: { node: PipelineNode; port: PipelinePort; distance: number } | null = null;

    for (const node of $pipelineNodes) {
      const candidates = expectedDirection === "input" ? node.inputs : node.outputs;
      for (const port of candidates) {
        const anchor = resolvePortAnchor(node, port);
        const dx = anchor.x - pointerX;
        const dy = anchor.y - pointerY;
        const dist = Math.sqrt(dx * dx + dy * dy);
        if (dist > EDGE_PROXIMITY_RADIUS) {
          continue;
        }
        if (!best || dist < best.distance) {
          best = { node, port, distance: dist };
        }
      }
    }

    return best;
  }

  function beginNodeDrag(event: CustomEvent<{ nodeId: string; clientX: number; clientY: number }>): void {
    if ($pipelineRunState.isRunning) return;
    if (portDrag) return;
    const node = findNode(event.detail.nodeId);
    if (!node) return;

    const world = worldFromClient(event.detail.clientX, event.detail.clientY);
    nodeDrag = {
      nodeId: node.id,
      offsetX: world.x - node.position.x,
      offsetY: world.y - node.position.y,
    };
  }

  function beginPortDrag(
    event: CustomEvent<{
      nodeId: string;
      portId: string;
      direction: "input" | "output";
      clientX: number;
      clientY: number;
    }>,
  ): void {
    if ($pipelineRunState.isRunning) return;
    const node = findNode(event.detail.nodeId);
    const port = findPort(event.detail.nodeId, event.detail.portId);
    if (!node || !port) return;

    const anchor = resolvePortAnchor(node, port);
    const world = worldFromClient(event.detail.clientX, event.detail.clientY);

    portDrag = {
      nodeId: node.id,
      portId: port.id,
      direction: port.direction,
      worldX: anchor.x,
      worldY: anchor.y,
      pointerX: world.x,
      pointerY: world.y,
    };
  }

  function clearInvalidFlash(): void {
    invalidFlash = false;
    if (invalidFlashTimer !== null) {
      clearTimeout(invalidFlashTimer);
      invalidFlashTimer = null;
    }
  }

  function flashInvalid(): void {
    clearInvalidFlash();
    invalidFlash = true;
    invalidFlashTimer = setTimeout(() => {
      invalidFlash = false;
      invalidFlashTimer = null;
    }, 320);
  }

  function handleWindowMouseMove(event: MouseEvent): void {
    const world = worldFromClient(event.clientX, event.clientY);

    if (nodeDrag) {
      movePipelineNode(nodeDrag.nodeId, {
        x: world.x - nodeDrag.offsetX,
        y: world.y - nodeDrag.offsetY,
      });
      return;
    }

    if (portDrag) {
      portDrag = {
        ...portDrag,
        pointerX: world.x,
        pointerY: world.y,
      };
    }
  }

  function handleWindowMouseUp(): void {
    if (nodeDrag) {
      nodeDrag = null;
      return;
    }

    if (!portDrag) {
      return;
    }

    const sourceNode = findNode(portDrag.nodeId);
    const sourcePort = sourceNode
      ? [...sourceNode.inputs, ...sourceNode.outputs].find((port) => port.id === portDrag?.portId) ?? null
      : null;

    if (!sourceNode || !sourcePort) {
      portDrag = null;
      return;
    }

    const expectedDirection = sourcePort.direction === "output" ? "input" : "output";
    const hit = findNearestTargetPort(portDrag.pointerX, portDrag.pointerY, expectedDirection);

    if (!hit) {
      flashInvalid();
      portDrag = null;
      return;
    }

    const sourceIsOutput = sourcePort.direction === "output";
    const sourceNodeId = sourceIsOutput ? sourceNode.id : hit.node.id;
    const sourcePortId = sourceIsOutput ? sourcePort.id : hit.port.id;
    const targetNodeId = sourceIsOutput ? hit.node.id : sourceNode.id;
    const targetPortId = sourceIsOutput ? hit.port.id : sourcePort.id;

    const validation = addPipelineEdge(sourceNodeId, sourcePortId, targetNodeId, targetPortId);
    if (!validation.ok) {
      flashInvalid();
    }

    portDrag = null;
  }

  function handleCanvasMouseDown(event: MouseEvent): void {
    if (event.button !== 0) return;
    if (!(event.target instanceof HTMLElement)) return;

    if (event.target.closest(".pipeline-node") || event.target.closest(".pipeline-inspector") || event.target.closest(".pipeline-toolbar")) {
      return;
    }

    selectPipelineNode(null);
    selectPipelineEdge(null);
  }

  function removeSelectedNode(): void {
    if ($pipelineRunState.isRunning) return;
    if (!$selectedPipelineNode) return;
    removePipelineNode($selectedPipelineNode.id);
  }

  function createNode(nodeType: PipelineNodeType): void {
    if ($pipelineRunState.isRunning) return;
    const vp = {
      x: (-panX + 200) / zoom,
      y: (-panY + 120) / zoom,
    };
    addPipelineNode(nodeType, vp);
  }

  function syncGraphToPhysicsStores(): void {
    for (const node of $pipelineNodes) {
      if (node.type === "model_source") {
        const targetModel = String(node.parameters.modelName ?? "").trim();
        if (targetModel.length > 0 && targetModel !== get(modelNameInput)) {
          modelNameInput.set(targetModel);
        }
      }

      if (node.type === "reaction_builder") {
        const initialText = String(node.parameters.initialState ?? "");
        const finalText = String(node.parameters.finalState ?? "");
        const cms = Number(node.parameters.cmsEnergy ?? get(cmsEnergyInput));
        const maxLoop = Number(node.parameters.maxLoopOrder ?? get(maxLoopOrderInput));

        const parseIds = (value: string): string[] =>
          value
            .split(",")
            .map((part) => part.trim())
            .filter((part) => part.length > 0);

        const nextInitial = parseIds(initialText);
        const nextFinal = parseIds(finalText);

        if (nextInitial.length > 0) {
          initialIdsInput.set(nextInitial);
        }
        if (nextFinal.length > 0) {
          finalIdsInput.set(nextFinal);
        }
        if (Number.isFinite(cms) && cms > 0) {
          cmsEnergyInput.set(cms);
        }
        if (Number.isFinite(maxLoop) && maxLoop >= 0) {
          maxLoopOrderInput.set(Math.round(maxLoop));
        }
      }

      if (node.type === "observable_script") {
        const name = String(node.parameters.name ?? "Observable").trim();
        const script = String(node.parameters.script ?? "").trim();
        if (script.length === 0) continue;

        observableScripts.update((existing) => {
          const idx = existing.findIndex((entry) => entry.name === name);
          if (idx >= 0) {
            const next = [...existing];
            next[idx] = { ...next[idx], script };
            return next;
          }
          return [
            ...existing,
            {
              name,
              script,
              isValid: true,
            },
          ];
        });
      }
    }
  }

  function handleInspectorParamChange(
    event: CustomEvent<{ nodeId: string; key: string; value: string | number | boolean | string[] }>,
  ): void {
    updatePipelineNodeParameters(event.detail.nodeId, {
      [event.detail.key]: event.detail.value,
    });
  }

  function triggerAutoLayout(): void {
    if ($pipelineRunState.isRunning) return;
    runPipelineAutoLayout(get(canvasItems));
  }

  async function handleRunPipeline(): Promise<void> {
    if ($pipelineRunState.isRunning) return;
    try {
      await runPipelineExecution();
    } catch {
      // Runtime states in the store contain diagnostics.
    }
  }

  function clickEdge(edgeId: string): void {
    selectPipelineEdge($selectedPipelineEdgeId === edgeId ? null : edgeId);
  }

  function edgeClass(edgeId: string): string {
    const status = $pipelineEdgeExecutionStates[edgeId]?.status ?? "idle";
    if (status === "running") return "edge-path edge-running";
    if (status === "completed") return "edge-path edge-completed";
    if (status === "error") return "edge-path edge-error";
    if (status === "stale") return "edge-path edge-stale";
    return "edge-path";
  }

  function connectionPath(edgeId: string): string {
    const edge = $pipelineEdges.find((candidate) => candidate.id === edgeId);
    if (!edge) return "";
    const sourceNode = findNode(edge.sourceNodeId);
    const targetNode = findNode(edge.targetNodeId);
    if (!sourceNode || !targetNode) return "";
    const sourcePort = findPort(edge.sourceNodeId, edge.sourcePortId);
    const targetPort = findPort(edge.targetNodeId, edge.targetPortId);
    if (!sourcePort || !targetPort) return "";

    const start = resolvePortAnchor(sourceNode, sourcePort);
    const end = resolvePortAnchor(targetNode, targetPort);
    return drawBezier(start.x, start.y, end.x, end.y);
  }

  function ghostConnectionPath(): string {
    if (!portDrag) return "";
    const activePortDrag = portDrag;
    const sourceNode = findNode(activePortDrag.nodeId);
    const sourcePort = sourceNode
      ? [...sourceNode.inputs, ...sourceNode.outputs].find((port) => port.id === activePortDrag.portId) ?? null
      : null;

    if (!sourceNode || !sourcePort) return "";

    const startAnchor = resolvePortAnchor(sourceNode, sourcePort);
    return drawBezier(startAnchor.x, startAnchor.y, portDrag.pointerX, portDrag.pointerY);
  }

  $: modelOptions = (() => {
    const options = new Set<string>();
    const activeName = get(modelNameInput);
    if (activeName.trim().length > 0) {
      options.add(activeName.trim());
    }
    if ($theoreticalModel?.name) {
      options.add($theoreticalModel.name.trim());
    }
    options.add("Standard Model");
    return Array.from(options).filter((name) => name.length > 0);
  })();

  $: if ($pipelineLastValidation && !$pipelineLastValidation.ok) {
    flashInvalid();
  }

  $: syncGraphToPhysicsStores();

  onMount(() => {
    window.addEventListener("mousemove", handleWindowMouseMove);
    window.addEventListener("mouseup", handleWindowMouseUp);
  });

  onDestroy(() => {
    window.removeEventListener("mousemove", handleWindowMouseMove);
    window.removeEventListener("mouseup", handleWindowMouseUp);
    clearInvalidFlash();
  });
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
  class="pipeline-layer"
  class:invalid-flash={invalidFlash}
  on:mousedown={handleCanvasMouseDown}
>
  <svg class="pipeline-svg" aria-hidden="true">
    <g>
      {#each $pipelineEdges as edge (edge.id)}
        <!-- svelte-ignore a11y-no-static-element-interactions -->
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <path
          class={edgeClass(edge.id)}
          d={connectionPath(edge.id)}
          on:dblclick={() => removePipelineEdge(edge.id)}
          on:click={() => clickEdge(edge.id)}
        ></path>
      {/each}

      {#if portDrag}
        <path class="edge-ghost" d={ghostConnectionPath()}></path>
      {/if}
    </g>
  </svg>

  <div class="pipeline-nodes">
    {#each $pipelineNodes as node (node.id)}
      <PipelineNodeCard
        {node}
        selected={node.id === $pipelineGraph.selectedNodeId}
        status={$pipelineNodeExecutionStates[node.id]?.status ?? "idle"}
        errorMessage={$pipelineNodeExecutionStates[node.id]?.error ?? null}
        on:select={(event) => {
          selectPipelineNode(event.detail.nodeId);
          selectPipelineEdge(null);
        }}
        on:remove={(event) => removePipelineNode(event.detail.nodeId)}
        on:startdrag={beginNodeDrag}
        on:portdragstart={beginPortDrag}
      />
    {/each}
  </div>

  <div class="pipeline-toolbar">
    <div class="toolbar-title">Pipeline Graph</div>
    <div class="toolbar-actions">
      {#each PIPELINE_NODE_TYPES as nodeType}
        <button type="button" class="toolbar-btn" on:click={() => createNode(nodeType)} disabled={$pipelineRunState.isRunning}>
          + {getPipelineNodeDefinition(nodeType).label}
        </button>
      {/each}
      <button type="button" class="toolbar-btn accent" on:click={triggerAutoLayout} disabled={$pipelineRunState.isRunning}>
        Format Graph
      </button>
      <button type="button" class="toolbar-btn accent" on:click={handleRunPipeline} disabled={$pipelineRunState.isRunning}>
        {$pipelineRunState.isRunning ? "Running…" : "Run Pipeline"}
      </button>
      <button type="button" class="toolbar-btn" on:click={clearPipelineRuntimeState} disabled={$pipelineRunState.isRunning}>
        Clear Runtime
      </button>
      <button type="button" class="toolbar-btn" on:click={removeSelectedNode} disabled={!$selectedPipelineNode || $pipelineRunState.isRunning}>
        Remove Selected
      </button>
    </div>
  </div>

  <PipelineInspector
    node={$selectedPipelineNode}
    {modelOptions}
    selectedEdgeId={$selectedPipelineEdgePayload?.edgeId ?? null}
    selectedEdgePayload={$selectedPipelineEdgePayload?.payload}
    selectedEdgeState={$selectedPipelineEdgePayload?.edgeState ?? null}
    on:paramchange={handleInspectorParamChange}
  />
</div>

<style>
  .pipeline-layer {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 3;
  }

  .pipeline-layer.invalid-flash {
    animation: pipeline-invalid-flash 280ms ease;
  }

  @keyframes pipeline-invalid-flash {
    0% {
      box-shadow: inset 0 0 0 0 rgba(var(--color-error-rgb), 0);
    }
    30% {
      box-shadow: inset 0 0 0 2px rgba(var(--color-error-rgb), 0.58);
    }
    100% {
      box-shadow: inset 0 0 0 0 rgba(var(--color-error-rgb), 0);
    }
  }

  .pipeline-svg {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: visible;
    pointer-events: none;
  }

  .edge-path {
    fill: none;
    stroke: rgba(var(--color-text-primary-rgb), 0.35);
    stroke-width: 2.1;
    pointer-events: auto;
    cursor: pointer;
  }

  .edge-running {
    stroke: rgba(var(--color-accent-rgb), 0.95);
    stroke-dasharray: 10 10;
    animation: edge-flow 1s linear infinite;
  }

  .edge-completed {
    stroke: var(--color-success);
    stroke-dasharray: none;
  }

  .edge-error {
    stroke: var(--color-error);
    stroke-dasharray: 6 4;
  }

  .edge-stale {
    stroke: color-mix(in oklab, var(--color-warning) 58%, var(--color-text-muted));
    stroke-dasharray: 5 5;
  }

  .edge-path:hover {
    stroke: var(--color-accent-hover);
    stroke-width: 2.8;
  }

  .edge-ghost {
    fill: none;
    stroke: rgba(var(--color-text-primary-rgb), 0.45);
    stroke-width: 1.8;
    stroke-dasharray: 5 5;
  }

  @keyframes edge-flow {
    to {
      stroke-dashoffset: -20;
    }
  }

  .pipeline-nodes {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }

  .pipeline-nodes :global(.pipeline-node) {
    pointer-events: auto;
  }

  .pipeline-toolbar {
    position: absolute;
    top: 0.55rem;
    left: 0.55rem;
    width: min(34rem, 48vw);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    box-shadow: var(--shadow-elevated);
    background: color-mix(in oklab, var(--color-bg-base) 88%, black 12%);
    pointer-events: auto;
    z-index: 8;
  }

  .toolbar-title {
    padding: 0.45rem 0.6rem;
    border-bottom: 1px solid var(--color-border);
    color: var(--color-text-primary);
    font-family: var(--font-mono);
    font-size: 0.68rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .toolbar-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    padding: 0.46rem 0.55rem 0.58rem;
  }

  .toolbar-btn {
    border: 1px solid var(--color-border);
    background: var(--color-bg-surface);
    color: var(--color-text-muted);
    font-family: var(--font-mono);
    font-size: 0.64rem;
    padding: 0.2rem 0.4rem;
    line-height: 1.2;
    text-align: left;
    max-width: 11.5rem;
  }

  .toolbar-btn:hover:not(:disabled) {
    border-color: var(--color-accent);
    color: var(--color-text-primary);
  }

  .toolbar-btn.accent {
    color: var(--color-accent);
    border-color: color-mix(in oklab, var(--color-accent) 72%, var(--color-border));
  }

  .toolbar-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
