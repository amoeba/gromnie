use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;

use crate::client::ServerInfo;

pub type TransportFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, std::io::Error>> + Send + 'a>>;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub enum TransportChannel {
    Login,
    World,
}

pub trait ClientTransport: Send + Sync {
    fn send<'a>(
        &'a mut self,
        server: &'a ServerInfo,
        channel: TransportChannel,
        bytes: Vec<u8>,
    ) -> TransportFuture<'a, ()>;

    fn recv<'a>(&'a mut self, buf: &'a mut [u8]) -> TransportFuture<'a, (usize, SocketAddr)>;
}

#[cfg(not(target_arch = "wasm32"))]
pub struct NativeUdpTransport {
    socket: tokio::net::UdpSocket,
}

#[cfg(not(target_arch = "wasm32"))]
impl NativeUdpTransport {
    pub async fn bind_ephemeral() -> Result<Self, std::io::Error> {
        let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
        Ok(Self { socket })
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl ClientTransport for NativeUdpTransport {
    fn send<'a>(
        &'a mut self,
        server: &'a ServerInfo,
        channel: TransportChannel,
        bytes: Vec<u8>,
    ) -> TransportFuture<'a, ()> {
        Box::pin(async move {
            let dest_addr = match channel {
                TransportChannel::Login => server.login_addr().await?,
                TransportChannel::World => server.world_addr().await?,
            };
            self.socket.send_to(&bytes, dest_addr).await?;
            Ok(())
        })
    }

    fn recv<'a>(&'a mut self, buf: &'a mut [u8]) -> TransportFuture<'a, (usize, SocketAddr)> {
        Box::pin(async move { self.socket.recv_from(buf).await })
    }
}
