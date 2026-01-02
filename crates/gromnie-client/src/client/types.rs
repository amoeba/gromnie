use crate::client::messages::OutgoingMessageContent;

// Note: CharacterInfo is now defined in gromnie-events and re-exported

/// Internal client actions (extends SimpleClientAction with internal-only variants)
#[derive(Debug)]
pub enum ClientAction {
    /// Send a raw message to the server (internal only)
    SendMessage(Box<OutgoingMessageContent>),
    /// Disconnect from the server
    Disconnect,
    /// Log in as a specific character
    LoginCharacter {
        character_id: u32,
        character_name: String,
        account: String,
    },
    /// Send LoginComplete notification to server
    SendLoginComplete,
    /// Send a chat message to everyone nearby (CommunicationTalk)
    SendChatSay {
        message: String,
    },
    /// Send a direct message to a specific player (CommunicationTalkDirectByName)
    SendChatTell {
        recipient_name: String,
        message: String,
    },
    /// Reload scripts
    ReloadScripts {
        script_dir: std::path::PathBuf,
    },
    /// Log a message from a script
    LogScriptMessage {
        script_id: String,
        message: String,
    },
}
