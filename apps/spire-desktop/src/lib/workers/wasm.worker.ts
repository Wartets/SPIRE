/**
 * SPIRE - WASM Compute Worker
 *
 * Dedicated Web Worker that hosts the spire-kernel WebAssembly module.
 * Receives JSON-serialised command requests from the `WasmBackend` on
 * the main thread, invokes the corresponding WASM entry point, and
 * posts back the result.
 *
 * ## Module Loading
 *
 * The WASM binary (`spire_kernel_bg.wasm`) and its JS glue are loaded
 * lazily on the first incoming request.  If the WASM artefact is not
 * available (e.g., not yet compiled), all requests fail gracefully with
 * an error response rather than crashing the worker.
 *
 * ## Message Protocol
 *
 * Inbound:  { id: number, command: string, args: Record<string, unknown> }
 * Outbound: { id: number, ok: boolean, data?: unknown, error?: string }
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface WasmRequest {
  id: number;
  command: string;
  args: Record<string, unknown>;
}

interface WasmResponse {
  id: number;
  ok: boolean;
  data?: unknown;
  error?: string;
}

// ---------------------------------------------------------------------------
// WASM Module Handle
// ---------------------------------------------------------------------------

let wasmModule: Record<string, (...args: unknown[]) => unknown> | null = null;
let wasmLoadAttempted = false;
let wasmLoadError: string | null = null;

/**
 * Attempt to load the spire-kernel WASM module.
 *
 * This is a best-effort loader.  If the wasm-pack artefact is not
 * present on the server, it catches the error and marks the module
 * as unavailable so subsequent calls fail fast.
 */
async function ensureWasmLoaded(): Promise<void> {
  if (wasmModule) return;
  if (wasmLoadAttempted) return;
  wasmLoadAttempted = true;

  try {
    // @ts-expect-error - spire-kernel-wasm is an optional dependency built separately
    const mod = await import(/* @vite-ignore */ "spire-kernel-wasm");
    if (typeof mod.default === "function") {
      await mod.default(); // init() for wasm-bindgen
    }
    wasmModule = mod as unknown as Record<string, (...args: unknown[]) => unknown>;
  } catch (err) {
    wasmLoadError =
      err instanceof Error
        ? err.message
        : "Failed to load spire-kernel WASM module.";
  }
}

// ---------------------------------------------------------------------------
// Command Dispatch
// ---------------------------------------------------------------------------

/**
 * Map an IPC command name to the corresponding WASM function.
 *
 * The WASM entry points are expected to follow the naming convention
 * produced by `wasm-bindgen`: snake_case Rust → camelCase JS.
 * Each function takes a single JSON-string argument and returns a
 * JSON string.
 */
async function dispatch(command: string, args: Record<string, unknown>): Promise<unknown> {
  await ensureWasmLoaded();

  if (!wasmModule) {
    throw new Error(
      wasmLoadError ?? "WASM module is not available.",
    );
  }

  // Convert command name (snake_case) to camelCase for wasm-bindgen.
  const fnName = command.replace(/_([a-z])/g, (_, c: string) => c.toUpperCase());

  // Ensure we only dispatch to own exports of the WASM module and not to
  // inherited properties on the prototype chain.
  const hasOwnExport =
    Object.prototype.hasOwnProperty.call(wasmModule, fnName);

  if (!hasOwnExport) {
    throw new Error(
      `WASM module does not export function "${fnName}" (command: "${command}").`,
    );
  }

  const fn = wasmModule[fnName];
  if (typeof fn !== "function") {
    throw new Error(
      `WASM export "${fnName}" is not callable (command: "${command}").`,
    );
  }

  // Pass serialised JSON - the WASM side will deserialise with serde.
  const result = fn(JSON.stringify(args));

  // If the WASM function returns a Promise (async), await it.
  const resolved = result instanceof Promise ? await result : result;

  // If the result is a string, parse it as JSON.
  if (typeof resolved === "string") {
    try {
      return JSON.parse(resolved);
    } catch {
      return resolved;
    }
  }

  return resolved;
}

// ---------------------------------------------------------------------------
// Message Handler
// ---------------------------------------------------------------------------

self.onmessage = async (event: MessageEvent<WasmRequest>) => {
  const { id, command, args } = event.data;

  try {
    const data = await dispatch(command, args);
    const response: WasmResponse = { id, ok: true, data };
    self.postMessage(response);
  } catch (err) {
    const response: WasmResponse = {
      id,
      ok: false,
      error: err instanceof Error ? err.message : String(err),
    };
    self.postMessage(response);
  }
};
