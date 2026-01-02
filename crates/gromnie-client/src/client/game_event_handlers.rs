//! Game event handler trait implementations.
//!
//! This module contains all GameEventHandler trait implementations for the Client.
//! Each handler focuses on business logic only - parsing and error handling
//! are centralized in the game_event_handler module.

use tracing::info;

use crate::client::Client;
use crate::client::GameEvent;
use crate::client::game_event_handler::GameEventHandler;
use acprotocol::readers::{ACDataType, ACReader};

/// Communication_HearDirectSpeech game event (tell messages)
/// Format: Message, SenderName, SenderId, TargetId, Type, SecretFlags
pub struct CommunicationHearDirectSpeech {
    pub message: String,
    pub sender_name: String,
    pub sender_id: u32,
    pub target_id: u32,
    pub message_type: u32,
}

impl ACDataType for CommunicationHearDirectSpeech {
    fn read(cursor: &mut dyn ACReader) -> Result<Self, Box<dyn std::error::Error>> {
        let message = String::read(cursor)?;
        let sender_name = String::read(cursor)?;
        let sender_id = u32::read(cursor)?;
        let target_id = u32::read(cursor)?;
        let message_type = u32::read(cursor)?;

        Ok(CommunicationHearDirectSpeech {
            message,
            sender_name,
            sender_id,
            target_id,
            message_type,
        })
    }
}

/// Handle Communication_HearDirectSpeech game events
impl GameEventHandler<CommunicationHearDirectSpeech> for Client {
    fn handle(&mut self, event: CommunicationHearDirectSpeech) -> Option<GameEvent> {
        let chat_text = format!("{} tells you, \"{}\"", event.sender_name, event.message);

        info!(target: "net", "Direct speech received - Type: {}, Text: {}", event.message_type, chat_text);

        Some(GameEvent::ChatMessageReceived {
            message: chat_text,
            message_type: event.message_type,
        })
    }
}

/// Communication_TransientString game event (status messages)
/// Format: just a string message
pub struct CommunicationTransientString {
    pub message: String,
}

impl ACDataType for CommunicationTransientString {
    fn read(cursor: &mut dyn ACReader) -> Result<Self, Box<dyn std::error::Error>> {
        let message = String::read(cursor)?;
        Ok(CommunicationTransientString { message })
    }
}

/// Handle Communication_TransientString game events
impl GameEventHandler<CommunicationTransientString> for Client {
    fn handle(&mut self, event: CommunicationTransientString) -> Option<GameEvent> {
        info!(target: "net", "Transient string: {}", event.message);

        Some(GameEvent::ChatMessageReceived {
            message: event.message,
            message_type: 0x05, // System message type
        })
    }
}
