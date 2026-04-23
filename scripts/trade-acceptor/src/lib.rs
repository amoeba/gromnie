//! Trade Acceptor Script
//!
//! Watches for incoming trade requests and automatically accepts them.
//! Demonstrates the full trading interaction loop.
//!
//! Flow:
//!  1. Another player initiates a trade with us
//!  2. Server sends TradeRegistered (gives us initiator/partner IDs and stamp)
//!  3. Server sends TradeOpened (trade window is open)
//!  4. We call accept_trade() to accept
//!  5. Server confirms with TradeAccepted
//!
//! Note: In a real script you would inspect the items offered before accepting.

use gromnie::host_interface::ProtocolEvent;
use gromnie::ScriptEvent;
use gromnie_scripting_api as gromnie;

#[derive(Default)]
pub struct TradeAcceptorScript {
    trade_open: bool,
}

impl gromnie::Script for TradeAcceptorScript {
    fn new() -> Self {
        Self { trade_open: false }
    }

    fn id(&self) -> &str {
        "trade_acceptor"
    }

    fn name(&self) -> &str {
        "Trade Acceptor"
    }

    fn description(&self) -> &str {
        "Automatically accepts all incoming trade requests."
    }

    fn on_load(&mut self) {
        gromnie::log("Trade Acceptor script loaded. Will auto-accept all incoming trades.");
    }

    fn on_unload(&mut self) {
        gromnie::log("Trade Acceptor script unloaded.");
        if self.trade_open {
            gromnie::close_trade();
        }
    }

    fn subscribed_events(&self) -> Vec<u32> {
        vec![0xFFFFFFFF]
    }

    fn on_event(&mut self, event: ScriptEvent) {
        use gromnie::host::*;

        if let ScriptEvent::Game(GameEvent::Protocol(proto)) = event {
            if let ProtocolEvent::GameEvent(ordered) = proto {
                match ordered.event {
                    GameEventMsg::TradeRegistered(msg) => {
                        // Server has registered a new trade session between two players.
                        // The client caches this internally so accept_trade() can use it.
                        gromnie::log(&format!(
                            "Trade registered: initiator=0x{:08X}, partner=0x{:08X}, stamp={}",
                            msg.initiator_id, msg.partner_id, msg.stamp
                        ));
                    }
                    GameEventMsg::TradeOpened(msg) => {
                        // Trade window opened on our end.
                        gromnie::log(&format!(
                            "Trade window opened (object_id=0x{:08X}). Auto-accepting...",
                            msg.object_id
                        ));
                        self.trade_open = true;
                        // In a real script you might wait to inspect offered items first.
                        // Here we accept immediately.
                        gromnie::accept_trade();
                    }
                    GameEventMsg::TradeItemAdded(msg) => {
                        gromnie::log(&format!("Item 0x{:08X} added to trade window", msg.item_id));
                    }
                    GameEventMsg::TradeItemRemoved(msg) => {
                        gromnie::log(&format!(
                            "Item 0x{:08X} removed from trade window",
                            msg.item_id
                        ));
                    }
                    GameEventMsg::TradeAccepted => {
                        gromnie::log("Trade accepted by a participant.");
                    }
                    GameEventMsg::TradeDeclined => {
                        gromnie::log("Trade declined.");
                        self.trade_open = false;
                    }
                    GameEventMsg::TradeReset => {
                        gromnie::log("Trade reset.");
                    }
                    GameEventMsg::TradeClosed => {
                        gromnie::log("Trade closed.");
                        self.trade_open = false;
                    }
                    GameEventMsg::TradeFailure(msg) => {
                        gromnie::log(&format!("Trade failed (reason={})", msg.reason));
                        self.trade_open = false;
                    }
                    _ => {}
                }
            }
        }
    }

    fn on_tick(&mut self, _delta_millis: u64) {}
}

gromnie::register_script!(TradeAcceptorScript);
