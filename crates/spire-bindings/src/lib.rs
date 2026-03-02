//! # SPIRE Bindings
//!
//! The **spire-bindings** crate exposes the `spire-kernel` functionality to
//! external consumers through stable Foreign Function Interfaces (FFI):
//!
//! - [`wasm_api`] — WebAssembly exports via `wasm-bindgen` for JavaScript/TypeScript
//!   consumers (browser and Tauri frontend).
//! - `python_api` — Python extension module via PyO3 for integration with
//!   scientific Python stacks (NumPy, Jupyter, etc.).
//!
//! Both modules wrap the same kernel calls, ensuring a consistent API contract
//! regardless of the host language.

pub mod wasm_api;

#[cfg(feature = "python")]
pub mod python_api;
