use std::{
    collections::HashMap,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use event_listener::Event;
use flume::Receiver;
use futures::lock::Mutex;
use futures_util::FutureExt;
use js_sys::{Array, ArrayBuffer, Uint8Array};
use send_wrapper::SendWrapper;
use thiserror::Error;
use wasm_bindgen::{JsCast, JsValue, closure::Closure, prelude::wasm_bindgen};
use wasm_bindgen_futures::spawn_local;
use web_sys::{BinaryType, MessageEvent, WebSocket};
use wisp_mux::{
    ClientMux, WispError, WispV2Handshake,
    extensions::{AnyProtocolExtensionBuilder, udp::UdpProtocolExtensionBuilder},
    packet::StreamType,
    stream::MuxStream,
    ws::{
        Payload, TransportRead, TransportWrite, async_iterator_transport_read,
        async_iterator_transport_write,
    },
};

pub(crate) mod util;

pub mod transport;

type BrowserWispTransportRead = Pin<Box<dyn TransportRead>>;
pub(crate) type BrowserWispTransportWrite = Pin<Box<dyn TransportWrite>>;

#[derive(Default)]
pub(crate) struct ClientState {
    pub(crate) mux: Option<ClientMux<BrowserWispTransportWrite>>,
    pub(crate) streams: HashMap<u32, MuxStream<BrowserWispTransportWrite>>,
    pub(crate) last_mux_error: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum WispVersionPolicy {
    #[default]
    RequireV2,
    AllowV1Downgrade,
}

impl WispVersionPolicy {
    fn rejects_downgrade(self, downgraded: bool) -> bool {
        matches!(self, Self::RequireV2) && downgraded
    }
}

#[derive(Error, Debug)]
enum WebSocketTransportError {
    #[error("websocket error: {0}")]
    Unknown(String),
    #[error("websocket send failed: {0}")]
    SendFailed(String),
    #[error("websocket close failed: {0}")]
    CloseFailed(String),
}

impl From<WebSocketTransportError> for WispError {
    fn from(err: WebSocketTransportError) -> Self {
        Self::WsImplError(Box::new(err))
    }
}

enum WebSocketMessage {
    Error(WebSocketTransportError),
    Message(Vec<u8>),
}

struct WebSocketReader {
    read_rx: Receiver<WebSocketMessage>,
    closed: Arc<AtomicBool>,
    close_event: Arc<Event>,
}

impl WebSocketReader {
    fn into_read(self) -> BrowserWispTransportRead {
        Box::pin(async_iterator_transport_read(self, |this| {
            Box::pin(async move {
                use WebSocketMessage as M;
                if this.closed.load(Ordering::Acquire) {
                    return Err(WispError::WsImplSocketClosed);
                }

                let res = futures_util::select! {
                    data = this.read_rx.recv_async() => data.ok(),
                    () = this.close_event.listen().fuse() => None
                };

                match res {
                    Some(M::Message(x)) => Ok(Some((Payload::from(x), this))),
                    Some(M::Error(x)) => Err(x.into()),
                    None => Ok(None),
                }
            })
        }))
    }
}

struct BrowserWebSocketTransport {
    inner: Arc<SendWrapper<WebSocket>>,
    open_event: Arc<Event>,
    error_event: Arc<Event>,
    close_event: Arc<Event>,
    closed: Arc<AtomicBool>,

    #[allow(dead_code)]
    onopen: SendWrapper<Closure<dyn Fn()>>,
    #[allow(dead_code)]
    onclose: SendWrapper<Closure<dyn Fn()>>,
    #[allow(dead_code)]
    onerror: SendWrapper<Closure<dyn Fn(JsValue)>>,
    #[allow(dead_code)]
    onmessage: SendWrapper<Closure<dyn Fn(MessageEvent)>>,
}

impl Drop for BrowserWebSocketTransport {
    fn drop(&mut self) {
        self.inner.set_onopen(None);
        self.inner.set_onclose(None);
        self.inner.set_onerror(None);
        self.inner.set_onmessage(None);
    }
}

impl BrowserWebSocketTransport {
    fn connect(url: &str, protocols: &[String]) -> Result<(Self, WebSocketReader), JsValue> {
        let (read_tx, read_rx) = flume::unbounded();
        let closed = Arc::new(AtomicBool::new(false));

        let open_event = Arc::new(Event::new());
        let close_event = Arc::new(Event::new());
        let error_event = Arc::new(Event::new());

        let onopen_event = open_event.clone();
        let onopen = Closure::wrap(
            Box::new(move || while onopen_event.notify(usize::MAX) == 0 {}) as Box<dyn Fn()>,
        );

        let onmessage_tx = read_tx.clone();
        let onmessage = Closure::wrap(Box::new(move |evt: MessageEvent| {
            if let Ok(arr) = evt.data().dyn_into::<ArrayBuffer>() {
                let _ =
                    onmessage_tx.send(WebSocketMessage::Message(Uint8Array::new(&arr).to_vec()));
            }
        }) as Box<dyn Fn(MessageEvent)>);

        let onclose_closed = closed.clone();
        let onclose_event = close_event.clone();
        let onclose = Closure::wrap(Box::new(move || {
            onclose_closed.store(true, Ordering::Release);
            onclose_event.notify(usize::MAX);
        }) as Box<dyn Fn()>);

        let onerror_tx = read_tx.clone();
        let onerror_closed = closed.clone();
        let onerror_close = close_event.clone();
        let onerror_event = error_event.clone();
        let onerror = Closure::wrap(Box::new(move |e| {
            let _ = onerror_tx.send(WebSocketMessage::Error(WebSocketTransportError::Unknown(
                format!("{e:?}"),
            )));
            onerror_closed.store(true, Ordering::Release);
            onerror_close.notify(usize::MAX);
            onerror_event.notify(usize::MAX);
        }) as Box<dyn Fn(JsValue)>);

        let ws = if protocols.is_empty() {
            WebSocket::new(url)
        } else {
            WebSocket::new_with_str_sequence(
                url,
                &protocols
                    .iter()
                    .fold(Array::new(), |acc, x| {
                        acc.push(&x.into());
                        acc
                    })
                    .into(),
            )
        }?;

        ws.set_binary_type(BinaryType::Arraybuffer);
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));

        Ok((
            Self {
                inner: Arc::new(SendWrapper::new(ws)),
                open_event,
                error_event,
                close_event: close_event.clone(),
                closed: closed.clone(),
                onopen: SendWrapper::new(onopen),
                onclose: SendWrapper::new(onclose),
                onerror: SendWrapper::new(onerror),
                onmessage: SendWrapper::new(onmessage),
            },
            WebSocketReader {
                read_rx,
                closed,
                close_event,
            },
        ))
    }

    async fn wait_for_open(&self) -> bool {
        if self.closed.load(Ordering::Acquire) {
            return false;
        }
        futures_util::select! {
            () = self.open_event.listen().fuse() => true,
            () = self.error_event.listen().fuse() => false,
        }
    }

    fn into_write(self) -> BrowserWispTransportWrite {
        let ws = self.inner.clone();
        let closed = self.closed.clone();
        let close_event = self.close_event.clone();
        Box::pin(async_iterator_transport_write(
            self,
            |this, item| {
                Box::pin(async move {
                    this.inner.send_with_u8_array(&item).map_err(|err| {
                        WispError::from(WebSocketTransportError::SendFailed(format!("{err:?}")))
                    })?;
                    Ok(this)
                })
            },
            (ws, closed, close_event),
            |(ws, closed, close_event)| {
                Box::pin(async move {
                    ws.set_onopen(None);
                    ws.set_onclose(None);
                    ws.set_onerror(None);
                    ws.set_onmessage(None);
                    closed.store(true, Ordering::Release);
                    close_event.notify(usize::MAX);
                    ws.close().map_err(|err| {
                        WispError::from(WebSocketTransportError::CloseFailed(format!("{err:?}")))
                    })
                })
            },
        ))
    }
}

#[wasm_bindgen]
pub struct GromnieWispClient {
    ws_url: String,
    protocols: Vec<String>,
    wisp_version_policy: WispVersionPolicy,
    pub(crate) state: Arc<Mutex<ClientState>>,
}

#[wasm_bindgen]
impl GromnieWispClient {
    #[wasm_bindgen(constructor)]
    pub fn new(ws_url: String) -> Self {
        Self {
            ws_url,
            protocols: Vec::new(),
            wisp_version_policy: WispVersionPolicy::default(),
            state: Arc::new(Mutex::new(ClientState::default())),
        }
    }

    pub fn set_allow_v1_downgrade(&mut self, allow: bool) {
        self.wisp_version_policy = if allow {
            WispVersionPolicy::AllowV1Downgrade
        } else {
            WispVersionPolicy::RequireV2
        };
    }

    pub async fn connect(&self) -> Result<(), JsValue> {
        let mut state = self.state.lock().await;
        if state.mux.is_some() {
            return Ok(());
        }

        let (transport, reader) =
            BrowserWebSocketTransport::connect(&self.ws_url, &self.protocols)?;
        if !transport.wait_for_open().await {
            return Err(JsValue::from_str("websocket failed to open"));
        }

        let handshake = WispV2Handshake::new(vec![AnyProtocolExtensionBuilder::new(
            UdpProtocolExtensionBuilder,
        )]);

        let client = ClientMux::new(reader.into_read(), transport.into_write(), Some(handshake))
            .await
            .map_err(js_error)?;
        let (mux, mux_task) = client.with_no_required_extensions();
        if self
            .wisp_version_policy
            .rejects_downgrade(mux.was_downgraded())
        {
            let _ = mux.close().await;
            let _ = mux_task.await;
            return Err(JsValue::from_str(downgrade_rejection_error_message()));
        }

        state.mux = Some(mux);
        state.last_mux_error = None;

        let state_ref = self.state.clone();
        spawn_local(async move {
            let result = mux_task.await;
            let mut state = state_ref.lock().await;
            state.mux = None;
            state.streams.clear();
            if let Err(err) = result {
                state.last_mux_error = Some(err.to_string());
            }
        });

        Ok(())
    }

    pub async fn open_tcp_stream(&self, host: String, port: u16) -> Result<u32, JsValue> {
        self.open_stream(StreamType::Tcp, host, port).await
    }

    pub async fn open_udp_stream(&self, host: String, port: u16) -> Result<u32, JsValue> {
        self.open_stream(StreamType::Udp, host, port).await
    }
}

impl GromnieWispClient {
    async fn open_stream(&self, ty: StreamType, host: String, port: u16) -> Result<u32, JsValue> {
        let mut state = self.state.lock().await;
        let mux = state.mux.as_ref().ok_or_else(|| {
            let msg = state.last_mux_error.as_deref().unwrap_or("not connected");
            JsValue::from_str(msg)
        })?;

        let stream = mux.new_stream(ty, host, port).await.map_err(js_error)?;
        let id = stream.get_stream_id();
        state.streams.insert(id, stream);
        Ok(id)
    }
}

fn downgrade_rejection_error_message() -> &'static str {
    "WISP downgrade rejected: server negotiated WISP v1; compatibility mode is required to allow downgrade"
}

use util::js_error;

pub mod client;

impl GromnieWispClient {
    /// Remove and return a stream by its ID, transferring ownership to the caller.
    pub async fn take_stream(&self, id: u32) -> Option<MuxStream<BrowserWispTransportWrite>> {
        let mut state = self.state.lock().await;
        state.streams.remove(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::WispVersionPolicy;

    #[test]
    fn require_v2_policy_rejects_downgrade() {
        assert!(WispVersionPolicy::RequireV2.rejects_downgrade(true));
    }

    #[test]
    fn require_v2_policy_accepts_v2_connection() {
        assert!(!WispVersionPolicy::RequireV2.rejects_downgrade(false));
    }

    #[test]
    fn compatibility_policy_allows_downgrade() {
        assert!(!WispVersionPolicy::AllowV1Downgrade.rejects_downgrade(true));
    }

    #[test]
    fn downgrade_rejection_error_is_clear() {
        assert_eq!(
            super::downgrade_rejection_error_message(),
            "WISP downgrade rejected: server negotiated WISP v1; compatibility mode is required to allow downgrade",
        );
    }
}
