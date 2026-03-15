export interface LongPressDetail {
  x: number;
  y: number;
  pointerType: string;
  target: EventTarget | null;
}

export interface LongPressOptions {
  duration?: number;
  moveTolerance?: number;
  enabled?: boolean;
  onLongPress?: (detail: LongPressDetail) => void;
}

const DEFAULT_DURATION = 480;
const DEFAULT_MOVE_TOLERANCE = 10;

export function longpress(node: HTMLElement, options: LongPressOptions = {}) {
  let duration = options.duration ?? DEFAULT_DURATION;
  let moveTolerance = options.moveTolerance ?? DEFAULT_MOVE_TOLERANCE;
  let enabled = options.enabled ?? true;
  let onLongPress = options.onLongPress;

  let timer: ReturnType<typeof setTimeout> | null = null;
  let pointerId: number | null = null;
  let startX = 0;
  let startY = 0;
  let pointerType = "";
  let target: EventTarget | null = null;

  function clearTimer(): void {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
  }

  function cancel(): void {
    clearTimer();
    pointerId = null;
  }

  function handlePointerDown(event: PointerEvent): void {
    if (!enabled) return;
    if (event.pointerType === "mouse") return;
    if (event.button !== 0) return;

    cancel();
    pointerId = event.pointerId;
    startX = event.clientX;
    startY = event.clientY;
    pointerType = event.pointerType;
    target = event.target;

    timer = setTimeout(() => {
      timer = null;
      onLongPress?.({
        x: startX,
        y: startY,
        pointerType,
        target,
      });
      pointerId = null;
    }, duration);
  }

  function handlePointerMove(event: PointerEvent): void {
    if (pointerId === null || event.pointerId !== pointerId) return;
    const dx = event.clientX - startX;
    const dy = event.clientY - startY;
    if (Math.hypot(dx, dy) > moveTolerance) {
      cancel();
    }
  }

  function handlePointerEnd(event: PointerEvent): void {
    if (pointerId !== null && event.pointerId === pointerId) {
      cancel();
    }
  }

  node.addEventListener("pointerdown", handlePointerDown);
  node.addEventListener("pointermove", handlePointerMove);
  node.addEventListener("pointerup", handlePointerEnd);
  node.addEventListener("pointercancel", handlePointerEnd);
  node.addEventListener("lostpointercapture", handlePointerEnd);

  return {
    update(next: LongPressOptions = {}) {
      duration = next.duration ?? DEFAULT_DURATION;
      moveTolerance = next.moveTolerance ?? DEFAULT_MOVE_TOLERANCE;
      enabled = next.enabled ?? true;
      onLongPress = next.onLongPress;
      if (!enabled) cancel();
    },
    destroy() {
      cancel();
      node.removeEventListener("pointerdown", handlePointerDown);
      node.removeEventListener("pointermove", handlePointerMove);
      node.removeEventListener("pointerup", handlePointerEnd);
      node.removeEventListener("pointercancel", handlePointerEnd);
      node.removeEventListener("lostpointercapture", handlePointerEnd);
    },
  };
}
