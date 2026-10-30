//! Phase 0 only: HERDR_SPIKE="<panes>:<mode>" runs the terminal spike unattended
//! and prints its numbers to stdout, then quits.

#[tauri::command]
pub fn spike_config() -> Option<String> {
    std::env::var("HERDR_SPIKE").ok().filter(|v| !v.is_empty())
}

#[tauri::command]
pub fn spike_report(app: tauri::AppHandle, line: String, done: bool) {
    println!("SPIKE {line}");
    if done {
        app.exit(0);
    }
}
