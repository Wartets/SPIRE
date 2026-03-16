/**
 * SPIRE – Widget Event Isolation Action
 *
 * Svelte action that prevents wheel, pointerdown, mousedown, and
 * touchstart events from propagating out of a widget's interactive
 * sub-region into the Infinite Canvas transform layer.
 *
 * Without isolation, scrolling inside a data table, tweaking a slider,
 * or dragging a chart control would inadvertently pan or zoom the
 * workspace canvas.
 *
 * ## Usage
 *
 * ```svelte
 * <script>
 *   import { isolateEvents } from "$lib/actions/widgetEvents";
 * </script>
 *
 * <!-- Stops all four event types on the root widget div -->
 * <div class="my-widget" use:isolateEvents>…</div>
 *
 * <!-- Opt-in to individual event types only -->
 * <canvas use:isolateEvents={{ wheel: true, pointer: false, mouse: false, touch: false }}>
 * </canvas>
 * ```
 *
 * ## Design Notes
 *
 * - Wheel listeners are registered as { passive: true } so the browser
 *   never delays scroll rendering.
 * - Touch listeners are also passive, preserving native scroll momentum.
 * - Pointer and mouse listeners are active (default) so that downstream
 *   widget drag logic can call preventDefault when needed.
 * - The action is SSR-safe: all DOM work is skipped during server-side
 *   rendering because the function returns early when `window` is absent.
 */

export interface IsolateEventsOptions {
  /** Stop wheel events (controls canvas zoom).  Default: true. */
  wheel?: boolean;
  /** Stop pointerdown events (controls canvas pan drag start).  Default: true. */
  pointer?: boolean;
  /** Stop mousedown events (controls canvas pan drag start).  Default: true. */
  mouse?: boolean;
  /** Stop touchstart events (controls canvas touch pan).  Default: true. */
  touch?: boolean;
}

const DEFAULTS: Required<IsolateEventsOptions> = {
  wheel: true,
  pointer: true,
  mouse: true,
  touch: true,
};

// ---------------------------------------------------------------------------
// Stable listener references so we can remove them in destroy().
// ---------------------------------------------------------------------------
function stopWheel(e: WheelEvent): void { e.stopPropagation(); }
function stopPointer(e: PointerEvent): void { e.stopPropagation(); }
function stopMouse(e: MouseEvent): void { e.stopPropagation(); }
function stopTouch(e: TouchEvent): void { e.stopPropagation(); }

// ---------------------------------------------------------------------------
// Action
// ---------------------------------------------------------------------------

/**
 * Attach event-isolation listeners to `node`.
 *
 * @param node    – The DOM element to protect.
 * @param options – Fine-grained control over which event types are stopped.
 */
export function isolateEvents(
  node: HTMLElement,
  options: IsolateEventsOptions = {},
): SvelteActionReturnType {
  if (typeof window === "undefined") {
    return { update() {/* SSR no-op */}, destroy() {/* SSR no-op */} };
  }

  const opts: Required<IsolateEventsOptions> = { ...DEFAULTS, ...options };

  function attach(o: Required<IsolateEventsOptions>): void {
    if (o.wheel)   node.addEventListener("wheel",        stopWheel,   { passive: true });
    if (o.pointer) node.addEventListener("pointerdown",  stopPointer);
    if (o.mouse)   node.addEventListener("mousedown",    stopMouse);
    if (o.touch)   node.addEventListener("touchstart",   stopTouch,   { passive: true });
  }

  function detach(o: Required<IsolateEventsOptions>): void {
    if (o.wheel)   node.removeEventListener("wheel",        stopWheel);
    if (o.pointer) node.removeEventListener("pointerdown",  stopPointer);
    if (o.mouse)   node.removeEventListener("mousedown",    stopMouse);
    if (o.touch)   node.removeEventListener("touchstart",   stopTouch);
  }

  attach(opts);

  return {
    update(newOptions: IsolateEventsOptions = {}): void {
      detach(opts);
      Object.assign(opts, { ...DEFAULTS, ...newOptions });
      attach(opts);
    },
    destroy(): void {
      detach(opts);
    },
  };
}

// Svelte action return type helper (avoids importing from svelte internals).
type SvelteActionReturnType = {
  update?: (newOptions?: IsolateEventsOptions) => void;
  destroy?: () => void;
};
