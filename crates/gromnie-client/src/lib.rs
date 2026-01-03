pub mod client;
pub mod config;
pub mod crypto;

// Re-export for backward compatibility during migration
pub use client::PatchingProgress;
