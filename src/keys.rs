//! keys.json as GPUI key bindings. What the file means is
//! `herdr_shared::keys`; this binds it in the window.

pub use herdr_shared::keys::*;
use gpui::{App, KeyBinding, KeyBindingContextPredicate, NoAction};
use gpui_shell::action::ShellAction;
use std::rc::Rc;

/// Reads keys.json (writing the defaults if it's missing) into bindings, or says what's wrong.
pub fn load() -> Result<Vec<KeyBinding>, String> {
    parse(&read()?)
}

pub fn parse(text: &str) -> Result<Vec<KeyBinding>, String> {
    let (bindings, mut problems) = herdr_shared::keys::parse(text)?;
    let mut out = Vec::new();
    for b in bindings {
        let predicate = match KeyBindingContextPredicate::parse(&b.tab) {
            Ok(p) => Some(Rc::new(p)),
            Err(e) => {
                problems.push(format!("{}: {e}", b.section));
                continue;
            }
        };
        let action: Box<dyn gpui::Action> = match &b.bound {
            Bound::Unbound => Box::new(NoAction),
            Bound::Action(action) => {
                #[cfg(target_os = "macos")]
                let native = crate::macos::action(action);
                #[cfg(not(target_os = "macos"))]
                let native: Option<Box<dyn gpui::Action>> = None;
                native.unwrap_or_else(|| Box::new(ShellAction::new(action.clone())))
            }
        };
        match KeyBinding::load(&b.keys, action, predicate, false, None, &gpui::DummyKeyboardMapper) {
            Ok(k) => out.push(k),
            Err(e) => problems.push(format!("{}.{}: {e}", b.section, b.keys)),
        }
    }
    if problems.is_empty() { Ok(out) } else { Err(problems.join("; ")) }
}

/// The UI kit's own keys (text editing in inputs, lists): bound at its init,
/// kept across every reload of keys.json.
struct KitKeys(Vec<KeyBinding>);
impl gpui::Global for KitKeys {}

/// Restores kit bindings, installs user bindings, then adds reserved bindings.
pub fn install(bindings: Vec<KeyBinding>, cx: &mut App) {
    if !cx.has_global::<KitKeys>() {
        let kit = cx.key_bindings().borrow().bindings().cloned().collect();
        cx.set_global(KitKeys(kit));
    }
    let kit = cx.global::<KitKeys>().0.clone();
    cx.clear_key_bindings();
    cx.bind_keys(kit);
    cx.bind_keys(bindings);
    cx.bind_keys([
        KeyBinding::new(ESCAPE_HATCH, ShellAction::new("app.reset"), None),
        KeyBinding::new("escape", ShellAction::new("overlay.close"), Some("Overlay")),
    ]);
}

#[cfg(test)]
mod tests {
    #[test]
    fn shipped_keys_load() {
        for keys in [include_str!("../defaults/keys.json"), include_str!("../defaults/keys.macos.json")] {
            assert_eq!(super::parse(keys).err(), None);
        }
    }
}
