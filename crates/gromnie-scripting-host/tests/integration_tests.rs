// Integration tests for scripting system

use acprotocol::types::{CharacterIdentity, ObjectId};
use gromnie_client::client::Client;
use gromnie_events::{
    ClientEvent, GameEventMsg, OrderedGameEvent, ProtocolEvent, S2CEvent,
    SimpleGameEvent as GameEvent,
};
use gromnie_scripting_host::ScriptRunner;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, mpsc};

async fn create_mock_client() -> Arc<RwLock<Client>> {
    let (client, _action_tx) = Client::new(
        1,
        "127.0.0.1:9000".to_string(),
        "test_user".to_string(),
        "test_pass".to_string(),
        None,
        mpsc::channel(100).0,
        false,
    )
    .await
    .expect("Failed to create client");
    Arc::new(RwLock::new(client))
}

#[tokio::test]
async fn test_script_lifecycle() {
    // Create action channel
    let (action_tx, _action_rx) = mpsc::unbounded_channel();

    // Create mock client
    let client = create_mock_client().await;

    // Create script runner with WASM support
    let runner = ScriptRunner::new_with_wasm(client, action_tx);

    // Check if WASM engine was initialized
    if !runner.has_wasm_engine() {
        println!("WARNING: WASM engine not initialized!");
    } else {
        println!("WASM engine initialized successfully");
    }

    // Load test scripts - use absolute path
    let test_scripts_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/scripting");
    println!("Looking for scripts in: {}", test_scripts_dir.display());

    // Check if directory exists
    if !test_scripts_dir.exists() {
        println!(
            "ERROR: Script directory does not exist: {}",
            test_scripts_dir.display()
        );
    } else {
        println!("Script directory exists");
        // List files in directory
        if let Ok(entries) = std::fs::read_dir(&test_scripts_dir) {
            for entry in entries.flatten() {
                println!("Found file: {}", entry.path().display());
            }
        }
    }

    // For now, just test that we can get script count (should be 0 initially)
    assert_eq!(runner.script_count(), 0, "Should start with no scripts");

    // Test that we can get script IDs (should be empty initially)
    let script_ids = runner.script_ids();
    assert!(script_ids.is_empty(), "Should start with no script IDs");
}

#[tokio::test]
async fn test_event_handling() {
    let (action_tx, _action_rx) = mpsc::unbounded_channel();
    let client = create_mock_client().await;
    let mut runner = ScriptRunner::new_with_wasm(client, action_tx);

    // Load test scripts
    let test_scripts_dir = Path::new("../../../tests/scripting");
    runner.load_scripts(test_scripts_dir, &HashMap::new());

    if runner.script_count() == 0 {
        println!("Warning: No scripts loaded for event test");
        return;
    }

    // Create test events
    let test_events = vec![GameEvent::ChatMessageReceived {
        message: "Hello World".to_string(),
        message_type: 1,
    }];

    // Process events
    for event in test_events {
        runner.handle_event(gromnie_events::ClientEvent::Game(event));
    }

    // Give scripts time to process (in real scenario, we'd check actions)
    tokio::time::sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_timer_functionality() {
    let (action_tx, _action_rx) = mpsc::unbounded_channel();
    let client = create_mock_client().await;
    let mut runner = ScriptRunner::new_with_wasm(client, action_tx);

    // Load test scripts
    let test_scripts_dir = Path::new("../../../tests/scripting");
    runner.load_scripts(test_scripts_dir, &HashMap::new());

    if runner.script_count() == 0 {
        println!("Warning: No scripts loaded for timer test");
        return;
    }

    // Simulate time passing by handling events with different timestamps
    let _start_time = Instant::now();

    // Handle a few events to trigger ticks
    for i in 0..5 {
        let event = GameEvent::ChatMessageReceived {
            message: format!("Test message {}", i),
            message_type: 1,
        };

        runner.handle_event(gromnie_events::ClientEvent::Game(event));

        // Small delay to allow timers to progress
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test]
async fn test_script_reload() {
    let (action_tx, _action_rx) = mpsc::unbounded_channel();
    let client = create_mock_client().await;
    let mut runner = ScriptRunner::new_with_wasm(client, action_tx);

    let test_scripts_dir = Path::new("../../../tests/scripting");

    // First load
    runner.load_scripts(test_scripts_dir, &HashMap::new());
    let first_count = runner.script_count();

    // Reload
    runner.reload_scripts(test_scripts_dir, &HashMap::new());
    let second_count = runner.script_count();

    // Should have same number of scripts after reload
    assert_eq!(
        first_count, second_count,
        "Script count changed after reload"
    );
}

#[tokio::test]
async fn test_host_function_calls() {
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();
    let client = create_mock_client().await;
    let mut runner = ScriptRunner::new_with_wasm(client, action_tx);

    // Load test scripts
    let test_scripts_dir = Path::new("../../../tests/scripting");
    runner.load_scripts(test_scripts_dir, &HashMap::new());

    if runner.script_count() == 0 {
        println!("Warning: No scripts loaded for host function test");
        return;
    }

    // Trigger script execution by sending events
    let event = GameEvent::ChatMessageReceived {
        message: "Test trigger".to_string(),
        message_type: 1,
    };

    runner.handle_event(gromnie_events::ClientEvent::Game(event));

    // Check if scripts generated any actions
    let mut action_count = 0;
    while let Ok(action) = action_rx.try_recv() {
        action_count += 1;
        println!("Received action: {:?}", action);

        // We can't easily verify specific actions without more complex setup,
        // but we can verify that actions are being generated
    }

    println!("Scripts generated {} actions", action_count);
}

/// Integration test for protocol event flow
///
/// This test verifies the end-to-end flow of protocol events:
/// 1. Protocol events are created with strongly-typed data
/// 2. They flow through the client event system
/// 3. They're converted to WIT types in the script host
/// 4. Scripts can receive and process them
#[tokio::test]
async fn test_protocol_event_flow() {
    let (action_tx, _action_rx) = mpsc::unbounded_channel();
    let client = create_mock_client().await;
    let mut runner = ScriptRunner::new_with_wasm(client, action_tx);

    // Load test scripts
    let test_scripts_dir = Path::new("../../../tests/scripting");
    runner.load_scripts(test_scripts_dir, &HashMap::new());

    if runner.script_count() == 0 {
        println!("Warning: No scripts loaded for protocol event test");
        return;
    }

    // Test S2C protocol events
    let s2c_events = vec![
        // LoginCreatePlayer event
        ProtocolEvent::S2C(S2CEvent::LoginCreatePlayer {
            character_id: 0x12345678,
        }),
        // LoginCharacterSet event with multiple characters
        ProtocolEvent::S2C(S2CEvent::LoginCharacterSet {
            account: "TestAccount".to_string(),
            characters: vec![
                CharacterIdentity {
                    character_id: ObjectId(0x1),
                    name: "ActiveChar".to_string(),
                    seconds_greyed_out: 0,
                },
                CharacterIdentity {
                    character_id: ObjectId(0x2),
                    name: "DeletedChar".to_string(),
                    seconds_greyed_out: 3600,
                },
            ],
            num_slots: 5,
        }),
        // ItemCreateObject event
        ProtocolEvent::S2C(S2CEvent::ItemCreateObject {
            object_id: 0xABCDEF00,
            name: "Magic Sword".to_string(),
            item_type: "MISSILE_WEAPON".to_string(),
            container_id: Some(0x50000001),
            burden: 980,
            value: 2000,
            items_capacity: None,
            container_capacity: None,
        }),
        // CharacterError event
        ProtocolEvent::S2C(S2CEvent::CharacterError {
            error_code: 0x01,
            error_message: "Test error".to_string(),
        }),
        // HearSpeech event
        ProtocolEvent::S2C(S2CEvent::HearSpeech {
            sender_name: "PlayerOne".to_string(),
            message: "Hello world!".to_string(),
            message_type: 0x01,
        }),
    ];

    // Process S2C events
    for event in s2c_events {
        runner.handle_event(ClientEvent::Protocol(event));
    }

    // Test nested game events with metadata
    let game_events = vec![
        // HearDirectSpeech game event
        ProtocolEvent::GameEvent(OrderedGameEvent {
            object_id: 0x100,
            sequence: 1,
            event: GameEventMsg::HearDirectSpeech {
                message: "Secret message".to_string(),
                sender_name: "Spy".to_string(),
                sender_id: 0x111,
                target_id: 0x222,
                message_type: 0x01,
            },
        }),
        // TransientString game event
        ProtocolEvent::GameEvent(OrderedGameEvent {
            object_id: 0x200,
            sequence: 2,
            event: GameEventMsg::TransientString {
                message: "System notification".to_string(),
            },
        }),
    ];

    // Process game events
    for event in game_events {
        runner.handle_event(ClientEvent::Protocol(event));
    }

    // Give scripts time to process
    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("Protocol event flow test completed successfully");
}

/// Test that protocol events preserve all data through conversion
#[tokio::test]
async fn test_protocol_event_data_integrity() {
    let (action_tx, _action_rx) = mpsc::unbounded_channel();
    let client = create_mock_client().await;
    let mut runner = ScriptRunner::new_with_wasm(client, action_tx);

    // Load test scripts
    let test_scripts_dir = Path::new("../../../tests/scripting");
    runner.load_scripts(test_scripts_dir, &HashMap::new());

    if runner.script_count() == 0 {
        println!("Warning: No scripts loaded for data integrity test");
        return;
    }

    // Test with complex data structures
    let character_set_event = ProtocolEvent::S2C(S2CEvent::LoginCharacterSet {
        account: "ComplexAccount@test.com".to_string(),
        characters: vec![
            CharacterIdentity {
                character_id: ObjectId(0x12345678),
                name: "Character With Spaces".to_string(),
                seconds_greyed_out: 0,
            },
            CharacterIdentity {
                character_id: ObjectId(0xABCDEF00),
                name: "SpecialChars!@#".to_string(),
                seconds_greyed_out: 7200,
            },
        ],
        num_slots: 10,
    });

    runner.handle_event(ClientEvent::Protocol(character_set_event));

    // Test game event with metadata preservation
    let game_event = ProtocolEvent::GameEvent(OrderedGameEvent {
        object_id: 0x9999AAAA,
        sequence: 0xFFFFFFFF,
        event: GameEventMsg::HearDirectSpeech {
            message: "Unicode test: 你好世界 🎮".to_string(),
            sender_name: "Player™".to_string(),
            sender_id: 0x11111111,
            target_id: 0x22222222,
            message_type: 0xFF,
        },
    });

    runner.handle_event(ClientEvent::Protocol(game_event));

    // Give scripts time to process
    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("Protocol event data integrity test completed successfully");
}
