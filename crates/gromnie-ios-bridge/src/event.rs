use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeEventKind {
    Connecting,
    Characters {
        account: String,
        slots: u32,
        characters: Vec<Character>,
    },
    EnteringWorld {
        character_id: u32,
    },
    EnteredWorld {
        character_id: u32,
        character_name: String,
    },
    Chat {
        message: String,
        message_type: u32,
    },
    Error {
        code: &'static str,
        message: String,
        recover_to: &'static str,
    },
    Disconnected {
        reason: String,
        user_initiated: bool,
    },
}

#[derive(Debug, Serialize)]
pub struct Character {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct BridgeEvent {
    pub sequence: u64,
    pub timestamp_ms: u128,
    #[serde(flatten)]
    pub kind: BridgeEventKind,
}
