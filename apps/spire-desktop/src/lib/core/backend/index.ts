/**
 * SPIRE — Backend Module Barrel Export
 *
 * Centralises all backend-related exports so that consumers can import
 * from a single path:
 *
 * ```ts
 * import { backend, backendKind, BackendError } from "$lib/core/backend";
 * ```
 */

// Interface & error types
export type { SpireBackend, BackendKind, BackendErrorKind } from "./BackendInterface";
export { BackendError } from "./BackendInterface";

// Concrete implementations (rarely imported directly)
export { TauriBackend } from "./TauriBackend";
export { WasmBackend } from "./WasmBackend";
export { MockBackend } from "./MockBackend";

// Provider (the primary consumer API)
export {
  backend,
  backendKind,
  backendLabel,
  getBackend,
  setBackend,
} from "./BackendProvider";
