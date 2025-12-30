use std::time::Instant;

use acprotocol::messages::c2s::{
    CharacterSendCharGenResult, DDDInterrogationResponseMessage, LoginSendEnterWorld,
};

/// Enum for outgoing messages to be sent in the network loop
#[derive(Debug, Clone)]
pub enum OutgoingMessageContent {
    DDDInterrogationResponse(DDDInterrogationResponseMessage),
    CharacterCreation(CharacterSendCharGenResult),
    // ACE-compatible character creation (uses custom serialization format)
    CharacterCreationAce(String, crate::client::ace_protocol::AceCharGenResult),
    // Character login request - sent when user clicks "Enter" on character select
    EnterWorldRequest,
    // Character login - sent when selecting a character to enter the game world (after server ready)
    EnterWorld(LoginSendEnterWorld),
    // GameAction message (raw bytes including opcode)
    GameAction(Vec<u8>),
}

/// Struct for outgoing messages that may have attributes like delay, queue, etc.
#[derive(Debug)]
pub struct OutgoingMessage {
    pub content: OutgoingMessageContent,
    pub deadline: Option<Instant>, // None means send immediately
                                   // Future attributes could include: queue, priority, etc.
}

impl OutgoingMessage {
    /// Create a new immediate message (send right away)
    pub fn new(content: OutgoingMessageContent) -> Self {
        Self {
            content,
            deadline: None,
        }
    }

    /// Add a delay to the message (in seconds from now)
    pub fn with_delay(mut self, delay_seconds: u64) -> Self {
        self.deadline = Some(Instant::now() + std::time::Duration::from_secs(delay_seconds));
        self
    }

    /// Add a delay to the message (in milliseconds from now)
    pub fn with_delay_ms(mut self, delay_ms: u64) -> Self {
        self.deadline = Some(Instant::now() + std::time::Duration::from_millis(delay_ms));
        self
    }

    /// Check if this message is ready to be sent
    pub fn is_ready(&self) -> bool {
        match self.deadline {
            None => true,                                 // No deadline, send immediately
            Some(deadline) => Instant::now() >= deadline, // Deadline passed, send now
        }
    }
}
