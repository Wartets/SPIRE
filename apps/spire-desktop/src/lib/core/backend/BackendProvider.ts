/**
 * SPIRE — Backend Provider (Environment Detection & Routing)
 *
 * Detects the current execution environment and instantiates the
 * appropriate `SpireBackend` implementation.  The resolved backend
 * is exported as a Svelte readable store so that any component can
 * subscribe to it reactively.
 *
 * ## Detection Priority
 *
 * 1. **Tauri IPC** — Check for `window.__TAURI_IPC__` or
 *    `window.__TAURI_INTERNALS__`.  If either exists, the app is running
 *    inside the native Tauri webview → use `TauriBackend`.
 *
 * 2. **WebAssembly** — Check `typeof WebAssembly !== "undefined"`.  If
 *    available and the WASM worker can be instantiated → use `WasmBackend`.
 *
 * 3. **Mock (Simulation Mode)** — Fallback → use `MockBackend`.
 *
 * The detection runs exactly once during module initialisation and the
 * result is frozen for the lifetime of the page.
 */

import { writable, derived, type Readable } from "svelte/store";
import type { SpireBackend, BackendKind } from "./BackendInterface";
import { TauriBackend } from "./TauriBackend";
import { WasmBackend } from "./WasmBackend";
import { MockBackend } from "./MockBackend";

// ---------------------------------------------------------------------------
// Environment Detection
// ---------------------------------------------------------------------------

/**
 * Probe the runtime environment and return the detected backend kind.
 *
 * This function is intentionally synchronous and safe — it must never
 * throw an exception, since it runs during module-level initialisation.
 */
function detectEnvironment(): BackendKind {
  try {
    // Tauri v1 exposes `window.__TAURI_IPC__`.
    // Tauri v2 exposes `window.__TAURI_INTERNALS__`.
    // Both indicate we are inside the Tauri webview.
    if (
      typeof window !== "undefined" &&
      (typeof (window as unknown as Record<string, unknown>).__TAURI_IPC__ === "function" ||
       typeof (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ !== "undefined")
    ) {
      return "tauri";
    }
  } catch {
    // `window` may not exist in SSR or test contexts — fall through.
  }

  try {
    if (typeof WebAssembly !== "undefined") {
      return "wasm";
    }
  } catch {
    // WebAssembly check failed — fall through to mock.
  }

  return "mock";
}

/**
 * Instantiate the backend for the given environment kind.
 */
function createBackend(kind: BackendKind): SpireBackend {
  switch (kind) {
    case "tauri":
      return new TauriBackend();
    case "wasm":
      return new WasmBackend();
    case "mock":
      return new MockBackend();
  }
}

// ---------------------------------------------------------------------------
// Singleton & Store
// ---------------------------------------------------------------------------

/** The detected environment (frozen at module init). */
const detectedKind = detectEnvironment();

/** The singleton backend instance. */
const backendInstance = createBackend(detectedKind);

/** Writable store holding the active backend (allows hot-swap for testing). */
const _backendStore = writable<SpireBackend>(backendInstance);

/**
 * Reactive Svelte store providing the current `SpireBackend`.
 *
 * Usage in a Svelte component:
 * ```svelte
 * <script>
 *   import { backend } from "$lib/core/backend/BackendProvider";
 *   const b = $backend;
 *   const model = await b.loadModel(particles, vertices);
 * </script>
 * ```
 */
export const backend: Readable<SpireBackend> = { subscribe: _backendStore.subscribe };

/**
 * Reactive store providing the current backend kind as a string.
 *
 * Useful for the status-bar indicator:
 * ```svelte
 * <span>{$backendKind}</span>
 * ```
 */
export const backendKind: Readable<BackendKind> = derived(
  _backendStore,
  ($b) => $b.kind,
);

/**
 * Human-readable label for the current backend kind.
 */
export const backendLabel: Readable<string> = derived(
  _backendStore,
  ($b) => {
    switch ($b.kind) {
      case "tauri": return "Native (Tauri)";
      case "wasm":  return "WebAssembly";
      case "mock":  return "Simulation";
    }
  },
);

// ---------------------------------------------------------------------------
// Imperative Access (for non-Svelte contexts like workers & services)
// ---------------------------------------------------------------------------

/**
 * Get the current backend instance imperatively.
 *
 * Prefer the `backend` store in Svelte components; use this in plain
 * TypeScript services, workers, or one-off scripts.
 */
export function getBackend(): SpireBackend {
  return backendInstance;
}

/**
 * Swap the active backend at runtime (for testing or manual override).
 *
 * This updates the Svelte store so all subscribed components react.
 */
export function setBackend(newBackend: SpireBackend): void {
  _backendStore.set(newBackend);
}
