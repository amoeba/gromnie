use serde::{Deserialize, Serialize};

/// Simplified versions of acprotocol GameEvent/OrderedGameEvent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimpleGameEvent {
    CharacterListReceived {
        account: String,
        characters: Vec<acprotocol::types::CharacterIdentity>,
        num_slots: u32,
    },
    /// Character login succeeded - received LoginComplete notification
    LoginSucceeded {
        character_id: u32,
        character_name: String,
    },
    /// Character login failed
    LoginFailed {
        reason: String,
    },
    /// Character error received from server
    CharacterError {
        error_code: u32,
        error_message: String,
    },
    /// Chat message received from server
    ChatMessageReceived {
        message: String,
        message_type: u32,
    },
    CreatePlayer {
        character_id: u32,
    },
    /// Progress update for connecting phase
    ConnectingSetProgress {
        progress: f64,
    },
    /// Progress update for patching/updating phase
    UpdatingSetProgress {
        progress: f64,
    },
    /// Item created with full details
    ItemCreateObject {
        object_id: u32,
        name: String,
        item_type: String,
        container_id: Option<u32>,
        burden: u32,
        value: u32,
        items_capacity: Option<u32>,
        container_capacity: Option<u32>,
    },
    /// Container contents received
    ItemOnViewContents {
        container_id: u32,
        items: Vec<u32>,
    },
    /// Player containers received (initial inventory list)
    PlayerContainersReceived {
        player_id: u32,
        containers: Vec<u32>,
    },
    /// Item deleted from world
    ItemDeleteObject {
        object_id: u32,
    },
    /// Item moved between containers
    ItemMovedObject {
        object_id: u32,
        new_container_id: u32,
    },
    /// Quality/property integer updated on an object
    QualitiesPrivateUpdateInt {
        object_id: u32,
        property_name: String,
        value: i32,
    },
    /// Generic item state update
    ItemSetState {
        object_id: u32,
        property_name: String,
        value: i32,
    },
    /// World name received from server
    WorldNameReceived {
        world_name: String,
    },
}
