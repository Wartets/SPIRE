/**
 * SPIRE — Window Manager Service
 *
 * Manages Tauri multi-window lifecycle for widget tear-offs.
 * When a user tears off a widget, a new native OS window is spawned
 * via the Tauri WebviewWindow API.  State synchronisation between
 * windows is handled through Tauri's event system.
 *
 * ## Architecture
 *
 * - The main window acts as the state authority.
 * - On physics store changes, the main window broadcasts a
 *   `spire://store-sync` event to all windows.
 * - Tear-off windows listen for `spire://store-sync` and hydrate
 *   their local stores.
 * - Tear-off windows emit `spire://store-action` when the user
 *   performs mutations (e.g., loading a model), which the main
 *   window handles.
 *
 * ## Fallback
 *
 * When running outside Tauri (pure web / WASM mode), the tear-off
 * button opens a new browser tab via `window.open()` instead.
 */

import type { WidgetType } from "$lib/stores/notebookStore";

// ---------------------------------------------------------------------------
// Tauri Detection
// ---------------------------------------------------------------------------

/** Whether we're running inside a Tauri native shell. */
function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI__" in window;
}

// ---------------------------------------------------------------------------
// Tear-Off Registry
// ---------------------------------------------------------------------------

/** Map of active tear-off window labels → widget metadata. */
const activeTearOffs = new Map<
  string,
  { widgetType: WidgetType; nodeId: string }
>();

let _tearOffCounter = 0;

function nextTearOffLabel(): string {
  _tearOffCounter += 1;
  return `spire-tearoff-${_tearOffCounter}`;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Tear off a widget into a new native OS window (Tauri) or browser tab (web).
 *
 * @param widgetType - The widget type to render in the new window.
 * @param nodeId - The layout node ID being torn off.
 * @param title - The window title.
 */
export async function tearOffWidget(
  widgetType: WidgetType,
  nodeId: string,
  title: string,
): Promise<void> {
  const windowLabel = nextTearOffLabel();
  const url = `/window?widgetType=${encodeURIComponent(widgetType)}&nodeId=${encodeURIComponent(nodeId)}`;

  if (isTauri()) {
    try {
      // Dynamic import to avoid bundling issues in non-Tauri environments
      const { WebviewWindow } = await import("@tauri-apps/api/window");

      const webview = new WebviewWindow(windowLabel, {
        url,
        title: `SPIRE — ${title}`,
        width: 700,
        height: 500,
        resizable: true,
        decorations: true,
        center: true,
      });

      // Track the tear-off
      activeTearOffs.set(windowLabel, { widgetType, nodeId });

      // Clean up on close
      webview.once("tauri://close-requested", () => {
        activeTearOffs.delete(windowLabel);
      });
    } catch (err) {
      console.error("Failed to create tear-off window:", err);
      // Fallback to browser tab
      window.open(url, windowLabel, "width=700,height=500");
    }
  } else {
    // Pure web fallback: open in a new tab/popup
    window.open(url, windowLabel, "width=700,height=500");
  }
}

/**
 * Broadcast a state-sync event to all tear-off windows.
 * Called by the main window when stores update.
 *
 * @param payload - Serialised store snapshot.
 */
export async function broadcastStoreSync(payload: unknown): Promise<void> {
  if (!isTauri() || activeTearOffs.size === 0) return;

  try {
    const { emit } = await import("@tauri-apps/api/event");
    await emit("spire://store-sync", payload);
  } catch (err) {
    console.error("Failed to broadcast store sync:", err);
  }
}

/**
 * Listen for state-sync events from the main window.
 * Used by tear-off windows to hydrate their stores.
 *
 * @param handler - Callback receiving the serialised payload.
 * @returns Unlisten function.
 */
export async function listenForStoreSync(
  handler: (payload: unknown) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};

  try {
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<unknown>("spire://store-sync", (event) => {
      handler(event.payload);
    });
    return unlisten;
  } catch {
    return () => {};
  }
}

/**
 * Emit a store-action event from a tear-off window to the main window.
 *
 * @param action - The action identifier.
 * @param payload - Action-specific data.
 */
export async function emitStoreAction(
  action: string,
  payload: unknown,
): Promise<void> {
  if (!isTauri()) return;

  try {
    const { emit } = await import("@tauri-apps/api/event");
    await emit("spire://store-action", { action, payload });
  } catch (err) {
    console.error("Failed to emit store action:", err);
  }
}

/**
 * Listen for store-action events from tear-off windows.
 * Used by the main window to handle remote mutations.
 *
 * @param handler - Callback receiving { action, payload }.
 * @returns Unlisten function.
 */
export async function listenForStoreActions(
  handler: (data: { action: string; payload: unknown }) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};

  try {
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<{ action: string; payload: unknown }>(
      "spire://store-action",
      (event) => {
        handler(event.payload);
      },
    );
    return unlisten;
  } catch {
    return () => {};
  }
}

/** Get the count of active tear-off windows. */
export function getTearOffCount(): number {
  return activeTearOffs.size;
}

/** Get all active tear-off window labels. */
export function getActiveTearOffs(): Map<
  string,
  { widgetType: WidgetType; nodeId: string }
> {
  return new Map(activeTearOffs);
}
