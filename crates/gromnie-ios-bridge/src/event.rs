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

#[cfg(test)]
mod tests {
    use super::*;

    fn event(kind: BridgeEventKind) -> BridgeEvent {
        BridgeEvent {
            sequence: 1,
            timestamp_ms: 123,
            kind,
        }
    }

    #[test]
    fn characters_event_has_the_documented_shape() {
        let value = serde_json::to_value(event(BridgeEventKind::Characters {
            account: "acct".to_string(),
            slots: 3,
            characters: vec![Character {
                id: 1,
                name: "Alice".to_string(),
            }],
        }))
        .unwrap();

        assert_eq!(value["type"], "characters");
        assert_eq!(value["sequence"], 1);
        assert_eq!(value["timestamp_ms"], 123);
        assert_eq!(value["account"], "acct");
        assert_eq!(value["slots"], 3);
        assert_eq!(value["characters"][0]["id"], 1);
        assert_eq!(value["characters"][0]["name"], "Alice");
    }

    #[test]
    fn chat_event_has_the_documented_shape() {
        let value = serde_json::to_value(event(BridgeEventKind::Chat {
            message: "hello".to_string(),
            message_type: 2,
        }))
        .unwrap();

        assert_eq!(value["type"], "chat");
        assert_eq!(value["message"], "hello");
        assert_eq!(value["message_type"], 2);
    }

    #[test]
    fn error_event_carries_a_recovery_target() {
        let value = serde_json::to_value(event(BridgeEventKind::Error {
            code: "login",
            message: "nope".to_string(),
            recover_to: "characters",
        }))
        .unwrap();

        assert_eq!(value["type"], "error");
        assert_eq!(value["code"], "login");
        assert_eq!(value["recover_to"], "characters");
    }

    #[test]
    fn disconnected_event_marks_user_initiated() {
        let value = serde_json::to_value(event(BridgeEventKind::Disconnected {
            reason: "done".to_string(),
            user_initiated: true,
        }))
        .unwrap();

        assert_eq!(value["type"], "disconnected");
        assert_eq!(value["reason"], "done");
        assert_eq!(value["user_initiated"], true);
    }
}
