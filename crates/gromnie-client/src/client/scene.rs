use acprotocol::enums::CharacterErrorType;
use gromnie_events::CharacterInfo;

/// Sub-states for Connecting phase with progress tracking
#[derive(Clone, Debug, PartialEq)]
pub enum ConnectingProgress {
    /// Initial state - 0%
    Initial,
    /// LoginRequest sent - 33%
    LoginRequestSent,
    /// ConnectRequest received - 66%
    ConnectRequestReceived,
    /// ConnectResponse sent - 100% (ready to transition)
    ConnectResponseSent,
}

/// Sub-states for Patching phase with progress tracking
#[derive(Clone, Debug, PartialEq)]
pub enum PatchingProgress {
    /// Haven't started patching yet
    NotStarted,
    /// Sent ConnectResponse, waiting for DDD file list
    WaitingForDDD,
    /// Got DDD, computing response
    ReceivedDDD,
    /// Sent response, waiting for character list
    SentDDDResponse,
    /// Received character list
    Complete,
}

/// Replaces CharacterLoginState - now part of CharacterSelectScene
#[derive(Clone, Debug)]
pub struct EnteringWorldState {
    pub character_id: u32,
    pub character_name: String,
    pub account: String,
    pub login_complete: bool, // Set once Character_LoginCompleteNotification is received
}

/// Scene-based UI state - represents what the user sees
#[derive(Clone, Debug)]
pub enum Scene {
    Connecting(ConnectingScene),
    CharacterSelect(CharacterSelectScene),
    CharacterCreate(CharacterCreateScene),
    InWorld(InWorldScene),
    Error(ErrorScene),
}

#[derive(Clone, Debug)]
pub struct ConnectingScene {
    pub connect_progress: ConnectingProgress,
    pub patch_progress: PatchingProgress,
    pub started_at: std::time::Instant,
    pub last_retry_at: std::time::Instant,
}

#[derive(Clone, Debug)]
pub struct CharacterSelectScene {
    pub account_name: String,
    pub characters: Vec<CharacterInfo>,
    pub entering_world: Option<EnteringWorldState>,
}

#[derive(Clone, Debug)]
pub struct CharacterCreateScene {
    pub character: Option<CharacterInfo>, // Stubbed for now
}

#[derive(Clone, Debug)]
pub struct InWorldScene {
    pub character_id: u32,
    pub character_name: String,
}

#[derive(Clone, Debug)]
pub struct ErrorScene {
    pub error: ClientError,
    pub can_retry: bool,
}

#[derive(Clone, Debug)]
pub enum ClientError {
    CharacterError(CharacterErrorType),
    ConnectionFailed(String),
    PatchingFailed(String),
    LoginTimeout,
    PatchingTimeout,
}

impl ConnectingScene {
    pub fn new() -> Self {
        let now = std::time::Instant::now();
        Self {
            connect_progress: ConnectingProgress::Initial,
            patch_progress: PatchingProgress::NotStarted,
            started_at: now,
            last_retry_at: now,
        }
    }

    /// Reset to initial state for reconnection attempts
    pub fn reset(&mut self) {
        let now = std::time::Instant::now();
        self.connect_progress = ConnectingProgress::Initial;
        self.patch_progress = PatchingProgress::NotStarted;
        self.started_at = now;
        self.last_retry_at = now;
    }

    /// Check if this connecting attempt has timed out (20s default)
    pub fn has_timed_out(&self, timeout: std::time::Duration) -> bool {
        self.started_at.elapsed() >= timeout
    }

    /// Check if it's time to retry (2s interval)
    pub fn should_retry(&self, retry_interval: std::time::Duration) -> bool {
        self.last_retry_at.elapsed() >= retry_interval
    }

    /// Update the last retry time
    pub fn update_retry_time(&mut self) {
        self.last_retry_at = std::time::Instant::now();
    }
}

impl CharacterSelectScene {
    pub fn new(account_name: String, characters: Vec<CharacterInfo>) -> Self {
        Self {
            account_name,
            characters,
            entering_world: None,
        }
    }

    /// Start entering the world with a character
    pub fn begin_entering_world(
        &mut self,
        character_id: u32,
        character_name: String,
        account: String,
    ) {
        self.entering_world = Some(EnteringWorldState {
            character_id,
            character_name,
            account,
            login_complete: false,
        });
    }

    /// Check if we're currently in the process of entering the world
    pub fn is_entering_world(&self) -> bool {
        self.entering_world.is_some()
    }

    /// Mark login as complete for the current entering_world state
    pub fn mark_login_complete(&mut self) {
        if let Some(entering) = &mut self.entering_world {
            entering.login_complete = true;
        }
    }

    /// Clear entering_world state (e.g., on error or disconnect)
    pub fn clear_entering_world(&mut self) {
        self.entering_world = None;
    }
}

impl CharacterCreateScene {
    pub fn new() -> Self {
        Self {
            character: None,
        }
    }
}

impl InWorldScene {
    pub fn new(character_id: u32, character_name: String) -> Self {
        Self {
            character_id,
            character_name,
        }
    }
}

impl ErrorScene {
    pub fn new(error: ClientError, can_retry: bool) -> Self {
        Self {
            error,
            can_retry,
        }
    }
}

impl Scene {
    /// Get a reference to the connecting scene if this is a Connecting scene
    pub fn as_connecting(&self) -> Option<&ConnectingScene> {
        match self {
            Scene::Connecting(scene) => Some(scene),
            _ => None,
        }
    }

    /// Get a mutable reference to the connecting scene if this is a Connecting scene
    pub fn as_connecting_mut(&mut self) -> Option<&mut ConnectingScene> {
        match self {
            Scene::Connecting(scene) => Some(scene),
            _ => None,
        }
    }

    /// Get a reference to the character select scene if this is a CharacterSelect scene
    pub fn as_character_select(&self) -> Option<&CharacterSelectScene> {
        match self {
            Scene::CharacterSelect(scene) => Some(scene),
            _ => None,
        }
    }

    /// Get a mutable reference to the character select scene if this is a CharacterSelect scene
    pub fn as_character_select_mut(&mut self) -> Option<&mut CharacterSelectScene> {
        match self {
            Scene::CharacterSelect(scene) => Some(scene),
            _ => None,
        }
    }

    /// Get a reference to the in world scene if this is an InWorld scene
    pub fn as_in_world(&self) -> Option<&InWorldScene> {
        match self {
            Scene::InWorld(scene) => Some(scene),
            _ => None,
        }
    }

    /// Get a reference to the error scene if this is an Error scene
    pub fn as_error(&self) -> Option<&ErrorScene> {
        match self {
            Scene::Error(scene) => Some(scene),
            _ => None,
        }
    }

    /// Check if this is an error scene that can be retried
    pub fn can_retry(&self) -> bool {
        match self {
            Scene::Error(scene) => scene.can_retry,
            _ => false,
        }
    }
}
