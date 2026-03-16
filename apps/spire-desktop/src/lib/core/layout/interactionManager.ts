export type ResizeDirection = "n" | "ne" | "e" | "se" | "s" | "sw" | "w" | "nw";

export interface DragOrigin {
  x: number;
  y: number;
}

export interface ResizeOrigin {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface DragPatch {
  x: number;
  y: number;
}

export interface ResizePatch {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface DragSession {
  mode: "drag";
  itemId: string;
  pointerId: number;
  startClientX: number;
  startClientY: number;
  origin: DragOrigin;
  startedAt: number;
}

interface ResizeSession {
  mode: "resize";
  itemId: string;
  pointerId: number;
  direction: ResizeDirection;
  startClientX: number;
  startClientY: number;
  origin: ResizeOrigin;
  minWidth: number;
  minHeight: number;
  startedAt: number;
}

type InteractionSession = DragSession | ResizeSession;

export type InteractionPatch =
  | { mode: "drag"; itemId: string; patch: DragPatch }
  | { mode: "resize"; itemId: string; direction: ResizeDirection; patch: ResizePatch };

const STALE_SESSION_TIMEOUT_MS = 12_000;

/**
 * Centralized pointer interaction manager for canvas widgets.
 *
 * Tracks at most one active interaction session globally so all drag/resize
 * math and pointer ownership is coordinated in one place.
 */
export class CanvasInteractionManager {
  private session: InteractionSession | null = null;

  hasActiveSession(): boolean {
    return this.session !== null;
  }

  private now(): number {
    return Date.now();
  }

  private isSessionStale(): boolean {
    if (!this.session) return false;
    return this.now() - this.session.startedAt > STALE_SESSION_TIMEOUT_MS;
  }

  private canStartWithPointer(pointerId: number): boolean {
    if (!this.session) return true;

    // Same pointer may restart after capture races.
    if (this.session.pointerId === pointerId) {
      this.session = null;
      return true;
    }

    // Safety valve for interrupted lifecycle where pointerup/cancel was lost.
    // We prefer fail-open recovery over a hard lock that disables all input.
    if (this.isSessionStale()) {
      this.session = null;
      return true;
    }

    // Different pointer started while a previous session appears active.
    // In practice this usually means a missed termination event in host/webview.
    // Reset aggressively so drag/resize never get stuck globally.
    this.session = null;
    return true;
  }

  startDrag(params: {
    itemId: string;
    pointerId: number;
    clientX: number;
    clientY: number;
    origin: DragOrigin;
  }): boolean {
    if (!this.canStartWithPointer(params.pointerId)) return false;

    this.session = {
      mode: "drag",
      itemId: params.itemId,
      pointerId: params.pointerId,
      startClientX: params.clientX,
      startClientY: params.clientY,
      origin: params.origin,
      startedAt: this.now(),
    };
    return true;
  }

  startResize(params: {
    itemId: string;
    pointerId: number;
    clientX: number;
    clientY: number;
    origin: ResizeOrigin;
    direction: ResizeDirection;
    minWidth: number;
    minHeight: number;
  }): boolean {
    if (!this.canStartWithPointer(params.pointerId)) return false;

    this.session = {
      mode: "resize",
      itemId: params.itemId,
      pointerId: params.pointerId,
      startClientX: params.clientX,
      startClientY: params.clientY,
      origin: params.origin,
      direction: params.direction,
      minWidth: params.minWidth,
      minHeight: params.minHeight,
      startedAt: this.now(),
    };
    return true;
  }

  move(pointerId: number, clientX: number, clientY: number, zoom: number): InteractionPatch | null {
    if (!this.session || this.session.pointerId !== pointerId) return null;

    const safeZoom = Math.max(zoom, 1e-6);
    const dx = (clientX - this.session.startClientX) / safeZoom;
    const dy = (clientY - this.session.startClientY) / safeZoom;

    if (this.session.mode === "drag") {
      return {
        mode: "drag",
        itemId: this.session.itemId,
        patch: {
          x: this.session.origin.x + dx,
          y: this.session.origin.y + dy,
        },
      };
    }

    let nextX = this.session.origin.x;
    let nextY = this.session.origin.y;
    let nextW = this.session.origin.width;
    let nextH = this.session.origin.height;

    if (this.session.direction.includes("e")) {
      nextW = Math.max(this.session.minWidth, this.session.origin.width + dx);
    }

    if (this.session.direction.includes("s")) {
      nextH = Math.max(this.session.minHeight, this.session.origin.height + dy);
    }

    if (this.session.direction.includes("w")) {
      const proposedW = this.session.origin.width - dx;
      nextW = Math.max(this.session.minWidth, proposedW);
      nextX = this.session.origin.x + (this.session.origin.width - nextW);
    }

    if (this.session.direction.includes("n")) {
      const proposedH = this.session.origin.height - dy;
      nextH = Math.max(this.session.minHeight, proposedH);
      nextY = this.session.origin.y + (this.session.origin.height - nextH);
    }

    return {
      mode: "resize",
      itemId: this.session.itemId,
      direction: this.session.direction,
      patch: {
        x: nextX,
        y: nextY,
        width: nextW,
        height: nextH,
      },
    };
  }

  end(pointerId: number): InteractionSession | null {
    if (!this.session || this.session.pointerId !== pointerId) return null;
    const ended = this.session;
    this.session = null;
    return ended;
  }

  cancel(pointerId: number): void {
    if (this.session?.pointerId === pointerId) {
      this.session = null;
    }
  }

  /** Force reset when DOM-side capture lifecycle is interrupted. */
  forceReset(): void {
    this.session = null;
  }

  activePointerId(): number | null {
    return this.session?.pointerId ?? null;
  }
}

export type InteractionManager = CanvasInteractionManager;
export const interactionManager = new CanvasInteractionManager();
