use std::path::PathBuf;

/// Platform-specific path handling that follows XDG Base Directory spec on Unix-like systems
/// and Windows conventions on Windows.
///
/// On macOS and Linux:
/// - Config: $XDG_CONFIG_HOME/{name} (default: ~/.config/{name})
/// - Data: $XDG_DATA_HOME/{name} (default: ~/.local/share/{name})
/// - Cache: $XDG_CACHE_HOME/{name} (default: ~/.cache/{name})
///
/// On Windows:
/// - Config: %APPDATA%\{name}
/// - Data: %APPDATA%\{name}
/// - Cache: %LOCALAPPDATA%\{name}
pub struct ProjectPaths {
    name: String,
}

impl ProjectPaths {
    /// Create a new ProjectPaths instance for the given application name.
    pub fn new(name: &str) -> Option<Self> {
        // Verify we can get home directory before creating
        get_home_dir()?;
        Some(ProjectPaths {
            name: name.to_string(),
        })
    }

    /// Get the configuration directory path.
    pub fn config_dir(&self) -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            get_windows_appdata()
                .map(|p| p.join(&self.name))
                .unwrap_or_else(|| PathBuf::from(format!(".{}", self.name)))
        }

        #[cfg(not(target_os = "windows"))]
        {
            get_xdg_config_dir(&self.name)
        }
    }

    /// Get the data directory path.
    pub fn data_dir(&self) -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            get_windows_appdata()
                .map(|p| p.join(&self.name))
                .unwrap_or_else(|| PathBuf::from(format!(".{}", self.name)))
        }

        #[cfg(not(target_os = "windows"))]
        {
            get_xdg_data_dir(&self.name)
        }
    }

    /// Get the cache directory path.
    pub fn cache_dir(&self) -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            get_windows_localappdata()
                .map(|p| p.join(&self.name))
                .unwrap_or_else(|| PathBuf::from(format!(".{}", self.name)))
        }

        #[cfg(not(target_os = "windows"))]
        {
            get_xdg_cache_dir(&self.name)
        }
    }
}

/// Get the home directory, respecting HOME and USERPROFILE environment variables.
fn get_home_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
}

/// Get XDG config directory (respects $XDG_CONFIG_HOME, defaults to ~/.config).
fn get_xdg_config_dir(name: &str) -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| get_home_dir().map(|h| h.join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join(name)
}

/// Get XDG data directory (respects $XDG_DATA_HOME, defaults to ~/.local/share).
fn get_xdg_data_dir(name: &str) -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| get_home_dir().map(|h| h.join(".local").join("share")))
        .unwrap_or_else(|| PathBuf::from(".local/share"))
        .join(name)
}

/// Get XDG cache directory (respects $XDG_CACHE_HOME, defaults to ~/.cache).
fn get_xdg_cache_dir(name: &str) -> PathBuf {
    std::env::var("XDG_CACHE_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| get_home_dir().map(|h| h.join(".cache")))
        .unwrap_or_else(|| PathBuf::from(".cache"))
        .join(name)
}

/// Get Windows APPDATA directory.
#[cfg(target_os = "windows")]
fn get_windows_appdata() -> Option<PathBuf> {
    std::env::var("APPDATA").ok().map(PathBuf::from)
}

/// Get Windows LOCALAPPDATA directory.
#[cfg(target_os = "windows")]
fn get_windows_localappdata() -> Option<PathBuf> {
    std::env::var("LOCALAPPDATA").ok().map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_paths_creation() {
        let paths = ProjectPaths::new("gromnie");
        assert!(paths.is_some());
    }

    #[test]
    fn test_config_dir_contains_name() {
        if let Some(paths) = ProjectPaths::new("gromnie") {
            let config_dir = paths.config_dir();
            assert!(config_dir.to_string_lossy().contains("gromnie"));
        }
    }

    #[test]
    fn test_data_dir_contains_name() {
        if let Some(paths) = ProjectPaths::new("gromnie") {
            let data_dir = paths.data_dir();
            assert!(data_dir.to_string_lossy().contains("gromnie"));
        }
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_xdg_config_dir_uses_config_path() {
        let config_dir = get_xdg_config_dir("test");
        let config_str = config_dir.to_string_lossy();
        assert!(config_str.contains(".config") || config_str.contains("XDG_CONFIG_HOME"));
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_xdg_data_dir_uses_local_share_path() {
        let data_dir = get_xdg_data_dir("test");
        let data_str = data_dir.to_string_lossy();
        assert!(
            data_str.contains(".local/share")
                || data_str.contains(".local\\share")
                || data_str.contains("XDG_DATA_HOME")
        );
    }
}
