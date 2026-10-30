//! Terminals for the webview: each one is a core terminal attached on its
//! own channel. Bytes go to the page raw over a Tauri channel; the page
//! grants credit once ghostty-web has parsed them, as the GPUI app does.

use herdr_shared::{core_bin, hc};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::State;
use tauri::ipc::{Channel, InvokeResponseBody};

#[derive(Default)]
pub struct Terminals {
    core: Mutex<Option<Arc<hc::Core>>>,
    /// The terminal each attached channel shows: closing the pane ends it.
    started: Mutex<HashMap<u32, String>>,
}

impl Terminals {
    fn core(&self) -> Result<Arc<hc::Core>, String> {
        let mut core = self.core.lock().unwrap();
        if let Some(c) = core.as_ref() {
            return Ok(c.clone());
        }
        let bin = core_bin::find(None).ok_or("herdr-core isn't installed")?;
        core_bin::use_core(bin);
        let c = hc::Core::connect().map_err(|e| format!("{e:#}"))?;
        hc::check(&c.hello).map_err(|m| format!("this core can't serve the app: {m:?}"))?;
        *core = Some(c.clone());
        Ok(c)
    }
}

impl Terminals {
    /// Ends every program this app started (the window is going away).
    pub fn kill_all(&self) {
        let Some(core) = self.core.lock().unwrap().clone() else { return };
        for (_, address) in self.started.lock().unwrap().drain() {
            let _ = core.call("terminal.kill", json!({ "terminal": address }));
        }
    }
}

#[derive(Serialize)]
pub struct Opened {
    channel: u32,
    terminal: Value,
}

/// Why a terminal's stream ended, for the page.
#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Ended {
    Closed { reason: Value },
    Lost,
}

/// Starts `argv` (the login shell by default) in a new core terminal and attaches it.
#[tauri::command]
pub fn term_open(
    state: State<'_, Terminals>,
    argv: Option<Vec<String>>,
    cols: u16,
    rows: u16,
    on_data: Channel<InvokeResponseBody>,
    on_end: Channel<Ended>,
) -> Result<Opened, String> {
    let core = state.core()?;
    let argv = argv.unwrap_or_else(|| vec![std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into()), "-l".into()]);
    // Kept like the GPUI app's terminals (a terminal started with keep: false
    // stalled with several panes open: herdr-core 0.1.14); closing kills it.
    let address = core.run(argv, None, json!({}), None).map_err(|e| format!("{e:#}"))?;
    let attached = core.attach(&address, cols, rows, json!({})).map_err(|e| format!("{e:#}"))?;
    state.started.lock().unwrap().insert(attached.channel, address);
    let stream = attached.stream;
    std::thread::spawn(move || forward(stream, on_data, on_end));
    Ok(Opened { channel: attached.channel, terminal: attached.terminal })
}

/// The core sends small chunks (a few hundred bytes from each PTY read); one
/// channel message per chunk costs far more than parsing it. Batch what
/// arrives within 2 ms, up to 256 KiB, like Orca's delivery batching.
const BATCH_WINDOW: Duration = Duration::from_millis(2);
const BATCH_MAX: usize = 256 * 1024;

fn forward(stream: async_channel::Receiver<hc::Stream>, on_data: Channel<InvokeResponseBody>, on_end: Channel<Ended>) {
    let end = |item: hc::Stream| match item {
        hc::Stream::Closed(reason) => {
            let _ = on_end.send(Ended::Closed { reason });
        }
        hc::Stream::Lost => {
            let _ = on_end.send(Ended::Lost);
        }
        hc::Stream::Data(_) => {}
    };
    while let Ok(first) = stream.recv_blocking() {
        let hc::Stream::Data(mut batch) = first else { return end(first) };
        let deadline = Instant::now() + BATCH_WINDOW;
        let mut ended = None;
        while batch.len() < BATCH_MAX {
            match stream.try_recv() {
                Ok(hc::Stream::Data(more)) => batch.extend_from_slice(&more),
                Ok(other) => {
                    ended = Some(other);
                    break;
                }
                Err(async_channel::TryRecvError::Closed) => break,
                Err(async_channel::TryRecvError::Empty) => {
                    if Instant::now() >= deadline {
                        break;
                    }
                    std::thread::sleep(Duration::from_micros(200));
                }
            }
        }
        if on_data.send(InvokeResponseBody::Raw(batch)).is_err() {
            return;
        }
        if let Some(item) = ended {
            return end(item);
        }
    }
}

#[tauri::command]
pub fn term_input(state: State<'_, Terminals>, channel: u32, data: String) -> Result<(), String> {
    state.core()?.input(channel, data.as_bytes());
    Ok(())
}

#[tauri::command]
pub fn term_credit(state: State<'_, Terminals>, channel: u32, n: usize) -> Result<(), String> {
    state.core()?.credit(channel, n);
    Ok(())
}

#[tauri::command]
pub fn term_resize(state: State<'_, Terminals>, channel: u32, cols: u16, rows: u16) -> Result<(), String> {
    state.core()?.resize(channel, cols, rows);
    Ok(())
}

#[tauri::command]
pub fn term_focus(state: State<'_, Terminals>, channel: u32, focused: bool) -> Result<(), String> {
    state.core()?.focus(channel, focused);
    Ok(())
}

#[tauri::command]
pub fn term_close(state: State<'_, Terminals>, channel: u32) -> Result<(), String> {
    let core = state.core()?;
    core.detach(channel);
    // The spike starts its own programs; none should outlive its pane.
    if let Some(address) = state.started.lock().unwrap().remove(&channel) {
        let _ = core.call("terminal.kill", json!({ "terminal": address }));
    }
    Ok(())
}
