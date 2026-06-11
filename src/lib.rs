pub mod models;
pub mod pipeline;
pub mod sources;
pub mod textifier;
pub mod analyzer;
pub mod presentation;

// ── Shared utilities ────────────────────────────────────────────

/// Step number symbols for display (①-⑩)
pub const STEP_NUMS: [&str; 10] = ["①", "②", "③", "④", "⑤", "⑥", "⑦", "⑧", "⑨", "⑩"];

/// Find an executable in PATH.
pub fn which(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.exists() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}

/// Get the user's home directory, falling back to /tmp.
pub fn home_dir() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| "/tmp".into())
}
