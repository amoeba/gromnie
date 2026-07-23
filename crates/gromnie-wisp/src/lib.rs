//! Shared WISP protocol helpers for gromnie.
//!
//! This crate provides small utility functions that are used by both
//! `gromnie-proxy` and `gromnie-web` to avoid duplicating WISP handshake
//! configuration and other protocol boilerplate.

use wisp_mux::{
    WispV2Handshake,
    extensions::{AnyProtocolExtensionBuilder, udp::UdpProtocolExtensionBuilder},
};

/// Create a [`WispV2Handshake`] with the default set of protocol extensions.
///
/// Currently this enables the UDP protocol extension, which is the only
/// extension used by gromnie's AC game server tunneling. Both the proxy
/// (server side) and the web client (client side) construct the same handshake,
/// so this helper eliminates that duplication.
pub fn default_wisp_handshake() -> WispV2Handshake {
    WispV2Handshake::new(vec![AnyProtocolExtensionBuilder::new(
        UdpProtocolExtensionBuilder,
    )])
}

/// Format a byte slice as a space-separated hex string, truncated to `max` bytes.
///
/// This is used by both `gromnie-proxy` and `gromnie-web` for debug logging
/// of WISP/UDP packets, eliminating duplicated hex formatting logic.
pub fn hex_preview(bytes: &[u8], max: usize) -> String {
    bytes
        .iter()
        .take(max)
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}
