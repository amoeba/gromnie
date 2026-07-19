use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use gromnie_client::client::ClientEvent;
use gromnie_client::client::SimpleClientAction;

use crate::GromnieWispClient;
use crate::transport::{NetLogCallback, WispUdpTransport, format_net_entry};

type AsyncCallback = Rc<RefCell<Option<js_sys::Function>>>;

#[wasm_bindgen]
pub struct WasmClient {
    wisp_client: Option<GromnieWispClient>,
    action_tx: Option<tokio::sync::mpsc::UnboundedSender<SimpleClientAction>>,
    on_event: AsyncCallback,
    on_net_log: NetLogCallback,
}

fn js_error(err: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&err.to_string())
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

#[wasm_bindgen]
impl WasmClient {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
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
        ws_url: String,
        server_host: String,
        account_name: String,
        password: String,
    ) -> Result<(), JsValue> {
        web_sys::console::log_1(&"[wasm] step 1: creating WISP client".into());

        // 1. Create and connect WISP client
        let wisp_client = GromnieWispClient::new(ws_url);
        wisp_client.connect().await?;

        web_sys::console::log_1(&"[wasm] step 2: opening UDP stream".into());

        // 2. Open UDP stream to game server (login port 9000)
        let stream_id = wisp_client
            .open_udp_stream(server_host.clone(), 9000)
            .await?;

        web_sys::console::log_1(&format!("[wasm] step 3: taking stream id={}", stream_id).into());

        // 3. Take the stream out of the WISP client
        let stream = wisp_client
            .take_stream(stream_id)
            .await
            .ok_or_else(|| JsValue::from_str("stream not found after opening"))?;

        web_sys::console::log_1(&"[wasm] step 4: creating transport".into());

        // 4. Create WISP UDP transport from the stream
        let net_log = self.on_net_log.clone();
        let transport = WispUdpTransport::new(wisp_client.state.clone(), stream, net_log);

        web_sys::console::log_1(&"[wasm] step 5: creating event channel".into());

        // 5. Create event channel
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(256);

        web_sys::console::log_1(&"[wasm] step 6: creating gromnie client".into());

        // 6. Create the gromnie client with our WISP transport
        let address = format!("{}:9000", server_host);
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
        client.do_login().await.map_err(|e| js_error(e))?;

        web_sys::console::log_1(&"[wasm] step 8: spawning recv loop".into());

        // 8. Spawn the recv loop with keepalive
        let net_log_ref = self.on_net_log.clone();
        spawn_local(async move {
            let mut buf = vec![0u8; 65536];
            let mut last_keepalive_ms = js_sys::Date::now() as u64;
            const KEEPALIVE_INTERVAL_MS: u64 = 5000;

            loop {
                match client.recv_packet(&mut buf).await {
                    Ok((len, addr)) => {
                        if let Some(cb) = net_log_ref.borrow().as_ref() {
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
                        break;
                    }
                }
            }
        });

        // 9. Spawn event forwarder
        let on_event_ref = self.on_event.clone();
        spawn_local(async move {
            let mut rx = event_rx;
            while let Some(event) = rx.recv().await {
                web_sys::console::log_1(&format!("[event] {:?}", event).into());
                if let Some(cb) = on_event_ref.borrow().as_ref() {
                    cb.call1(&JsValue::NULL, &event_to_js(&event)).ok();
                }
            }
        });

        // Store state
        self.wisp_client = Some(wisp_client);
        self.action_tx = Some(action_tx);

        Ok(())
    }

    pub fn select_character(&self, character_id: u32, account: &str) -> Result<(), JsValue> {
        let tx = self
            .action_tx
            .as_ref()
            .ok_or_else(|| js_error("not connected"))?;

        tx.send(SimpleClientAction::LoginCharacter {
            character_id,
            character_name: String::new(),
            account: account.to_string(),
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
}
