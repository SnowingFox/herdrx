//! Settings as the GPUI app paints them: the shared settings
//! (`herdr_shared::settings`) plus colors, sizes and fonts in GPUI's types.

pub use herdr_shared::settings::*;
use gpui::{Hsla, Pixels, Rgba, px};
use std::path::PathBuf;
use std::sync::Mutex;

pub fn parse_hex(s: &str) -> Option<Hsla> {
    let (r, g, b, a) = parse_rgba(s)?;
    Some(Rgba { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0, a: a as f32 / 255.0 }.into())
}

/// A theme color as GPUI paint (magenta if missing, so mistakes show).
pub fn color(name: &str) -> Hsla {
    theme().get(name).and_then(|v| parse_hex(v)).unwrap_or(gpui::hsla(0.83, 1.0, 0.5, 1.0))
}

/// A theme color name, or a literal `#rrggbb[aa]`.
pub fn color_or_literal(v: &str) -> Hsla {
    parse_hex(v).unwrap_or_else(|| color(v))
}

/// Installs new settings; returns whether anything changed.
pub fn set(s: Settings) -> bool {
    #[cfg(target_os = "macos")]
    gpui_macos::set_option_as_alt(match s.option_as_alt {
        OptionAsAlt::Auto => None,
        OptionAsAlt::Off => Some(false),
        OptionAsAlt::On => Some(true),
    });
    let changed = herdr_shared::settings::set(s);
    if changed {
        gpui_shell::set_pixel_scale(zoom());
    }
    changed
}

/// A designed size in px, at the current zoom.
pub fn z(v: f32) -> Pixels {
    px(v * zoom())
}

/// Font files the home brings: `fonts/` and each mod's `fonts/` (a layout's
/// look travels with its mod, as Herdr 98's does). Loads the ones not loaded
/// yet; true when any was.
pub fn load_home_fonts(cx: &mut gpui::App) -> bool {
    static LOADED: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
    let home = crate::mods::app_dir();
    let mut dirs = vec![home.join("fonts")];
    dirs.extend(std::fs::read_dir(home.join("mods")).into_iter().flatten().flatten().map(|e| e.path().join("fonts")));
    let mut new = Vec::new();
    let mut loaded = LOADED.lock().unwrap();
    for dir in dirs {
        for path in std::fs::read_dir(&dir).into_iter().flatten().flatten().map(|e| e.path()) {
            let font = path.extension().and_then(|e| e.to_str()).is_some_and(|e| matches!(e.to_lowercase().as_str(), "ttf" | "otf" | "ttc"));
            if font && !loaded.contains(&path) && let Ok(bytes) = std::fs::read(&path) {
                new.push(std::borrow::Cow::Owned(bytes));
                loaded.push(path);
            }
        }
    }
    if new.is_empty() {
        return false;
    }
    if let Err(e) = cx.text_system().add_fonts(new) {
        tracing::warn!("fonts in {} not loaded: {e:#}", home.display());
    }
    note_fonts_added();
    note_fonts(cx.text_system().all_font_names());
    true
}

/// Fonts Herdr ships, so the defaults look the same everywhere.
pub const BUNDLED_FONTS: &[&[u8]] = &[
    include_bytes!("../assets/fonts/GeistMono-Variable.ttf"),
    include_bytes!("../assets/fonts/GeistMono-Italic-Variable.ttf"),
    include_bytes!("../assets/fonts/Geist.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-Variable.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono-Italic-Variable.ttf"),
    SYMBOLS_FONT_FILE,
];

/// The symbols font itself: terminals read icons' glyphs from it directly.
pub const SYMBOLS_FONT_FILE: &[u8] = include_bytes!("../assets/fonts/SymbolsNerdFont-Regular.ttf");
