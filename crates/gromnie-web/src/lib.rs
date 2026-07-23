use std::{
    collections::HashMap,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
};

use event_listener::Event;
use flume::Receiver;
use futures::lock::Mutex;
use futures_util::FutureExt;
use js_sys::{Array, ArrayBuffer, Uint8Array};
use send_wrapper::SendWrapper;
use thiserror::Error;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::{BinaryType, MessageEvent, WebSocket};
use wisp_mux::{
    ClientMux, WispError,
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

#[derive(Error, Debug)]
enum WebSocketTransportError {
    #[error("websocket error: {0:?}")]
    Unknown(JsValue),
    #[error("websocket send failed: {0:?}")]
    SendFailed(JsValue),
    #[error("websocket close failed: {0:?}")]
    CloseFailed(JsValue),
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

/// WebSocket connection state, tracked in a single shared enum instead of
/// separate `Event`/`AtomicBool` instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WsState {
    Connecting,
    Open,
    Closed,
    Error,
}

impl WsState {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn from_u8(v: u8) -> Self {
        match v {
            0 => WsState::Connecting,
            1 => WsState::Open,
            2 => WsState::Closed,
            3 => WsState::Error,
            _ => WsState::Connecting,
        }
    }
}

/// Shared WebSocket state: a single `Event` plus an atomic state enum.
///
/// This replaces the previous four separate synchronization primitives
/// (`open_event`, `error_event`, `close_event`, `closed: AtomicBool`)
/// with a single `Event` that fires on any state transition.
struct SharedWsState {
    state: AtomicU8,
    event: Event,
}

impl SharedWsState {
    fn new() -> Self {
        Self {
            state: AtomicU8::new(WsState::Connecting.as_u8()),
            event: Event::new(),
        }
    }

    fn set_state(&self, state: WsState) {
        self.state.store(state.as_u8(), Ordering::Release);
        self.event.notify(usize::MAX);
    }

    fn get_state(&self) -> WsState {
        WsState::from_u8(self.state.load(Ordering::Acquire))
    }
}

struct WebSocketReader {
    read_rx: Receiver<WebSocketMessage>,
    shared: Arc<SharedWsState>,
}

impl WebSocketReader {
    fn into_read(self) -> BrowserWispTransportRead {
        Box::pin(async_iterator_transport_read(self, |this| {
            Box::pin(async move {
                use WebSocketMessage as M;
                match this.shared.get_state() {
                    WsState::Closed | WsState::Error => return Err(WispError::WsImplSocketClosed),
                    _ => {}
                }

                let res = futures_util::select! {
                    data = this.read_rx.recv_async() => data.ok(),
                    () = this.shared.event.listen().fuse() => None
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
    shared: Arc<SharedWsState>,

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

/// Callbacks registered on the browser `WebSocket`.
struct WsCallbacks {
    onopen: Closure<dyn Fn()>,
    onclose: Closure<dyn Fn()>,
    onerror: Closure<dyn Fn(JsValue)>,
    onmessage: Closure<dyn Fn(MessageEvent)>,
}

/// Register all four WebSocket event callbacks and wire them to the shared
/// state and message channel.
fn register_callbacks(
    ws: &WebSocket,
    shared: &Arc<SharedWsState>,
    read_tx: flume::Sender<WebSocketMessage>,
) -> WsCallbacks {
    let onopen_shared = shared.clone();
    let onopen = Closure::wrap(Box::new(move || {
        onopen_shared.set_state(WsState::Open);
    }) as Box<dyn Fn()>);

    let onmessage_tx = read_tx.clone();
    let onmessage = Closure::wrap(Box::new(move |evt: MessageEvent| {
        if let Ok(arr) = evt.data().dyn_into::<ArrayBuffer>() {
            let _ = onmessage_tx.send(WebSocketMessage::Message(Uint8Array::new(&arr).to_vec()));
        }
    }) as Box<dyn Fn(MessageEvent)>);

    let onclose_shared = shared.clone();
    let onclose = Closure::wrap(Box::new(move || {
        onclose_shared.set_state(WsState::Closed);
    }) as Box<dyn Fn()>);

    let onerror_tx = read_tx;
    let onerror_shared = shared.clone();
    let onerror = Closure::wrap(Box::new(move |e| {
        let _ = onerror_tx.send(WebSocketMessage::Error(WebSocketTransportError::Unknown(e)));
        onerror_shared.set_state(WsState::Error);
    }) as Box<dyn Fn(JsValue)>);

    ws.set_binary_type(BinaryType::Arraybuffer);
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));

    WsCallbacks {
        onopen,
        onclose,
        onerror,
        onmessage,
    }
}

impl BrowserWebSocketTransport {
    fn connect(url: &str, protocols: &[String]) -> Result<(Self, WebSocketReader), JsValue> {
        let (read_tx, read_rx) = flume::unbounded();
        let shared = Arc::new(SharedWsState::new());

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

        let callbacks = register_callbacks(&ws, &shared, read_tx);

        Ok((
            Self {
                inner: Arc::new(SendWrapper::new(ws)),
                shared: shared.clone(),
                onopen: SendWrapper::new(callbacks.onopen),
                onclose: SendWrapper::new(callbacks.onclose),
                onerror: SendWrapper::new(callbacks.onerror),
                onmessage: SendWrapper::new(callbacks.onmessage),
            },
            WebSocketReader { read_rx, shared },
        ))
    }

    async fn wait_for_open(&self) -> bool {
        loop {
            match self.shared.get_state() {
                WsState::Open => return true,
                WsState::Error | WsState::Closed => return false,
                WsState::Connecting => {}
            }
            self.shared.event.listen().await;
        }
    }

    fn into_write(self) -> BrowserWispTransportWrite {
        let ws = self.inner.clone();
        let shared = self.shared.clone();
        Box::pin(async_iterator_transport_write(
            self,
            |this, item| {
                Box::pin(async move {
                    this.inner
                        .send_with_u8_array(&item)
                        .map_err(|err| WispError::from(WebSocketTransportError::SendFailed(err)))?;
                    Ok(this)
                })
            },
            (ws, shared),
            |(ws, shared)| {
                Box::pin(async move {
                    ws.set_onopen(None);
                    ws.set_onclose(None);
                    ws.set_onerror(None);
                    ws.set_onmessage(None);
                    shared.set_state(WsState::Closed);
                    ws.close()
                        .map_err(|err| WispError::from(WebSocketTransportError::CloseFailed(err)))
                })
            },
        ))
    }
}

/// Internal WISP-over-WebSocket transport layer.
///
/// This struct is used internally by [`WasmClient`] to establish a WISP
/// multiplexed connection over a browser `WebSocket`. It is not exported
/// to JavaScript — use `WasmClient` for the public API.
pub(crate) struct GromnieWispClient {
    ws_url: String,
    protocols: Vec<String>,
    allow_v1_downgrade: bool,
    pub(crate) state: Arc<Mutex<ClientState>>,
}

impl GromnieWispClient {
    pub fn new(ws_url: String) -> Self {
        Self {
            ws_url,
            protocols: Vec::new(),
            allow_v1_downgrade: false,
            state: Arc::new(Mutex::new(ClientState::default())),
        }
    }

    #[allow(dead_code)]
    pub fn set_allow_v1_downgrade(&mut self, allow: bool) {
        self.allow_v1_downgrade = allow;
    }

    /// Returns `true` if a WISP v1 downgrade should be rejected.
    ///
    /// Downgrade is rejected when `allow_v1_downgrade` is `false` (the default)
    /// and the server actually negotiated WISP v1.
    fn should_reject_downgrade(&self, downgraded: bool) -> bool {
        !self.allow_v1_downgrade && downgraded
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

        let handshake = gromnie_wisp::default_wisp_handshake();

        let client = ClientMux::new(reader.into_read(), transport.into_write(), Some(handshake))
            .await
            .map_err(js_error)?;
        let (mux, mux_task) = client.with_no_required_extensions();
        if self.should_reject_downgrade(mux.was_downgraded()) {
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

    #[allow(dead_code)]
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

    /// Create a `WispUdpTransport` from a stream ID, encapsulating the
    /// stream-take and state-clone steps so callers don't need to access
    /// `self.state` directly.
    pub async fn create_udp_transport(
        &self,
        stream_id: u32,
        net_log: crate::transport::NetLogCallback,
    ) -> Result<crate::transport::WispUdpTransport, JsValue> {
        let stream = self
            .take_stream(stream_id)
            .await
            .ok_or_else(|| JsValue::from_str("stream not found after opening"))?;
        Ok(crate::transport::WispUdpTransport::new(
            self.state.clone(),
            stream,
            net_log,
        ))
    }

    /// Close the mux and all streams, terminating the WebSocket connection.
    pub async fn close(&self) {
        let mut state = self.state.lock().await;
        state.streams.clear();
        if let Some(mux) = state.mux.take() {
            let _ = mux.close().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GromnieWispClient;

    #[test]
    fn default_policy_rejects_downgrade() {
        let client = GromnieWispClient::new("ws://localhost".to_string());
        assert!(client.should_reject_downgrade(true));
    }

    #[test]
    fn default_policy_accepts_v2_connection() {
        let client = GromnieWispClient::new("ws://localhost".to_string());
        assert!(!client.should_reject_downgrade(false));
    }

    #[test]
    fn compatibility_mode_allows_downgrade() {
        let mut client = GromnieWispClient::new("ws://localhost".to_string());
        client.set_allow_v1_downgrade(true);
        assert!(!client.should_reject_downgrade(true));
    }

    #[test]
    fn downgrade_rejection_error_is_clear() {
        assert_eq!(
            super::downgrade_rejection_error_message(),
            "WISP downgrade rejected: server negotiated WISP v1; compatibility mode is required to allow downgrade",
        );
    }
}
