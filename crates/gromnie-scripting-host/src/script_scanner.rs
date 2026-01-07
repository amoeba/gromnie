//! Script file change detection for hot reloading
//!
//! This module provides functionality to detect when script files have been
//! modified, added, or removed from the script directory.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};
use tracing::debug;

/// Default scan interval (1000ms = 1Hz)
pub const DEFAULT_SCAN_INTERVAL: Duration = Duration::from_millis(1000);

/// Result of scanning for script changes
#[derive(Debug, Clone)]
pub struct ScanResult {
    /// Scripts that were modified (path and new modification time)
    pub changed: Vec<(PathBuf, SystemTime)>,
    /// Scripts that were added to the directory
    pub added: Vec<PathBuf>,
    /// Scripts that were removed from the directory
    pub removed: Vec<PathBuf>,
}

impl ScanResult {
    /// Returns true if there are any changes detected
    pub fn has_changes(&self) -> bool {
        !self.changed.is_empty() || !self.added.is_empty() || !self.removed.is_empty()
    }
}

/// Scanner for detecting script file changes
pub struct ScriptScanner {
    /// Directory to scan for scripts
    script_dir: PathBuf,
    /// Time between scans
    scan_interval: Duration,
    /// Last time we performed a scan
    last_scan: Option<Instant>,
    /// Cached state from last scan: path -> modification time
    pub cached_state: HashMap<PathBuf, SystemTime>,
}

impl ScriptScanner {
    /// Create a new scanner with the default scan interval (500ms)
    pub fn new(script_dir: PathBuf) -> Self {
        Self::with_interval(script_dir, DEFAULT_SCAN_INTERVAL)
    }

    /// Create a new scanner with a custom scan interval
    pub fn with_interval(script_dir: PathBuf, scan_interval: Duration) -> Self {
        // Pre-populate the cache with current files to avoid detecting them as "added"
        let cached_state = Self::get_scripts_in_dir(&script_dir);

        Self {
            script_dir,
            scan_interval,
            last_scan: None,
            cached_state,
        }
    }

    /// Check if enough time has elapsed since the last scan
    pub fn should_scan(&self) -> bool {
        match self.last_scan {
            Some(last) => last.elapsed() >= self.scan_interval,
            None => true,
        }
    }

    /// Scan the script directory for changes
    ///
    /// This updates the internal cache and returns any detected changes.
    /// Only scans `.wasm` files, matching the behavior of the script loader.
    ///
    /// **Note:** The first scan after creation will detect all existing files
    /// as "added" since the cache starts empty. Callers should handle this
    /// initial scan appropriately.
    pub fn scan_changes(&mut self) -> ScanResult {
        let now = Instant::now();
        self.last_scan = Some(now);

        tracing::debug!(
            target: "scripting",
            "Scanning script directory for changes: {}",
            self.script_dir.display()
        );

        // Get current state of the directory
        let current_state = Self::get_scripts_in_dir(&self.script_dir);

        // Compare with cached state to detect changes
        let mut result = ScanResult {
            changed: Vec::new(),
            added: Vec::new(),
            removed: Vec::new(),
        };

        // Check for changed and added files
        for (path, modified_time) in &current_state {
            match self.cached_state.get(path) {
                Some(cached_time) => {
                    // File exists in both, check if modified
                    if cached_time != modified_time {
                        debug!(
                            target: "scripting",
                            "Script changed: {} (old: {:?}, new: {:?})",
                            path.display(),
                            cached_time,
                            modified_time
                        );
                        result.changed.push((path.clone(), *modified_time));
                    }
                }
                None => {
                    // New file
                    debug!(
                        target: "scripting",
                        "Script added: {}",
                        path.display()
                    );
                    result.added.push(path.clone());
                }
            }
        }

        // Check for removed files
        for path in self.cached_state.keys() {
            if !current_state.contains_key(path) {
                debug!(
                    target: "scripting",
                    "Script removed: {}",
                    path.display()
                );
                result.removed.push(path.clone());
            }
        }

        // Update cache
        self.cached_state = current_state;

        result
    }

    /// Get the current script files in the directory and their modification times
    ///
    /// Only returns `.wasm` files, matching the loader's behavior.
    fn get_scripts_in_dir(script_dir: &PathBuf) -> HashMap<PathBuf, SystemTime> {
        let mut scripts = HashMap::new();

        // Check if directory exists
        if !script_dir.exists() {
            debug!(
                target: "scripting",
                "Script directory does not exist: {}",
                script_dir.display()
            );
            return scripts;
        }

        // Read directory entries
        let entries = match std::fs::read_dir(script_dir) {
            Ok(entries) => entries,
            Err(e) => {
                tracing::warn!(
                    target: "scripting",
                    "Failed to read script directory {}: {}",
                    script_dir.display(),
                    e
                );
                return scripts;
            }
        };

        // Process each entry
        for entry in entries.flatten() {
            let path = entry.path();

            // Only load .wasm files
            if path.extension().and_then(|s| s.to_str()) != Some("wasm") {
                continue;
            }

            // Get file metadata
            match std::fs::metadata(&path) {
                Ok(metadata) => match metadata.modified() {
                    Ok(modified_time) => {
                        scripts.insert(path, modified_time);
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "scripting",
                            "Failed to get modification time for {}: {}",
                            path.display(),
                            e
                        );
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        target: "scripting",
                        "Failed to read metadata for {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }

        scripts
    }

    /// Get the scan interval
    pub fn scan_interval(&self) -> Duration {
        self.scan_interval
    }

    /// Set the scan interval
    pub fn set_scan_interval(&mut self, interval: Duration) {
        self.scan_interval = interval;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_scanner_creation() {
        let dir = PathBuf::from("/tmp/test");
        let scanner = ScriptScanner::new(dir.clone());

        assert_eq!(scanner.script_dir, dir);
        assert_eq!(scanner.scan_interval(), DEFAULT_SCAN_INTERVAL);
        assert!(scanner.should_scan());
    }

    #[test]
    fn test_custom_scan_interval() {
        let dir = PathBuf::from("/tmp/test");
        let interval = Duration::from_millis(1000);
        let scanner = ScriptScanner::with_interval(dir.clone(), interval);

        assert_eq!(scanner.scan_interval(), interval);
    }

    #[test]
    fn test_should_scan_timing() {
        let temp_dir = TempDir::new().unwrap();
        let mut scanner = ScriptScanner::new(temp_dir.path().to_path_buf());

        // Should scan initially
        assert!(scanner.should_scan());

        // Perform a scan
        scanner.scan_changes();

        // Should not scan again immediately
        assert!(!scanner.should_scan());

        // Wait for scan interval to elapse
        std::thread::sleep(DEFAULT_SCAN_INTERVAL);
        assert!(scanner.should_scan());
    }

    #[test]
    fn test_detect_new_script() {
        let temp_dir = TempDir::new().unwrap();
        let mut scanner = ScriptScanner::new(temp_dir.path().to_path_buf());

        // Initial scan (empty)
        let result = scanner.scan_changes();
        assert!(!result.has_changes());

        // Create a new script file
        let script_path = temp_dir.path().join("test.wasm");
        let mut file = File::create(&script_path).unwrap();
        file.write_all(b"fake wasm content").unwrap();

        // Scan again
        let result = scanner.scan_changes();
        assert_eq!(result.added.len(), 1);
        assert_eq!(result.added[0], script_path);
        assert!(result.changed.is_empty());
        assert!(result.removed.is_empty());
    }

    #[test]
    fn test_detect_modified_script() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.wasm");

        // Create initial file
        let mut file = File::create(&script_path).unwrap();
        file.write_all(b"initial content").unwrap();

        let mut scanner = ScriptScanner::new(temp_dir.path().to_path_buf());

        // Initial scan
        scanner.scan_changes();

        // Modify the file
        std::thread::sleep(Duration::from_millis(10)); // Ensure different timestamp
        let mut file = File::create(&script_path).unwrap();
        file.write_all(b"modified content").unwrap();

        // Scan again
        let result = scanner.scan_changes();
        assert_eq!(result.changed.len(), 1);
        assert_eq!(result.changed[0].0, script_path);
        assert!(result.added.is_empty());
        assert!(result.removed.is_empty());
    }

    #[test]
    fn test_detect_removed_script() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.wasm");

        // Create initial file
        File::create(&script_path).unwrap();

        let mut scanner = ScriptScanner::new(temp_dir.path().to_path_buf());

        // Initial scan
        scanner.scan_changes();

        // Remove the file
        fs::remove_file(&script_path).unwrap();

        // Scan again
        let result = scanner.scan_changes();
        assert_eq!(result.removed.len(), 1);
        assert_eq!(result.removed[0], script_path);
        assert!(result.changed.is_empty());
        assert!(result.added.is_empty());
    }

    #[test]
    fn test_ignores_non_wasm_files() {
        let temp_dir = TempDir::new().unwrap();
        let mut scanner = ScriptScanner::new(temp_dir.path().to_path_buf());

        // Create non-wasm files
        File::create(temp_dir.path().join("test.txt")).unwrap();
        File::create(temp_dir.path().join("test.rs")).unwrap();
        File::create(temp_dir.path().join("test.toml")).unwrap();

        // Scan - should not detect any files
        scanner.scan_changes();
        let result = scanner.scan_changes();

        assert!(!result.has_changes());
        assert_eq!(scanner.cached_state.len(), 0);
    }

    #[test]
    fn test_only_scans_wasm_files() {
        let temp_dir = TempDir::new().unwrap();
        let mut scanner = ScriptScanner::new(temp_dir.path().to_path_buf());

        // Create mixed files
        File::create(temp_dir.path().join("script1.wasm")).unwrap();
        File::create(temp_dir.path().join("script2.txt")).unwrap();
        File::create(temp_dir.path().join("script2.wasm")).unwrap();

        // Scan
        let result = scanner.scan_changes();

        // Should only detect .wasm files
        assert_eq!(result.added.len(), 2);
        assert!(
            scanner
                .cached_state
                .contains_key(&temp_dir.path().join("script1.wasm"))
        );
        assert!(
            scanner
                .cached_state
                .contains_key(&temp_dir.path().join("script2.wasm"))
        );
        assert!(
            !scanner
                .cached_state
                .contains_key(&temp_dir.path().join("script2.txt"))
        );
    }

    #[test]
    fn test_handles_missing_directory() {
        let missing_dir = PathBuf::from("/tmp/this_does_not_exist_12345");
        let mut scanner = ScriptScanner::new(missing_dir.clone());

        // Should not crash
        let result = scanner.scan_changes();
        assert!(!result.has_changes());
        assert!(scanner.cached_state.is_empty());
    }
}
