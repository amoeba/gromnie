use std::cell::RefCell;
use std::collections::HashMap;
use std::net::SocketAddr;
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
        let hex = gromnie_wisp::hex_preview(bytes, 32);
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
        let hex = gromnie_wisp::hex_preview(bytes, usize::MAX);
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

            let mut login = self.streams.remove(&TransportChannel::Login);
            let mut world = self.streams.remove(&TransportChannel::World);

            let result = match (&mut login, &mut world) {
                (Some(l), Some(w)) => {
                    // Both streams available: race them with select!
                    let l_fut = StreamExt::next(l).fuse();
                    let w_fut = StreamExt::next(w).fuse();
                    pin_mut!(l_fut, w_fut);

                    let r = select! {
                        r = l_fut => {
                            if r.is_none() { login = None; }
                            r
                        }
                        r = w_fut => {
                            if r.is_none() { world = None; }
                            r
                        }
                    };

                    if r.is_some() {
                        r
                    } else {
                        // One stream closed; fall back to the other.
                        match (&mut login, &mut world) {
                            (Some(l), None) => StreamExt::next(l).await,
                            (None, Some(w)) => StreamExt::next(w).await,
                            _ => None,
                        }
                    }
                }
                (Some(l), None) => StreamExt::next(l).await,
                (None, Some(w)) => StreamExt::next(w).await,
                (None, None) => None,
            };

            if let Some(s) = login {
                self.streams.insert(TransportChannel::Login, s);
            }
            if let Some(s) = world {
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
