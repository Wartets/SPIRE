import type { CanvasItem } from "$lib/stores/layoutStore";
import type { PipelineGraph, WorldPoint } from "./graph";

export interface GraphObstacle {
  x: number;
  y: number;
  width: number;
  height: number;
  padding?: number;
}

export interface AutoLayoutConfig {
  startX: number;
  startY: number;
  columnGap: number;
  rowGap: number;
  obstacleStep: number;
  maxObstacleIterations: number;
}

export interface AutoLayoutResult {
  positions: Record<string, WorldPoint>;
  depthByNode: Record<string, number>;
  order: string[];
}

const DEFAULT_CONFIG: AutoLayoutConfig = {
  startX: 120,
  startY: 120,
  columnGap: 340,
  rowGap: 190,
  obstacleStep: 96,
  maxObstacleIterations: 24,
};

function inflateObstacle(obstacle: GraphObstacle): GraphObstacle {
  const padding = obstacle.padding ?? 16;
  return {
    x: obstacle.x - padding,
    y: obstacle.y - padding,
    width: obstacle.width + padding * 2,
    height: obstacle.height + padding * 2,
    padding,
  };
}

function intersects(
  x: number,
  y: number,
  width: number,
  height: number,
  obstacle: GraphObstacle,
): boolean {
  return !(
    x + width < obstacle.x ||
    x > obstacle.x + obstacle.width ||
    y + height < obstacle.y ||
    y > obstacle.y + obstacle.height
  );
}

function buildAdjacency(graph: PipelineGraph): {
  outgoing: Map<string, Set<string>>;
  incomingCount: Map<string, number>;
} {
  const outgoing = new Map<string, Set<string>>();
  const incomingCount = new Map<string, number>();

  for (const nodeId of Object.keys(graph.nodes)) {
    outgoing.set(nodeId, new Set<string>());
    incomingCount.set(nodeId, 0);
  }

  for (const edge of Object.values(graph.edges)) {
    const out = outgoing.get(edge.sourceNodeId);
    if (out && !out.has(edge.targetNodeId)) {
      out.add(edge.targetNodeId);
      incomingCount.set(edge.targetNodeId, (incomingCount.get(edge.targetNodeId) ?? 0) + 1);
    }
  }

  return { outgoing, incomingCount };
}

function topologicalOrder(graph: PipelineGraph): string[] {
  const { outgoing, incomingCount } = buildAdjacency(graph);
  const queue = Array.from(incomingCount.entries())
    .filter(([, degree]) => degree === 0)
    .map(([nodeId]) => nodeId)
    .sort((a, b) => a.localeCompare(b));

  const order: string[] = [];
  while (queue.length > 0) {
    const nodeId = queue.shift();
    if (!nodeId) break;
    order.push(nodeId);

    const targets = Array.from(outgoing.get(nodeId) ?? []);
    for (const targetId of targets) {
      const next = (incomingCount.get(targetId) ?? 0) - 1;
      incomingCount.set(targetId, next);
      if (next === 0) {
        queue.push(targetId);
        queue.sort((a, b) => a.localeCompare(b));
      }
    }
  }

  if (order.length !== Object.keys(graph.nodes).length) {
    return Object.keys(graph.nodes).sort((a, b) => a.localeCompare(b));
  }

  return order;
}

function computeDepth(graph: PipelineGraph, order: string[]): Record<string, number> {
  const depthByNode: Record<string, number> = {};
  for (const nodeId of order) {
    depthByNode[nodeId] = 0;
  }

  const outgoing = new Map<string, string[]>();
  for (const nodeId of order) {
    outgoing.set(nodeId, []);
  }
  for (const edge of Object.values(graph.edges)) {
    const list = outgoing.get(edge.sourceNodeId);
    if (list) {
      list.push(edge.targetNodeId);
    }
  }

  for (const nodeId of order) {
    const children = outgoing.get(nodeId) ?? [];
    for (const childId of children) {
      const childDepth = depthByNode[childId] ?? 0;
      const candidate = (depthByNode[nodeId] ?? 0) + 1;
      if (candidate > childDepth) {
        depthByNode[childId] = candidate;
      }
    }
  }

  return depthByNode;
}

function avoidObstacles(
  x: number,
  y: number,
  width: number,
  height: number,
  obstacles: GraphObstacle[],
  config: AutoLayoutConfig,
): WorldPoint {
  let px = x;
  let py = y;

  for (let attempt = 0; attempt < config.maxObstacleIterations; attempt++) {
    const hit = obstacles.find((obstacle) => intersects(px, py, width, height, obstacle));
    if (!hit) {
      return { x: px, y: py };
    }

    const shiftDown = hit.y + hit.height + config.obstacleStep;
    const shiftRight = hit.x + hit.width + config.obstacleStep;

    if (attempt % 3 === 0) {
      py = shiftDown;
    } else if (attempt % 3 === 1) {
      px = shiftRight;
    } else {
      py += config.obstacleStep;
    }
  }

  return { x: px, y: py };
}

function avoidNodeCollisions(
  candidate: WorldPoint,
  width: number,
  height: number,
  placed: Array<{ id: string; x: number; y: number; w: number; h: number }>,
  config: AutoLayoutConfig,
): WorldPoint {
  let px = candidate.x;
  let py = candidate.y;

  for (let iteration = 0; iteration < 36; iteration++) {
    const hit = placed.find((other) =>
      intersects(px, py, width, height, {
        x: other.x - 12,
        y: other.y - 12,
        width: other.w + 24,
        height: other.h + 24,
      }),
    );

    if (!hit) {
      return { x: px, y: py };
    }

    py = hit.y + hit.h + Math.max(48, config.rowGap * 0.35);
  }

  return { x: px, y: py };
}

export function autoLayoutPipelineGraph(
  graph: PipelineGraph,
  widgetObstacles: CanvasItem[] = [],
  userConfig?: Partial<AutoLayoutConfig>,
): AutoLayoutResult {
  const config: AutoLayoutConfig = { ...DEFAULT_CONFIG, ...(userConfig ?? {}) };
  const order = topologicalOrder(graph);
  const depthByNode = computeDepth(graph, order);

  const rankMap = new Map<number, string[]>();
  for (const nodeId of order) {
    const depth = depthByNode[nodeId] ?? 0;
    const list = rankMap.get(depth) ?? [];
    list.push(nodeId);
    rankMap.set(depth, list);
  }

  const obstacles = widgetObstacles.map((item) =>
    inflateObstacle({ x: item.x, y: item.y, width: item.width, height: item.height }),
  );

  const positions: Record<string, WorldPoint> = {};
  const placed: Array<{ id: string; x: number; y: number; w: number; h: number }> = [];

  const rankedDepths = Array.from(rankMap.keys()).sort((a, b) => a - b);
  for (const depth of rankedDepths) {
    const ids = (rankMap.get(depth) ?? []).sort((a, b) => a.localeCompare(b));
    ids.forEach((nodeId, index) => {
      const node = graph.nodes[nodeId];
      if (!node) return;

      const tentative: WorldPoint = {
        x: config.startX + depth * config.columnGap,
        y: config.startY + index * config.rowGap,
      };

      const obstacleSafe = avoidObstacles(
        tentative.x,
        tentative.y,
        node.size.width,
        node.size.height,
        obstacles,
        config,
      );

      const collisionSafe = avoidNodeCollisions(
        obstacleSafe,
        node.size.width,
        node.size.height,
        placed,
        config,
      );

      positions[nodeId] = collisionSafe;
      placed.push({
        id: nodeId,
        x: collisionSafe.x,
        y: collisionSafe.y,
        w: node.size.width,
        h: node.size.height,
      });
    });
  }

  return {
    positions,
    depthByNode,
    order,
  };
}

export function applyAutoLayoutToGraph(
  graph: PipelineGraph,
  widgetObstacles: CanvasItem[] = [],
  userConfig?: Partial<AutoLayoutConfig>,
): PipelineGraph {
  const result = autoLayoutPipelineGraph(graph, widgetObstacles, userConfig);
  const next: PipelineGraph = JSON.parse(JSON.stringify(graph)) as PipelineGraph;

  for (const [nodeId, pos] of Object.entries(result.positions)) {
    const node = next.nodes[nodeId];
    if (node) {
      node.position = { ...pos };
    }
  }

  next.updatedAt = new Date().toISOString();
  return next;
}
