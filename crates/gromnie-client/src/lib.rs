pub mod client;
pub mod config;
pub mod crypto;
pub mod instant;
pub mod transport;

// Re-export for backward compatibility during migration
pub use client::PatchingProgress;
