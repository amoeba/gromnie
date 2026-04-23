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
    /// Send a movement command to the server (MovementDoMovementCommand)
    DoMovementCommand {
        /// Motion command value (see acprotocol Motion enum constants)
        motion: u32,
        /// Speed multiplier (1.0 = normal speed)
        speed: f32,
        /// Hold key modifier: 0=Invalid, 1=None, 2=Run
        hold_key: u32,
    },
    /// Stop a movement command (MovementStopMovementCommand)
    StopMovementCommand {
        /// Motion command to stop (must match the motion from DoMovementCommand)
        motion: u32,
        /// Hold key modifier (must match)
        hold_key: u32,
    },
}
