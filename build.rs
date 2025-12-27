use std::process::Command;

fn main() {
    let version = env!("CARGO_PKG_VERSION");

    // Check if release mode
    let profile = std::env::var("PROFILE").unwrap_or_default();
    let is_release = profile == "release";

    let version_str = if is_release {
        version.to_string()
    } else {
        // Get git hash
        let git_hash = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout).ok()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "unknown".to_string())
            .trim()
            .to_string();

        // Check if git workdir is dirty
        let is_dirty = Command::new("git")
            .args(["diff-index", "--quiet", "HEAD"])
            .output()
            .ok()
            .map(|output| !output.status.success())
            .unwrap_or(false);

        if is_dirty {
            format!("{}-dirty", git_hash)
        } else {
            git_hash
        }
    };

    println!("cargo:rustc-env=VERSION_STRING={}", version_str);
}
