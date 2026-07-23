use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use gromnie_client::client::ClientEvent;
use gromnie_client::client::SimpleClientAction;

use crate::GromnieWispClient;
use crate::transport::{NetLogCallback, format_net_entry};
use crate::util::js_error;

type AsyncCallback = Rc<RefCell<Option<js_sys::Function>>>;

#[wasm_bindgen]
pub struct GromnieClient {
    wisp_url: String,
    account: String,
    wisp_client: Option<GromnieWispClient>,
    action_tx: Option<tokio::sync::mpsc::UnboundedSender<SimpleClientAction>>,
    on_event: AsyncCallback,
    on_net_log: NetLogCallback,
}

fn event_to_js(event: &ClientEvent) -> JsValue {
    let desc = match event {
        ClientEvent::Game(ge) => format!("game:{:?}", ge),
        ClientEvent::Protocol(pe) => format!("protocol:{:?}", pe),
        ClientEvent::State(se) => format!("state:{:?}", se),
        ClientEvent::System(se) => format!("system:{:?}", se),
    };
    JsValue::from_str(&desc)
}

/// Spawn the recv loop on the local executor.
///
/// Receives packets from the WISP transport, logs them, sends periodic
/// keepalives, and processes incoming messages. On error, emits a
/// `Disconnected` system event and terminates.
fn spawn_recv_loop(
    mut client: gromnie_client::client::Client,
    net_log: NetLogCallback,
    error_tx: tokio::sync::mpsc::Sender<ClientEvent>,
) {
    const KEEPALIVE_INTERVAL_MS: u64 = 5000;
    spawn_local(async move {
        let mut buf = vec![0u8; 65536];
        let mut last_keepalive_ms = js_sys::Date::now() as u64;

        loop {
            match client.recv_packet(&mut buf).await {
                Ok((len, addr)) => {
                    if let Some(cb) = net_log.borrow().as_ref() {
                        let msg = format_net_entry("RX", "?", &buf[..len]);
                        cb.call1(&JsValue::NULL, &msg.into()).ok();
                    }

                    let now_ms = js_sys::Date::now() as u64;
                    if now_ms - last_keepalive_ms >= KEEPALIVE_INTERVAL_MS {
                        if let Err(e) = client.send_keepalive().await {
                            web_sys::console::error_1(&format!("keepalive error: {e}").into());
                        }
                        last_keepalive_ms = now_ms;
                    }

                    client.process_packet(&buf[..len], len, &addr).await;
                    client.process_messages();
                    client.process_actions();
                    client.process_game_actions();
                    if let Err(e) = client.send_pending_messages().await {
                        web_sys::console::error_1(&format!("send_pending error: {e}").into());
                    }
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("recv error: {e}").into());
                    let _ = error_tx
                        .send(ClientEvent::System(
                            gromnie_client::client::ClientSystemEvent::Disconnected {
                                will_reconnect: false,
                                reconnect_attempt: 0,
                                delay_secs: 0,
                            },
                        ))
                        .await;
                    break;
                }
            }
        }
    });
}

/// Spawn the event forwarder on the local executor.
///
/// Receives `ClientEvent`s from the channel and forwards them to the
/// JS callback registered via `set_on_event`.
fn spawn_event_forwarder(
    event_rx: tokio::sync::mpsc::Receiver<ClientEvent>,
    on_event: AsyncCallback,
) {
    spawn_local(async move {
        let mut rx = event_rx;
        while let Some(event) = rx.recv().await {
            web_sys::console::log_1(&format!("[event] {:?}", event).into());
            if let Some(cb) = on_event.borrow().as_ref() {
                cb.call1(&JsValue::NULL, &event_to_js(&event)).ok();
            }
        }
    });
}

#[wasm_bindgen]
impl GromnieClient {
    #[wasm_bindgen(constructor)]
    pub fn new(wisp_url: String) -> Self {
        Self {
            wisp_url,
            account: String::new(),
            wisp_client: None,
            action_tx: None,
            on_event: Rc::new(RefCell::new(None)),
            on_net_log: Rc::new(RefCell::new(None)),
        }
    }

    pub fn set_on_event(&mut self, callback: js_sys::Function) {
        *self.on_event.borrow_mut() = Some(callback);
    }

    pub fn set_on_net_log(&mut self, callback: js_sys::Function) {
        *self.on_net_log.borrow_mut() = Some(callback);
    }

    pub async fn connect(
        &mut self,
        server_host: String,
        server_port: u16,
        account_name: String,
        password: String,
    ) -> Result<(), JsValue> {
        self.account = account_name.clone();

        web_sys::console::log_1(&"[wasm] step 1: creating WISP client".into());

        // 1. Create and connect WISP client
        let wisp_client = GromnieWispClient::new(self.wisp_url.clone());
        wisp_client.connect().await?;

        web_sys::console::log_1(&"[wasm] step 2: opening UDP stream".into());

        // 2. Open UDP stream to game server
        let stream_id = wisp_client
            .open_udp_stream(server_host.clone(), server_port)
            .await?;

        web_sys::console::log_1(&format!("[wasm] step 3: taking stream id={}", stream_id).into());

        // 3-4. Take the stream and create WISP UDP transport
        let net_log = self.on_net_log.clone();
        let transport = wisp_client.create_udp_transport(stream_id, net_log).await?;

        web_sys::console::log_1(&"[wasm] step 5: creating event channel".into());

        // 5. Create event channel
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(256);
        let error_tx_clone = event_tx.clone();

        web_sys::console::log_1(&"[wasm] step 6: creating gromnie client".into());

        // 6. Create the gromnie client with our WISP transport
        let address = format!("{}:{}", server_host, server_port);
        let (mut client, action_tx) = gromnie_client::client::Client::new_with_transport(
            1,
            address,
            account_name,
            password,
            None,
            event_tx,
            false,
            Box::new(transport),
        )
        .await;

        web_sys::console::log_1(&"[wasm] step 7: calling do_login".into());

        // 7. Send the first LoginRequest packet
        client.do_login().await.map_err(js_error)?;

        web_sys::console::log_1(&"[wasm] step 8: spawning recv loop".into());

        // 8. Spawn the recv loop with keepalive
        spawn_recv_loop(client, self.on_net_log.clone(), error_tx_clone);

        // 9. Spawn event forwarder
        spawn_event_forwarder(event_rx, self.on_event.clone());

        // Store state
        self.wisp_client = Some(wisp_client);
        self.action_tx = Some(action_tx);

        Ok(())
    }

    pub fn select_character(&self, character_id: u32) -> Result<(), JsValue> {
        let tx = self
            .action_tx
            .as_ref()
            .ok_or_else(|| js_error("not connected"))?;

        tx.send(SimpleClientAction::LoginCharacter {
            character_id,
            character_name: String::new(),
            account: self.account.clone(),
        })
        .map_err(|e| js_error(format!("send failed: {e}")))?;

        Ok(())
    }

    pub fn send_chat(&self, message: &str) -> Result<(), JsValue> {
        let tx = self
            .action_tx
            .as_ref()
            .ok_or_else(|| js_error("not connected"))?;

        tx.send(SimpleClientAction::SendChatSay {
            message: message.to_string(),
        })
        .map_err(|e| js_error(format!("send failed: {e}")))?;

        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), JsValue> {
        let tx = self
            .action_tx
            .as_ref()
            .ok_or_else(|| js_error("not connected"))?;

        tx.send(SimpleClientAction::Disconnect)
            .map_err(|e| js_error(format!("send failed: {e}")))?;

        // Close the WebSocket so the recv loop terminates and emits Disconnected
        if let Some(wisp) = &self.wisp_client {
            wisp.close().await;
        }

        Ok(())
    }
}
