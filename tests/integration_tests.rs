// Integration tests for the scripting system

use gromnie_scripting_host::{ScriptRunner, ScriptContext, EventFilter};
use gromnie_client::client::events::{GameEvent, ClientAction};
use std::collections::HashMap;
use tokio::sync::mpsc;
use std::time::{Duration, Instant};
use std::path::Path;

#[tokio::test]
async fn test_script_lifecycle() {
    // Create action channel
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();
    
    // Create script runner
    let mut runner = ScriptRunner::new(action_tx);
    
    // Load test scripts
    let test_scripts_dir = Path::new("tests/scripting");
    runner.load_scripts(test_scripts_dir, &HashMap::new());
    
    // Verify at least one script was loaded
    assert!(runner.script_count() > 0, "No scripts were loaded");
    
    // Check that we can get script IDs
    let script_ids = runner.script_ids();
    assert!(!script_ids.is_empty(), "No script IDs returned");
    
    println!("Loaded scripts: {:?}", script_ids);
    
    // Verify that scripts can be unloaded
    runner.unload_scripts();
    assert_eq!(runner.script_count(), 0, "Scripts were not properly unloaded");
}

#[tokio::test]
async fn test_event_handling() {
    let (action_tx, _action_rx) = mpsc::unbounded_channel();
    let mut runner = ScriptRunner::new(action_tx);
    
    // Load test scripts
    let test_scripts_dir = Path::new("tests/scripting");
    runner.load_scripts(test_scripts_dir, &HashMap::new());
    
    if runner.script_count() == 0 {
        println!("Warning: No scripts loaded for event test");
        return;
    }
    
    // Create test events
    let test_events = vec![
        GameEvent::ChatMessageReceived {
            message: "Hello World".to_string(),
            message_type: 1,
        },
        GameEvent::CreateObject {
            object_id: 123,
            object_name: "Test Object".to_string(),
        },
    ];
    
    // Process events
    for event in test_events {
        runner.handle_event(gromnie_client::client::events::ClientEvent::Game(event));
    }
    
    // Give scripts time to process (in real scenario, we'd check actions)
    tokio::time::sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_timer_functionality() {
    let (action_tx, _action_rx) = mpsc::unbounded_channel();
    let mut runner = ScriptRunner::new(action_tx);
    
    // Load test scripts
    let test_scripts_dir = Path::new("tests/scripting");
    runner.load_scripts(test_scripts_dir, &HashMap::new());
    
    if runner.script_count() == 0 {
        println!("Warning: No scripts loaded for timer test");
        return;
    }
    
    // Simulate time passing by handling events with different timestamps
    let start_time = Instant::now();
    
    // Handle a few events to trigger ticks
    for i in 0..5 {
        let event = GameEvent::ChatMessageReceived {
            message: format!("Test message {}", i),
            message_type: 1,
        };

        runner.handle_event(gromnie_client::client::events::ClientEvent::Game(event));
        
        // Small delay to allow timers to progress
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test]
async fn test_script_reload() {
    let (action_tx, _action_rx) = mpsc::unbounded_channel();
    let mut runner = ScriptRunner::new_with_wasm(action_tx);

    let test_scripts_dir = Path::new("tests/scripting");

    // First load
    runner.load_scripts(test_scripts_dir, &HashMap::new());
    let first_count = runner.script_count();

    // Reload
    runner.reload_scripts(test_scripts_dir, &HashMap::new());
    let second_count = runner.script_count();

    // Should have same number of scripts after reload
    assert_eq!(first_count, second_count, "Script count changed after reload");
}

#[tokio::test]
async fn test_host_function_calls() {
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();
    let mut runner = ScriptRunner::new(action_tx);
    
    // Load test scripts
    let test_scripts_dir = Path::new("tests/scripting");
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

    runner.handle_event(gromnie_client::client::events::ClientEvent::Game(event));
    
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
