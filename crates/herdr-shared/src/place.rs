//! Where the window was: its size, its place on which display, and whether it
//! was maximized or full screen. Kept in `window.json` so the next start opens
//! it the same way.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// The first start's size.
pub const FIRST: (f32, f32) = (1280.0, 820.0);
/// Writes wait for a drag or resize to settle.
pub const SETTLE: Duration = Duration::from_millis(400);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Windowed,
    Maximized,
    Fullscreen,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub state: State,
    /// x, y, width, height of the window when not maximized (what un-maximizing
    /// goes back to), in the display's coordinates.
    pub bounds: [f32; 4],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

fn path() -> PathBuf {
    crate::paths::app_dir().join("window.json")
}

pub fn load() -> Option<Place> {
    std::fs::read_to_string(path()).ok().and_then(|s| serde_json::from_str(&s).ok())
}

pub fn write(p: &Place) {
    if let Ok(s) = serde_json::to_string_pretty(p) {
        let _ = crate::persist::write_atomic(&path(), s + "\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_json_keeps_its_format() {
        let p = Place { state: State::Maximized, bounds: [10.0, 20.0, 1280.0, 820.0], display: None };
        let text = serde_json::to_string(&p).unwrap();
        assert_eq!(text, r#"{"state":"maximized","bounds":[10.0,20.0,1280.0,820.0]}"#);
        assert_eq!(serde_json::from_str::<Place>(&text).unwrap(), p);
    }
}
