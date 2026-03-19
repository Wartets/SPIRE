export interface BoundingBox {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export interface RectLike {
  x: number;
  y: number;
  width: number;
  height: number;
}

export const DEFAULT_OVERSCAN_RATIO = 1.5;

export function getVisibleBounds(
  panX: number,
  panY: number,
  zoom: number,
  screenWidth: number,
  screenHeight: number,
): BoundingBox {
  const safeZoom = Math.max(zoom, 1e-6);
  const left = -panX / safeZoom;
  const top = -panY / safeZoom;
  return {
    left,
    top,
    right: left + screenWidth / safeZoom,
    bottom: top + screenHeight / safeZoom,
  };
}

export function expandBounds(bounds: BoundingBox, overscanRatio: number = DEFAULT_OVERSCAN_RATIO): BoundingBox {
  const ratio = Math.max(0, overscanRatio);
  const width = bounds.right - bounds.left;
  const height = bounds.bottom - bounds.top;
  const dx = width * ratio;
  const dy = height * ratio;
  return {
    left: bounds.left - dx,
    top: bounds.top - dy,
    right: bounds.right + dx,
    bottom: bounds.bottom + dy,
  };
}

export function intersectsAabb(a: BoundingBox, b: BoundingBox): boolean {
  return !(
    a.right < b.left ||
    a.left > b.right ||
    a.bottom < b.top ||
    a.top > b.bottom
  );
}

export function rectToBounds(rect: RectLike): BoundingBox {
  return {
    left: rect.x,
    top: rect.y,
    right: rect.x + rect.width,
    bottom: rect.y + rect.height,
  };
}

export function isWidgetVisible(rect: RectLike, visibleBounds: BoundingBox): boolean {
  return intersectsAabb(rectToBounds(rect), visibleBounds);
}
