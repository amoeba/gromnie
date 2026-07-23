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
