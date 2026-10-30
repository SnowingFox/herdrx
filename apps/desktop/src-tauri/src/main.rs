// Windows: a window app, no console of its own.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod spike;
mod term;

fn main() {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    tauri::Builder::default()
        .manage(term::Terminals::default())
        .invoke_handler(tauri::generate_handler![term::term_open, term::term_input, term::term_credit, term::term_resize, term::term_focus, term::term_close, spike::spike_config, spike::spike_report])
        .run(tauri::generate_context!())
        .expect("error while running Herdr");
}
