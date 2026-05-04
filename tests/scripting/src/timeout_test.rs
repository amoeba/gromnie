// Timeout test script that sleeps longer than the configured timeout
// This will be compiled to WASM for testing the timeout functionality

use gromnie_scripting_api as gromnie;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Default)]
pub struct TimeoutTestScript {
    sleep_duration_ms: u64,
}

impl gromnie::Script for TimeoutTestScript {
    fn new() -> Self {
        Self {
            sleep_duration_ms: 200, // Sleep for 200ms (should timeout at default 100ms)
        }
    }

    fn id(&self) -> &str {
        "timeout_test_script"
    }

    fn name(&self) -> &str {
        "Timeout Test Script"
    }

    fn description(&self) -> &str {
        "Script that sleeps longer than timeout to test timeout functionality"
    }

    fn on_load<'a>(&'a mut self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'a>> {
        Box::pin(async move {
            gromnie::log("TimeoutTestScript loaded successfully");
        })
    }

    fn on_unload<'a>(
        &'a mut self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'a>> {
        Box::pin(async move {
            gromnie::log("TimeoutTestScript unloaded successfully");
        })
    }

    fn subscribed_events(&self) -> Vec<u32> {
        // Subscribe to chat messages for testing
        vec![3] // ChatMessageReceived event ID
    }

    fn on_event<'a>(
        &'a mut self,
        _event: gromnie::ScriptEvent,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'a>> {
        let sleep_duration_ms = self.sleep_duration_ms;
        Box::pin(async move {
            let start_time = gromnie::get_event_time_millis();
            gromnie::log(&format!("TimeoutTestScript: Starting event handler (will sleep {}ms)", sleep_duration_ms));
            
            // Simulate a long-running operation by spinning (WASI might not have sleep)
            // This is intentionally blocking to test timeout functionality
            let start = gromnie::get_event_time_millis();
            let target = start + sleep_duration_ms;
            
            // Busy-wait loop to simulate blocking I/O
            loop {
                let current = gromnie::get_event_time_millis();
                if current >= target {
                    break;
                }
                // Add a small yield to prevent completely blocking the runtime
                // In a real scenario this would be a blocking I/O operation
            }
            
            let end_time = gromnie::get_event_time_millis();
            let actual_duration = end_time - start_time;
            
            gromnie::log(&format!("TimeoutTestScript: Event handler completed after {}ms", actual_duration));
        })
    }

    fn on_tick<'a>(
        &'a mut self,
        _delta_millis: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'a>> {
        let sleep_duration_ms = self.sleep_duration_ms;
        Box::pin(async move {
            let start_time = gromnie::get_event_time_millis();
            gromnie::log(&format!("TimeoutTestScript: Starting tick (will sleep {}ms)", sleep_duration_ms));
            
            // Simulate a long-running operation
            let start = gromnie::get_event_time_millis();
            let target = start + sleep_duration_ms;
            
            // Busy-wait loop to simulate blocking I/O
            loop {
                let current = gromnie::get_event_time_millis();
                if current >= target {
                    break;
                }
            }
            
            let end_time = gromnie::get_event_time_millis();
            let actual_duration = end_time - start_time;
            
            gromnie::log(&format!("TimeoutTestScript: Tick completed after {}ms", actual_duration));
        })
    }
}

gromnie::register_script!(TimeoutTestScript);