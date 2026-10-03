// The headless facade drives the client over a native UDP transport, so it is
// not built for wasm. `gromnie-web` has its own driver loop.
#[cfg(not(target_arch = "wasm32"))]
pub mod api;
pub mod client;
pub mod config;
pub mod crypto;
pub mod instant;
pub mod transport;

// Re-export for backward compatibility during migration
pub use client::PatchingProgress;
