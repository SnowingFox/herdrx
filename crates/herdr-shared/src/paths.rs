//! Where the app keeps things.

use std::path::PathBuf;

pub fn home() -> PathBuf {
    // HOME, or the profile folder on Windows.
    std::env::home_dir().unwrap_or_else(|| "/".into())
}

/// `~/.config/herdr-app`, or `$HERDR_APP_HOME`.
pub fn app_dir() -> PathBuf {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    env("HERDR_APP_HOME").unwrap_or_else(|| env("XDG_CONFIG_HOME").unwrap_or_else(|| home().join(".config")).join("herdr-app"))
}
