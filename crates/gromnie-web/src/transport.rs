use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

use futures::lock::Mutex;
use futures_util::{FutureExt, SinkExt, StreamExt, pin_mut, select};
use send_wrapper::SendWrapper;
use wasm_bindgen::prelude::*;

use gromnie_client::client::ServerInfo;
use gromnie_client::transport::{ClientTransport, TransportChannel, TransportFuture};
use wisp_mux::packet::StreamType;
use wisp_mux::stream::MuxStream;
use wisp_mux::ws::Payload;

use crate::{BrowserWispTransportWrite, ClientState};

pub(crate) type NetLogCallback = Rc<RefCell<Option<js_sys::Function>>>;

pub(crate) fn format_net_entry(dir: &str, channel: &str, bytes: &[u8]) -> String {
    if bytes.len() >= 8 {
        let seq = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let flags = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let hex: String = bytes
            .iter()
            .take(32)
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");
        let ellipsis = if bytes.len() > 32 { " ..." } else { "" };
        format!(
            "[{}] {} seq={} flags=0x{:08X} len={} | {}{}",
            dir,
            channel,
            seq,
            flags,
            bytes.len(),
            hex,
            ellipsis
        )
    } else {
        let hex: String = bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");
        format!("[{}] {} len={} | {}", dir, channel, bytes.len(), hex)
    }
}

pub struct WispUdpTransport {
    state: SendWrapper<Arc<Mutex<ClientState>>>,
    streams: SendWrapper<HashMap<TransportChannel, MuxStream<BrowserWispTransportWrite>>>,
    net_log: SendWrapper<NetLogCallback>,
}

impl WispUdpTransport {
    pub(crate) fn new(
        state: Arc<Mutex<ClientState>>,
        stream: MuxStream<BrowserWispTransportWrite>,
        net_log: NetLogCallback,
    ) -> Self {
        let mut streams = HashMap::new();
        streams.insert(TransportChannel::Login, stream);
        Self {
            state: SendWrapper::new(state),
            streams: SendWrapper::new(streams),
            net_log: SendWrapper::new(net_log),
        }
    }

    fn log_send(&self, channel: TransportChannel, bytes: &[u8]) {
        if let Some(cb) = self.net_log.borrow().as_ref() {
            let ch = match channel {
                TransportChannel::Login => "Login",
                TransportChannel::World => "World",
            };
            let msg = format_net_entry("TX", ch, bytes);
            cb.call1(&JsValue::NULL, &msg.into()).ok();
        }
    }

    async fn ensure_stream(
        &mut self,
        server: &ServerInfo,
        channel: TransportChannel,
    ) -> Result<(), std::io::Error> {
        if self.streams.contains_key(&channel) {
            return Ok(());
        }

        let port = match channel {
            TransportChannel::Login => server.login_port,
            TransportChannel::World => server.world_port,
        };

        let state = self.state.lock().await;
        let mux = state.mux.as_ref().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotConnected, "WISP not connected")
        })?;

        let stream = mux
            .new_stream(StreamType::Udp, server.host.clone(), port)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        self.streams.insert(channel, stream);
        Ok(())
    }
}

impl ClientTransport for WispUdpTransport {
    fn send<'a>(
        &'a mut self,
        server: &'a ServerInfo,
        channel: TransportChannel,
        bytes: Vec<u8>,
    ) -> TransportFuture<'a, ()> {
        Box::pin(async move {
            self.log_send(channel, &bytes);
            self.ensure_stream(server, channel).await?;
            let stream = self.streams.get_mut(&channel).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "Stream not found")
            })?;
            stream
                .send(Payload::from(bytes))
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        })
    }

    fn recv<'a>(&'a mut self, buf: &'a mut [u8]) -> TransportFuture<'a, (usize, SocketAddr)> {
        Box::pin(async move {
            if self.streams.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotConnected,
                    "No streams available",
                ));
            }

            let login = self.streams.remove(&TransportChannel::Login);
            let world = self.streams.remove(&TransportChannel::World);

            let mut stream_a = login;
            let mut stream_b = world;

            let result = loop {
                let fut_a = async {
                    match &mut stream_a {
                        Some(s) => StreamExt::next(s).await,
                        None => std::future::pending().await,
                    }
                };
                let fut_b = async {
                    match &mut stream_b {
                        Some(s) => StreamExt::next(s).await,
                        None => std::future::pending().await,
                    }
                };
                let fut_a = fut_a.fuse();
                let fut_b = fut_b.fuse();
                pin_mut!(fut_a, fut_b);

                let r: Option<Result<_, _>> = select! {
                    r = fut_a => r,
                    r = fut_b => r,
                };
                if r.is_some() {
                    break r;
                }
            };

            if let Some(s) = stream_a {
                self.streams.insert(TransportChannel::Login, s);
            }
            if let Some(s) = stream_b {
                self.streams.insert(TransportChannel::World, s);
            }

            match result {
                Some(Ok(payload)) => {
                    let data: Vec<u8> = payload.into();
                    let len = std::cmp::min(data.len(), buf.len());
                    buf[..len].copy_from_slice(&data[..len]);
                    let addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
                    Ok((len, addr))
                }
                Some(Err(e)) => Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("{}", e),
                )),
                None => Err(std::io::Error::new(
                    std::io::ErrorKind::ConnectionReset,
                    "All streams closed",
                )),
            }
        })
    }
}
