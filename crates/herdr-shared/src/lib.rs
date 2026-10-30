//! What the app does without a window: the core connection and the files it
//! keeps. Shared by the GPUI app and the Tauri app.

pub mod commands;
pub mod core_bin;
pub mod files;
pub mod hc;
pub mod layout;
pub mod layout_sync;
pub mod links;
pub mod mirror;
pub mod mirror_sync;
pub mod model;
pub mod paths;
pub mod persist;
pub mod process_cwd;
pub mod spawn;
pub mod win32_keys;
