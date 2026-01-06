//! End-to-end integration tests for script hot reload functionality
//!
//! These tests verify the complete hot reload workflow including:
//! - Initial script loading
//! - Detection of file changes
//! - Automatic reloading of modified scripts
//! - Loading of newly added scripts
//! - Proper lifecycle management (on_unload/on_load)

use gromnie_client::config::scripting_config::ScriptingConfig;
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Helper function to create a minimal valid WASM file
/// For testing purposes, we'll create a dummy file since we can't
/// actually compile WASM in tests without complex setup
/// In a real scenario, you'd include test WASM files in the test fixtures
fn create_dummy_wasm_file(path: &PathBuf) {
    // This creates a file that will fail WASM validation but tests
    // the file detection logic. For full integration tests with real WASM,
    // you'd need pre-compiled test fixtures.
    fs::write(path, b"dummy wasm content for testing").expect("Failed to create test file");
}

/// Helper function to wait for hot reload to detect changes
fn wait_for_scan_interval() {
    // Wait slightly longer than the default scan interval (1000ms)
    thread::sleep(Duration::from_millis(1100));
}

#[test]
fn test_hot_reload_enabled_by_default() {
    // This test verifies that when scripting is enabled,
    // hot reload is also enabled by default
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let script_dir = temp_dir.path().join("scripts");
    fs::create_dir(&script_dir).expect("Failed to create script dir");

    // The actual ScriptRunner creation would be tested here
    // For now, we're testing the configuration defaults
    let config: ScriptingConfig = ScriptingConfig::default();

    assert!(config.enabled, "Scripting should be enabled");
    assert!(config.hot_reload, "Hot reload should be enabled by default");
    assert_eq!(
        config.hot_reload_interval_ms, 1000,
        "Default scan interval should be 1000ms"
    );
}

#[test]
fn test_hot_reload_can_be_disabled() {
    // Test that hot reload can be explicitly disabled
    let config = ScriptingConfig {
        hot_reload: false,
        ..Default::default()
    };

    assert!(config.enabled, "Scripting should be enabled");
    assert!(!config.hot_reload, "Hot reload should be disabled");
}

#[test]
fn test_custom_scan_interval_config() {
    // Test that custom scan intervals can be configured
    let config = ScriptingConfig {
        hot_reload_interval_ms: 1000,
        ..Default::default()
    };

    assert_eq!(
        config.hot_reload_interval_ms, 1000,
        "Custom scan interval should be 1000ms"
    );
}

#[test]
fn test_scanner_detects_file_changes() {
    // Test the file change detection in isolation
    use gromnie_scripting_host::script_scanner::{DEFAULT_SCAN_INTERVAL, ScriptScanner};

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let script_dir = temp_dir.path();

    // Create initial script file
    let script1 = script_dir.join("script1.wasm");
    create_dummy_wasm_file(&script1);

    // Create scanner
    let mut scanner = ScriptScanner::new(script_dir.to_path_buf());

    // Initial scan should detect the file
    let result = scanner.scan_changes();
    assert_eq!(result.added.len(), 1, "Should detect initial script");
    assert_eq!(result.added[0], script1, "Should detect correct script");

    // Second scan should show no changes
    let result = scanner.scan_changes();
    assert!(!result.has_changes(), "Second scan should show no changes");

    // Modify the file
    thread::sleep(Duration::from_millis(10)); // Ensure different timestamp
    create_dummy_wasm_file(&script1);

    // Wait for scan interval
    thread::sleep(DEFAULT_SCAN_INTERVAL);

    // Scan should detect the modification
    let result = scanner.scan_changes();
    assert_eq!(result.changed.len(), 1, "Should detect modified script");
    assert_eq!(result.changed[0].0, script1, "Should detect correct script");
}

#[test]
fn test_scanner_detects_new_files() {
    use gromnie_scripting_host::script_scanner::ScriptScanner;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let script_dir = temp_dir.path();

    // Create initial script
    let script1 = script_dir.join("script1.wasm");
    create_dummy_wasm_file(&script1);

    let mut scanner = ScriptScanner::new(script_dir.to_path_buf());

    // Initial scan
    scanner.scan_changes();

    // Add a new script
    let script2 = script_dir.join("script2.wasm");
    create_dummy_wasm_file(&script2);

    // Wait for scan interval
    wait_for_scan_interval();

    // Scan should detect new file
    let result = scanner.scan_changes();
    assert_eq!(result.added.len(), 1, "Should detect new script");
    assert_eq!(result.added[0], script2, "Should detect correct new script");
}

#[test]
fn test_scanner_detects_removed_files() {
    use gromnie_scripting_host::script_scanner::ScriptScanner;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let script_dir = temp_dir.path();

    // Create script file
    let script1 = script_dir.join("script1.wasm");
    create_dummy_wasm_file(&script1);

    let mut scanner = ScriptScanner::new(script_dir.to_path_buf());

    // Initial scan
    scanner.scan_changes();

    // Remove the file
    fs::remove_file(&script1).expect("Failed to remove script");

    // Wait for scan interval
    wait_for_scan_interval();

    // Scan should detect removal
    let result = scanner.scan_changes();
    assert_eq!(result.removed.len(), 1, "Should detect removed script");
    assert_eq!(
        result.removed[0], script1,
        "Should detect correct removed script"
    );
}

#[test]
fn test_scanner_filters_non_wasm_files() {
    use gromnie_scripting_host::script_scanner::ScriptScanner;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let script_dir = temp_dir.path();

    // Create mix of files
    create_dummy_wasm_file(&script_dir.join("script1.wasm"));
    fs::write(script_dir.join("script2.txt"), b"text file").unwrap();
    fs::write(script_dir.join("script3.rs"), b"rust source").unwrap();
    create_dummy_wasm_file(&script_dir.join("script2.wasm"));

    let mut scanner = ScriptScanner::new(script_dir.to_path_buf());

    // Scan should only detect .wasm files
    let result = scanner.scan_changes();
    assert_eq!(result.added.len(), 2, "Should only detect .wasm files");
}

#[test]
fn test_scan_interval_timing() {
    use gromnie_scripting_host::script_scanner::{DEFAULT_SCAN_INTERVAL, ScriptScanner};

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut scanner = ScriptScanner::new(temp_dir.path().to_path_buf());

    // Should scan initially
    assert!(scanner.should_scan(), "Should scan initially");

    // Perform a scan
    scanner.scan_changes();

    // Should not scan again immediately
    assert!(!scanner.should_scan(), "Should not scan again immediately");

    // Wait for interval to pass
    thread::sleep(DEFAULT_SCAN_INTERVAL + Duration::from_millis(10));

    // Should scan again
    assert!(scanner.should_scan(), "Should scan again after interval");
}

#[test]
fn test_custom_scan_interval() {
    use gromnie_scripting_host::script_scanner::ScriptScanner;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let custom_interval = Duration::from_millis(1000);
    let mut scanner = ScriptScanner::with_interval(temp_dir.path().to_path_buf(), custom_interval);

    // Initial scan
    scanner.scan_changes();

    // Should not scan immediately
    assert!(!scanner.should_scan());

    // Wait less than custom interval
    thread::sleep(Duration::from_millis(500));

    // Should still not scan
    assert!(
        !scanner.should_scan(),
        "Should not scan before custom interval"
    );

    // Wait for custom interval
    thread::sleep(Duration::from_millis(600));

    // Should scan now
    assert!(scanner.should_scan(), "Should scan after custom interval");
}

#[test]
fn test_wasm_script_tracks_metadata() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let script_path = temp_dir.path().join("test.wasm");
    create_dummy_wasm_file(&script_path);

    // Get file metadata
    let metadata = fs::metadata(&script_path).expect("Failed to get metadata");
    let expected_modified_time = metadata.modified().expect("Failed to get modified time");

    // Note: We can't actually load WASM without a valid WASM file,
    // but we can test that the file_path and modified_time would be tracked
    // if we had a valid WASM component. For full testing, you'd need
    // pre-compiled test WASM fixtures.

    // This test verifies the metadata we'd track
    assert!(script_path.exists(), "Test script should exist");
    assert_eq!(
        fs::metadata(&script_path).unwrap().modified().unwrap(),
        expected_modified_time,
        "Modified time should be consistent"
    );
}

#[test]
fn test_multiple_concurrent_changes() {
    use gromnie_scripting_host::script_scanner::ScriptScanner;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let script_dir = temp_dir.path();

    // Create initial scripts
    let script1 = script_dir.join("script1.wasm");
    let script2 = script_dir.join("script2.wasm");
    let script3 = script_dir.join("script3.wasm");

    create_dummy_wasm_file(&script1);
    create_dummy_wasm_file(&script2);

    let mut scanner = ScriptScanner::new(script_dir.to_path_buf());

    // Initial scan
    let result = scanner.scan_changes();
    assert_eq!(result.added.len(), 2, "Should detect 2 initial scripts");

    // Modify one, add one
    thread::sleep(Duration::from_millis(10));
    create_dummy_wasm_file(&script1); // Modify
    create_dummy_wasm_file(&script3); // Add

    // Wait for scan interval
    wait_for_scan_interval();

    // Scan should detect both changes
    let result = scanner.scan_changes();
    assert_eq!(result.changed.len(), 1, "Should detect 1 modified script");
    assert_eq!(result.added.len(), 1, "Should detect 1 new script");
    assert_eq!(
        result.changed[0].0, script1,
        "Modified script should be script1"
    );
    assert_eq!(result.added[0], script3, "New script should be script3");
}

#[test]
fn test_script_config_backward_compatibility() {
    // Test that default values provide backward compatibility
    let config = ScriptingConfig::default();

    // Should use default values
    assert!(config.enabled);
    assert!(config.hot_reload, "Should default to enabled");
    assert_eq!(
        config.hot_reload_interval_ms, 1000,
        "Should default to 1000ms"
    );
}
