use std::net::SocketAddr;

use tracing::warn;

/// Server information tracking both login and world ports
#[derive(Clone, Debug)]
pub struct ServerInfo {
    pub host: String,
    pub login_port: u16, // Port 9000 - for LoginRequest and most traffic
    pub world_port: u16, // Port 9001 - for ConnectResponse and game data
}

impl ServerInfo {
    pub fn new(host: String, login_port: u16) -> Self {
        let world_port = login_port.saturating_add(1);
        if login_port == u16::MAX {
            warn!(target: "net", "login_port is {}, world_port will be the same value due to saturation (expected world_port = login_port + 1)", login_port);
        }
        ServerInfo {
            host,
            login_port,
            world_port,
        }
    }

    /// Check if the given peer address matches this server (either port)
    pub fn is_from(&self, peer: &SocketAddr) -> bool {
        let peer_ip = peer.ip().to_string();
        peer_ip == self.host || peer_ip == "127.0.0.1" || peer_ip == "::1"
    }

    /// Get the login server address for sending standard messages
    /// Prefers IPv4 but falls back to IPv6 if IPv4 is not available
    pub async fn login_addr(&self) -> Result<SocketAddr, std::io::Error> {
        let addr = format!("{}:{}", self.host, self.login_port);
        let addrs: Vec<SocketAddr> = tokio::net::lookup_host(&addr).await?.collect();

        // Prefer IPv4, but accept IPv6 if no IPv4 address is available
        addrs
            .iter()
            .find(|a| a.is_ipv4())
            .or_else(|| addrs.first())
            .copied()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Could not resolve address: {}", addr),
                )
            })
    }

    /// Get the world server address for sending ConnectResponse
    /// Prefers IPv4 but falls back to IPv6 if IPv4 is not available
    pub async fn world_addr(&self) -> Result<SocketAddr, std::io::Error> {
        let addr = format!("{}:{}", self.host, self.world_port);
        let addrs: Vec<SocketAddr> = tokio::net::lookup_host(&addr).await?.collect();

        // Prefer IPv4, but accept IPv6 if no IPv4 address is available
        addrs
            .iter()
            .find(|a| a.is_ipv4())
            .or_else(|| addrs.first())
            .copied()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Could not resolve address: {}", addr),
                )
            })
    }
}
