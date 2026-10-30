//! Opening the window where it was, and noting where it goes: the GPUI side
//! of `window.json` (`herdr_shared::place`).

use std::cell::RefCell;

use gpui::{App, Bounds, DisplayId, Pixels, Task, Window, WindowBounds, point, px, size};
use herdr_shared::place::{FIRST, Place, SETTLE, State, load, write};

thread_local! {
    /// The place now, as last seen; written to disk once it settles.
    static NOW: RefCell<Option<Place>> = RefCell::new(load());
}

fn uuid(d: &dyn gpui::PlatformDisplay) -> Option<String> {
    d.uuid().ok().map(|u| u.to_string())
}

/// How to open the window: last time's place if its display is still there and
/// the window's corner is on it, else last time's size centered; maximized or
/// full screen as it was. Linux: always centered (Wayland doesn't let a window
/// place itself, and X11 window managers shift it by their frame each time).
pub fn initial(cx: &App) -> (WindowBounds, Option<DisplayId>) {
    let Some(p) = NOW.with(|n| n.borrow().clone()) else {
        return (WindowBounds::Windowed(Bounds::centered(None, size(px(FIRST.0), px(FIRST.1)), cx)), None);
    };
    let [x, y, w, h] = p.bounds;
    let (w, h) = (w.max(480.0), h.max(320.0));
    let display = p.display.as_deref().and_then(|want| cx.displays().into_iter().find(|d| uuid(d.as_ref()).as_deref() == Some(want)));
    let at = point(px(x), px(y));
    let bounds = match &display {
        Some(d) if !cfg!(target_os = "linux") && d.bounds().contains(&at) => Bounds::new(at, size(px(w), px(h))),
        _ => Bounds::centered(display.as_ref().map(|d| d.id()), size(px(w), px(h)), cx),
    };
    let wb = match p.state {
        State::Windowed => WindowBounds::Windowed(bounds),
        State::Maximized => WindowBounds::Maximized(bounds),
        State::Fullscreen => WindowBounds::Fullscreen(bounds),
    };
    (wb, display.map(|d| d.id()))
}

/// Notes where the window is now; the place to write if it changed.
fn note(window: &Window, cx: &App) -> Option<Place> {
    let (state, b): (State, Bounds<Pixels>) = match window.inner_window_bounds() {
        WindowBounds::Windowed(b) => (State::Windowed, b),
        WindowBounds::Maximized(b) => (State::Maximized, b),
        WindowBounds::Fullscreen(b) => (State::Fullscreen, b),
    };
    let display = window.display(cx).and_then(|d| uuid(d.as_ref()));
    NOW.with(|n| {
        let mut n = n.borrow_mut();
        // Maximized on X11 reports the maximized size: keep the windowed one.
        let bounds = match (state, n.as_ref()) {
            (State::Windowed, _) | (_, None) => [f32::from(b.origin.x), f32::from(b.origin.y), f32::from(b.size.width), f32::from(b.size.height)],
            (_, Some(prev)) => prev.bounds,
        };
        let place = Place { state, bounds, display };
        if n.as_ref() == Some(&place) || b.size.width <= px(0.0) {
            return None;
        }
        *n = Some(place.clone());
        Some(place)
    })
}

/// After the window moved or resized: written once it settles (the task
/// replaces the last one, which cancels it).
pub fn changed(window: &Window, cx: &App) -> Option<Task<()>> {
    let p = note(window, cx)?;
    let ex = cx.background_executor().clone();
    Some(cx.background_executor().spawn(async move {
        ex.timer(SETTLE).await;
        write(&p);
    }))
}

/// Writes it now (the window is closing).
pub fn save(window: &Window, cx: &App) {
    note(window, cx);
    if let Some(p) = NOW.with(|n| n.borrow().clone()) {
        write(&p);
    }
}
