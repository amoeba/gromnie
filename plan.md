# Implementation Plan: Expose All acprotocol Events to Scripts

## Overview

Remove the `SimpleGameEvent` abstraction and expose all acprotocol server-to-client events directly to WASM scripts using strongly-typed WIT records. This gives scripts access to the full protocol event stream with full type safety instead of the current 4 limited event types.

## Requirements (from user)
- ✅ Remove SimpleGameEvent abstraction as soon as we can
- ✅ Fire events for ALL top-level S2C messages
- ✅ Fire events for ALL nested game events (within OrderedGameEvent)
- ✅ Expose acprotocol as strongly-typed Rust types via WIT
- ✅ Keep SimpleClientAction for script→client actions
- ✅ Breaking change OK (no backward compatibility needed)

## Design Approach

**Key Insight:** Define WIT record types that mirror acprotocol message structures, providing full type safety across the WASM boundary.

### Event Flow
```
acprotocol S2CMessage/GameEvent (parsed)
    ↓ (convert to wrapper type)
Rust ProtocolEvent enum (in gromnie-events)
    ↓ (convert to WIT types)
WIT s2c-event / game-event variant
    ↓ (passed to WASM with wasmtime bindings)
Scripts receive strongly-typed Rust structs
```

### Two-Level Event Structure
The protocol has two event layers:
1. **Top-level S2C messages** - ~94 types (LoginCreatePlayer, ItemCreateObject, etc.)
2. **Nested game events** - ~150+ types (within OrderedGameEvent 0xF7B0)

Both will be exposed as separate variant groups in WIT:
- `variant s2c-event` with ~94 variants
- `variant game-event-msg` with ~150+ variants

### Implementation Strategy: Phased Rollout

**Phase 1: Core Event Types** (~20 most common types)
- Define WIT records for essential messages first
- Get the architecture working end-to-end
- Validate the approach with real scripts

** Phase 2: Design codegen system similar to codegen system in ~/src/amoeba/asheron-rs

**Phase 3: Comprehensive Coverage** (remaining ~220 types)
- we don't have to do this right away
- use codegen

## Implementation Steps

### Step 1: Define Core WIT Event Types

**File:** `crates/gromnie-scripting-api/src/wit/gromnie-script.wit`

Add strongly-typed event definitions. Start with ~20 most common types:

```wit
// ===== S2C Message Types =====

record login-create-player-msg {
    character-id: u32,
}

record login-character-set-msg {
    account: string,
    characters: list<character-info>,
    num-allowed-characters: u32,
}

record item-create-object-msg {
    object-id: u32,
    name: string,
    // Add other common WeenieDescription fields as needed
}

record character-error-msg {
    error-code: u32,
    error-message: string,
}

record hear-speech-msg {
    sender-name: string,
    message: string,
    message-type: u32,
}

// ... more S2C message records ...

/// Top-level server-to-client messages
variant s2c-event {
    login-create-player(login-create-player-msg),
    login-character-set(login-character-set-msg),
    item-create-object(item-create-object-msg),
    character-error(character-error-msg),
    hear-speech(hear-speech-msg),
    hear-ranged-speech(hear-speech-msg),
    // Add more variants as needed
}

// ===== Game Event Types =====

record hear-direct-speech-msg {
    message: string,
    sender-name: string,
    sender-id: u32,
    target-id: u32,
    message-type: u32,
}

record transient-string-msg {
    message: string,
}

// ... more game event records ...

/// Nested game events (from OrderedGameEvent wrapper)
variant game-event-msg {
    hear-direct-speech(hear-direct-speech-msg),
    transient-string(transient-string-msg),
    // Add more variants as needed
}

// ===== Unified Protocol Event =====

/// Wrapper for ordered game events with metadata
record ordered-game-event {
    object-id: u32,
    sequence: u32,
    event: game-event-msg,
}

/// Protocol event from server
variant protocol-event {
    s2c(s2c-event),
    game-event(ordered-game-event),
}

/// Update the existing game-event variant
variant game-event {
    // OLD: Legacy simplified events (can be removed after migration)
    character-list-received(account-data),
    character-error(character-error),
    create-object(world-object),
    chat-message-received(chat-message),

    // NEW: Full protocol access
    protocol(protocol-event),
}
```

### Step 2: Add ProtocolEvent to ClientEvent enum

**File:** `crates/gromnie-events/src/protocol_events.rs` (NEW)

Create Rust types that will convert to WIT types:

```rust
use serde::{Deserialize, Serialize};

/// Protocol event - mirrors WIT structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolEvent {
    S2C(S2CEvent),
    GameEvent(OrderedGameEvent),
}

/// Top-level S2C message events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum S2CEvent {
    LoginCreatePlayer { character_id: u32 },
    LoginCharacterSet {
        account: String,
        characters: Vec<CharacterData>,
        num_allowed_characters: u32,
    },
    ItemCreateObject {
        object_id: u32,
        name: String,
    },
    CharacterError {
        error_code: u32,
        error_message: String,
    },
    HearSpeech {
        sender_name: String,
        message: String,
        message_type: u32,
    },
    HearRangedSpeech {
        sender_name: String,
        message: String,
        message_type: u32,
    },
    // Add more as needed
}

/// Nested game events with OrderedGameEvent metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderedGameEvent {
    pub object_id: u32,
    pub sequence: u32,
    pub event: GameEventMsg,
}

/// Game event messages (nested within OrderedGameEvent)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEventMsg {
    HearDirectSpeech {
        message: String,
        sender_name: String,
        sender_id: u32,
        target_id: u32,
        message_type: u32,
    },
    TransientString {
        message: String,
    },
    // Add more as needed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterData {
    pub id: u32,
    pub name: String,
    pub delete_pending: bool,
}
```

**File:** `crates/gromnie-events/src/client_events.rs`

Add new variant to ClientEvent:
```rust
#[derive(Debug, Clone)]
pub enum ClientEvent {
    Game(SimpleGameEvent),       // Keep for backward compat during migration
    Protocol(ProtocolEvent),     // NEW: Full protocol events
    State(ClientStateEvent),
    System(ClientSystemEvent),
}
```

### Step 3: Add Conversion Functions

**File:** `crates/gromnie-events/src/protocol_events.rs` (continued)

Add conversion functions from acprotocol types:

```rust
impl From<&acprotocol::messages::s2c::LoginCreatePlayer> for S2CEvent {
    fn from(msg: &acprotocol::messages::s2c::LoginCreatePlayer) -> Self {
        S2CEvent::LoginCreatePlayer {
            character_id: msg.character_id.0,
        }
    }
}

impl From<&acprotocol::messages::s2c::LoginLoginCharacterSet> for S2CEvent {
    fn from(msg: &acprotocol::messages::s2c::LoginLoginCharacterSet) -> Self {
        S2CEvent::LoginCharacterSet {
            account: msg.account.clone(),
            characters: msg.characters.list.iter()
                .map(|c| CharacterData {
                    id: c.character_id.0,
                    name: c.name.clone(),
                    delete_pending: c.delete_pending != 0,
                })
                .collect(),
            num_allowed_characters: msg.num_allowed_characters,
        }
    }
}

// Add more From implementations for each S2CEvent variant...

// Conversion from game_event_handlers types
impl From<&crate::client::game_event_handlers::CommunicationHearDirectSpeech> for GameEventMsg {
    fn from(event: &crate::client::game_event_handlers::CommunicationHearDirectSpeech) -> Self {
        GameEventMsg::HearDirectSpeech {
            message: event.message.clone(),
            sender_name: event.sender_name.clone(),
            sender_id: event.sender_id,
            target_id: event.target_id,
            message_type: event.message_type,
        }
    }
}

// Add more From implementations for each GameEventMsg variant...
```

### Step 4: Emit Protocol Events in Message Handlers

**File:** `crates/gromnie-client/src/client/message_handlers.rs`

Update each handler to emit protocol events. For example:

```rust
impl MessageHandler<acprotocol::messages::s2c::LoginCreatePlayer> for Client {
    fn handle(
        &mut self,
        create_player: acprotocol::messages::s2c::LoginCreatePlayer,
    ) -> Option<GameEvent> {
        let character_id = create_player.character_id.0;

        // Existing business logic...
        self.transition_to_in_world(...);

        // NEW: Emit protocol event
        let protocol_event = ProtocolEvent::S2C(S2CEvent::from(&create_player));
        let _ = self.raw_event_tx.try_send(ClientEvent::Protocol(protocol_event));

        // KEEP: Emit legacy SimpleGameEvent (for backward compat)
        Some(GameEvent::CreatePlayer { character_id })
    }
}
```

Apply to all existing message handlers.

**For unhandled messages:** Add a catch-all emission in the dispatch loop:

**File:** `crates/gromnie-client/src/client/client.rs` (line ~1140)

```rust
_ => {
    // NEW: Try to emit protocol event even for unhandled messages
    if let Some(protocol_event) = try_convert_to_protocol_event(&msg_type, &message) {
        let _ = self.raw_event_tx.try_send(ClientEvent::Protocol(protocol_event));
    }

    info!(target: "net", "Unhandled S2CMessage: {:?} (0x{:04X})", msg_type, message.opcode);
}
```

### Step 5: Emit Protocol Events for Game Events

**File:** `crates/gromnie-client/src/client/game_event_handlers.rs`

Update game event handlers similarly:

```rust
impl GameEventHandler<CommunicationHearDirectSpeech> for Client {
    fn handle(&mut self, event: CommunicationHearDirectSpeech) -> Option<GameEvent> {
        // Existing business logic...
        let chat_text = format!("{} tells you, \"{}\"", event.sender_name, event.message);

        // NEW: Emit protocol event
        // Extract object_id and sequence from context (need to pass these in)
        let protocol_event = ProtocolEvent::GameEvent(OrderedGameEvent {
            object_id: self.current_game_event_object_id,  // Add this field to Client
            sequence: self.current_game_event_sequence,     // Add this field to Client
            event: GameEventMsg::from(&event),
        });
        let _ = self.raw_event_tx.try_send(ClientEvent::Protocol(protocol_event));

        // KEEP: Emit legacy SimpleGameEvent
        Some(GameEvent::ChatMessageReceived {
            message: chat_text,
            message_type: event.message_type,
        })
    }
}
```

**File:** `crates/gromnie-client/src/client/client.rs` (line ~1151)

Modify `handle_game_event` to track object_id/sequence:

```rust
fn handle_game_event(&mut self, event_type: GameEventType, message: RawMessage) {
    // Parse object_id and sequence from message header
    let object_id = u32::from_le_bytes([message.data[0], message.data[1], message.data[2], message.data[3]]);
    let sequence = u32::from_le_bytes([message.data[4], message.data[5], message.data[6], message.data[7]]);

    // Store for handlers to use
    self.current_game_event_object_id = object_id;
    self.current_game_event_sequence = sequence;

    // Dispatch to handler...
    match event_type {
        // ...existing handlers...
    }
}
```

### Step 6: Update Script Host Conversion

**File:** `crates/gromnie-scripting-host/src/wasm/wasm_script.rs`

Update `client_event_to_wasm()` to convert ProtocolEvent to WIT types:

```rust
fn client_event_to_wasm(event: &ClientEvent) -> WitScriptEvent {
    match event {
        ClientEvent::Game(game_event) => {
            // OLD: Convert SimpleGameEvent (keep during migration)
            WitScriptEvent::Game(simple_game_event_to_wasm(game_event))
        }
        ClientEvent::Protocol(protocol_event) => {
            // NEW: Convert ProtocolEvent to WIT
            WitScriptEvent::Game(WitGameEvent::Protocol(
                protocol_event_to_wit(protocol_event)
            ))
        }
        // ... other variants
    }
}

fn protocol_event_to_wit(event: &ProtocolEvent) -> WitProtocolEvent {
    match event {
        ProtocolEvent::S2C(s2c_event) => {
            WitProtocolEvent::S2C(s2c_event_to_wit(s2c_event))
        }
        ProtocolEvent::GameEvent(game_event) => {
            WitProtocolEvent::GameEvent(WitOrderedGameEvent {
                object_id: game_event.object_id,
                sequence: game_event.sequence,
                event: game_event_msg_to_wit(&game_event.event),
            })
        }
    }
}

fn s2c_event_to_wit(event: &S2CEvent) -> WitS2CEvent {
    match event {
        S2CEvent::LoginCreatePlayer { character_id } => {
            WitS2CEvent::LoginCreatePlayer(WitLoginCreatePlayerMsg {
                character_id: *character_id,
            })
        }
        S2CEvent::LoginCharacterSet { account, characters, num_allowed_characters } => {
            WitS2CEvent::LoginCharacterSet(WitLoginCharacterSetMsg {
                account: account.clone(),
                characters: characters.iter().map(|c| WitCharacterInfo {
                    id: c.id,
                    name: c.name.clone(),
                    delete_pending: c.delete_pending,
                }).collect(),
                num_allowed_characters: *num_allowed_characters,
            })
        }
        // Add more conversions for each S2CEvent variant...
    }
}

fn game_event_msg_to_wit(event: &GameEventMsg) -> WitGameEventMsg {
    match event {
        GameEventMsg::HearDirectSpeech { message, sender_name, sender_id, target_id, message_type } => {
            WitGameEventMsg::HearDirectSpeech(WitHearDirectSpeechMsg {
                message: message.clone(),
                sender_name: sender_name.clone(),
                sender_id: *sender_id,
                target_id: *target_id,
                message_type: *message_type,
            })
        }
        // Add more conversions for each GameEventMsg variant...
    }
}
```

### Step 7: Update Test Scripts

**File:** `tests/scripting/src/lib.rs`

Update to handle strongly-typed protocol events:

```rust
fn on_event(&self, event: ScriptEvent) {
    match event {
        ScriptEvent::Game(GameEvent::Protocol(proto)) => {
            // Match on strongly-typed protocol events
            match proto {
                ProtocolEvent::S2C(s2c) => match s2c {
                    S2CEvent::ItemCreateObject(obj) => {
                        log(&format!("Object created: {} (0x{:08X})",
                            obj.name, obj.object_id));
                    }
                    S2CEvent::LoginCharacterSet(data) => {
                        log(&format!("Characters for {}: {} chars",
                            data.account, data.characters.len()));
                    }
                    _ => {} // Other S2C events
                }
                ProtocolEvent::GameEvent(game_event) => {
                    match game_event.event {
                        GameEventMsg::HearDirectSpeech(msg) => {
                            log(&format!("{} tells you: {}",
                                msg.sender_name, msg.message));
                        }
                        GameEventMsg::TransientString(msg) => {
                            log(&format!("System: {}", msg.message));
                        }
                        _ => {} // Other game events
                    }
                }
            }
        }
        // Keep old event handling during migration period
        ScriptEvent::Game(GameEvent::CreateObject(obj)) => {
            log(&format!("Object (legacy): {}", obj.name));
        }
        // ...
    }
}
```

### Step 8: Remove SimpleGameEvent (future cleanup)

After all consumers migrate to ProtocolEvent:
- Delete `crates/gromnie-events/src/simple_game_events.rs`
- Remove `ClientEvent::Game` variant
- Remove old WIT game-event variants (character-list-received, etc.)
- Update all references

## Critical Files

1. **`crates/gromnie-scripting-api/src/wit/gromnie-script.wit`** - Define WIT event types
2. **`crates/gromnie-events/src/protocol_events.rs`** (NEW) - Rust wrapper types
3. **`crates/gromnie-events/src/client_events.rs`** - Add ProtocolEvent variant
4. **`crates/gromnie-client/src/client/message_handlers.rs`** - Emit protocol events in handlers
5. **`crates/gromnie-client/src/client/game_event_handlers.rs`** - Emit game event protocol events
6. **`crates/gromnie-scripting-host/src/wasm/wasm_script.rs`** - Convert Rust → WIT types
7. **`tests/scripting/src/lib.rs`** - Update test scripts

## Phased Implementation Approach

### Phase 1: Core Event Types (~20 most common)
Start with essential events to validate the architecture:
- LoginCreatePlayer, LoginCharacterSet
- ItemCreateObject, CharacterError
- HearSpeech, HearRangedSpeech
- HearDirectSpeech, TransientString

**Goal:** Get end-to-end flow working with real scripts

### Phase 2: Expand Coverage (remaining ~220 types)
Two options for comprehensive coverage:

**Option B: Code generation**
- Write codegen to generate WIT from acprotocol types
- One-time effort, automatic coverage
- Need to handle acprotocol dependency location

**Recommendation:** Start with Option A (manual), switch to Option B if we need 50+ types

## Testing Strategy

1. Define core WIT types and Rust wrappers
2. Implement conversions for core types
3. Update message handlers to emit protocol events
4. Update one test script to handle new events
5. Verify events flow through to scripts correctly
6. Test pattern matching on strongly-typed events
7. Incrementally add more event types as needed
8. Eventually remove SimpleGameEvent

## Advantages of This Approach

- **Full type safety** - Scripts get strongly-typed Rust structs via WIT bindings
- **Compile-time checking** - Invalid event handling caught at script compile time
- **Better IDE support** - Autocomplete and type hints in script development
- **Explicit API** - Clear contract between host and scripts
- **Future-proof** - Easy to add new types incrementally
- **Performance** - No JSON serialization/parsing overhead

## Risks & Mitigations

**Risk:** Manually defining 240+ WIT types is tedious
**Mitigation:** Start with ~20 core types, add incrementally. Consider codegen for bulk.

**Risk:** WIT enum variants limited to reasonable count
**Mitigation:** Group related events, use separate enums for S2C vs GameEvent

**Risk:** Breaking change impacts all scripts
**Mitigation:** Acceptable per user; keep SimpleGameEvent during migration

**Risk:** acprotocol types may be complex to mirror in WIT
**Mitigation:** Simplify fields, omit rarely-used data, document differences

**Risk:** Maintaining sync between acprotocol and WIT types
**Mitigation:** Use From traits with clear conversion errors. Consider codegen.

## Implementation Timeline

### Initial Implementation (Phase 1 - Core Types)
1. **Day 1** (4-5 hours): Define core WIT types, create protocol_events.rs
2. **Day 2** (3-4 hours): Implement conversions, update message handlers
3. **Day 3** (2-3 hours): Update script host, test scripts, validate flow

**Total Phase 1:** ~10-12 hours to get core working

### Expansion (Phase 2 - Full Coverage)
- **Manual:** 5-10 min × 220 types = 18-37 hours (can be spread over time)
- **Codegen:** 8-12 hours one-time investment for generator

**Recommendation:** Phase 1 immediately, Phase 2 incrementally as needed
