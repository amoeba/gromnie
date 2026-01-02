/// Simplified versions of GameAction
#[derive(Debug, Clone)]
pub enum SimpleClientAction {
    /// Send a chat message to everyone nearby (CommunicationTalk)
    SendChatSay { message: String },
    /// Send a direct message to a specific player by name (CommunicationTalkDirectByName)
    SendChatTell {
        recipient_name: String,
        message: String,
    },
    /// Log in as a specific character
    LoginCharacter {
        character_id: u32,
        character_name: String,
        account: String,
    },
    /// Send LoginComplete notification to server after receiving initial objects
    SendLoginComplete,
    /// Disconnect from the server
    Disconnect,
    /// Reload scripts from the given directory
    ReloadScripts { script_dir: std::path::PathBuf },
    /// Log a message from a script
    LogScriptMessage { script_id: String, message: String },
}
