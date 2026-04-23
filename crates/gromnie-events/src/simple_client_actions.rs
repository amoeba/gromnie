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

    // ===== Trading =====
    /// Open trade negotiations with another player (TradeOpenTradeNegotiations)
    OpenTrade { partner_id: u32 },
    /// Add an item to the trade window at a given slot (TradeAddToTrade)
    AddToTrade { item_id: u32, slot: u32 },
    /// Accept the current trade (TradeAcceptTrade) - uses server-registered trade data
    AcceptTrade,
    /// Decline the current trade (TradeDeclineTrade)
    DeclineTrade,
    /// Reset the current trade (TradeResetTrade)
    ResetTrade,
    /// Close trade negotiations (TradeCloseTradeNegotiations)
    CloseTrade,

    // ===== Spell Casting =====
    /// Cast a targeted spell at a specific object (MagicCastTargetedSpell)
    CastTargetedSpell { target_id: u32, spell_id: u32 },
    /// Cast an untargeted (self or area) spell (MagicCastUntargetedSpell)
    CastUntargetedSpell { spell_id: u32 },
}
