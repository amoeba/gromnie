use std::net::SocketAddr;

/// Server information tracking both login and world ports
#[derive(Clone, Debug)]
pub struct ServerInfo {
    pub host: String,
    pub login_port: u16, // Port 9000 - for LoginRequest and most traffic
    pub world_port: u16, // Port 9001 - for ConnectResponse and game data
}

impl ServerInfo {
    pub fn new(host: String, login_port: u16) -> Self {
        ServerInfo {
            host,
            login_port,
            world_port: login_port + 1,
        }
    }

    /// Check if the given peer address matches this server (either port)
    pub fn is_from(&self, peer: &SocketAddr) -> bool {
        let peer_ip = peer.ip().to_string();
        peer_ip == self.host || peer_ip == "127.0.0.1" || peer_ip == "::1"
    }

    /// Get the login server address for sending standard messages
    pub async fn login_addr(&self) -> Result<SocketAddr, std::io::Error> {
        let addr = format!("{}:{}", self.host, self.login_port);
        tokio::net::lookup_host(addr)
            .await?
            .find(|a| a.is_ipv4())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not resolve IPv4 address",
                )
            })
    }

    /// Get the world server address for sending ConnectResponse
    pub async fn world_addr(&self) -> Result<SocketAddr, std::io::Error> {
        let addr = format!("{}:{}", self.host, self.world_port);
        tokio::net::lookup_host(addr)
            .await?
            .find(|a| a.is_ipv4())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not resolve IPv4 address",
                )
            })
    }
}
