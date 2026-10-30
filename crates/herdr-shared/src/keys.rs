//! ~/.config/herdr-app/keys.json: every key is a string action, reloaded live.
//!
//! ```json
//! {
//!   "global":   { "ctrl-shift-t": "tab.new", "ctrl-1": "tab.select:1" },
//!   "terminal": { "ctrl-left": "send:\u001bb", "ctrl-shift-t": "terminal" },
//!   "layout:terminals": { "ctrl-]": "space.next" },
//!   "mod:editor": { "ctrl-s": "editor.save" }
//! }
//! ```
//!
//! An action is a command (`pane.split:right`: the value fills its first
//! argument), `send:<text>` (types into the focused terminal),
//! `<mod>.<command>` (delivered to that mod's `nextCommand()`), or `terminal`
//! (the program gets the key: the binding is switched off where this section
//! applies).

use std::collections::BTreeMap;

/// The reserved shortcut that restores the default layout and keys.
pub const ESCAPE_HATCH: &str = "ctrl-alt-shift-escape";

/// The shipped keys: prefix-free; plain ctrl+letter always stays with the
/// program. Herdr uses ctrl+shift (defaults/keys.json), or cmd on macOS
/// (defaults/keys.macos.json, with Mac terminal editing keys).
#[cfg(not(target_os = "macos"))]
pub const DEFAULTS: &str = include_str!("../../../defaults/keys.json");
#[cfg(target_os = "macos")]
pub const DEFAULTS: &str = include_str!("../../../defaults/keys.macos.json");

fn defaults() -> BTreeMap<String, BTreeMap<String, String>> {
    serde_json::from_str(DEFAULTS).expect("defaults/keys.json is valid")
}

/// Every binding as `(section, keys, action)`, for the keybind help.
pub fn listing() -> Vec<(String, String, String)> {
    let text = std::fs::read_to_string(path()).unwrap_or_default();
    let sections: BTreeMap<String, BTreeMap<String, String>> = serde_json::from_str(&text).unwrap_or_else(|_| defaults());
    let mut out = Vec::new();
    for (s, keys) in sections {
        for (k, a) in keys {
            let a = current(&a);
            if !gone_mod(&a) {
                out.push((s.clone(), k, a));
            }
        }
    }
    out.push(("global".into(), ESCAPE_HATCH.into(), "app.reset".into()));
    out
}

pub fn path() -> std::path::PathBuf {
    crate::paths::app_dir().join("keys.json")
}

/// The key tab a section applies in. `global` is the window itself,
/// not "no tab": GPUI ranks a binding without one above every other, so
/// it would beat `layout:` (same depth, added earlier) and only lose to
/// `terminal` and `mod:` by order.
pub fn tab(section: &str) -> Result<String, String> {
    match section {
        "global" => Ok("Herdr".into()),
        "terminal" => Ok("Terminal".into()),
        s => match (s.strip_prefix("mod:"), s.strip_prefix("layout:")) {
            (Some(p), _) if !p.is_empty() => Ok(format!("mod == {p}")),
            // While layouts/<name>.json is shown (the window's tab says which).
            (_, Some(l)) if !l.is_empty() => Ok(format!("layout == {l}")),
            _ => Err(format!("unknown section {s:?}: use \"global\", \"terminal\", \"layout:<name>\" or \"mod:<name>\"")),
        },
    }
}

/// 0.1.0's words in a keys.json from then (installed once, so still theirs):
/// projects became spaces, contexts tabs, the start popup the new tab.
pub fn current(action: &str) -> String {
    if action == "mod.open:start" {
        return "tab.new".into();
    }
    let command = |a: &str| crate::commands::find(a.split_once(':').map_or(a, |(n, _)| n)).is_some();
    for (old, new) in [("project.", "space."), ("context.", "tab.")] {
        if let Some(rest) = action.strip_prefix(old) {
            let renamed = format!("{new}{rest}");
            // Only a command that was renamed: a mod named `project` keeps its own.
            if command(&renamed) && !command(action) {
                return renamed;
            }
        }
    }
    action.to_owned()
}

/// A mod's command whose mod is gone (0.1.x's `ribbon.next`): bound to nothing.
fn gone_mod(action: &str) -> bool {
    let name = action.split_once(':').map_or(action, |(n, _)| n);
    crate::commands::find(name).is_none()
        && name.split_once('.').is_some_and(|(m, _)| !m.starts_with("frame-") && !crate::paths::app_dir().join("mods").join(m).is_dir())
}

fn valid_action(action: &str) -> bool {
    let name = action.split_once(':').map_or(action, |(n, _)| n);
    crate::commands::find(name).is_some()
        || action == "terminal"
        || action.starts_with("send:")
        || name.split_once('.').is_some_and(|(m, c)| !m.is_empty() && !c.is_empty())
}

/// One keys.json entry, checked: the window binds `keys` to `bound` in `tab`.
#[derive(Clone, Debug, PartialEq)]
pub struct Binding {
    pub section: String,
    /// The key tab it applies in (`tab`): "Herdr", "Terminal", "mod == x", "layout == x".
    pub tab: String,
    /// As written: `ctrl-shift-t`, `cmd-]`, a space between chord steps.
    pub keys: String,
    pub bound: Bound,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Bound {
    /// A command, `send:<text>` or `<mod>.<command>`.
    Action(String),
    /// `terminal`: off where this section applies, so the program gets the key.
    Unbound,
}

/// Reads keys.json, writing the defaults if it's missing.
pub fn read() -> Result<String, String> {
    let p = path();
    if !p.exists() {
        let _ = std::fs::write(&p, DEFAULTS);
    }
    std::fs::read_to_string(&p).map_err(|e| e.to_string())
}

/// The bindings keys.json asks for, and what's wrong with the rest. Keystroke
/// syntax is the window's to check.
pub fn parse(text: &str) -> Result<(Vec<Binding>, Vec<String>), String> {
    let sections: BTreeMap<String, BTreeMap<String, String>> = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    let mut problems = Vec::new();
    for (section, keys) in &sections {
        let tab = match tab(section) {
            Ok(c) => c,
            Err(e) => {
                problems.push(e);
                continue;
            }
        };
        for (keys, action) in keys {
            let action = current(action);
            if gone_mod(&action) {
                continue;
            }
            if !valid_action(&action) {
                problems.push(format!("{section}.{keys}: unknown action {action:?}"));
                continue;
            }
            if keys == ESCAPE_HATCH {
                problems.push(format!("{ESCAPE_HATCH} is reserved for resetting Herdr"));
                continue;
            }
            let bound = if action == "terminal" {
                // Off where this section applies; globally, simply not bound.
                if section == "global" {
                    continue;
                }
                Bound::Unbound
            } else {
                Bound::Action(action)
            };
            out.push(Binding { section: section.clone(), tab: tab.clone(), keys: keys.clone(), bound });
        }
    }
    Ok((out, problems))
}

#[cfg(test)]
mod tests {
    #[test]
    fn keys_from_0_1_0_mean_the_same_now() {
        assert_eq!(super::current("project.select:2"), "space.select:2");
        assert_eq!(super::current("context.new"), "tab.new");
        assert_eq!(super::current("mod.open:start"), "tab.new");
        assert_eq!(super::current("pane.zoom"), "pane.zoom");
    }

    #[test]
    fn shipped_keys_parse() {
        for keys in [include_str!("../../../defaults/keys.json"), include_str!("../../../defaults/keys.macos.json")] {
            assert_eq!(super::parse(keys).map(|(_, problems)| problems), Ok(vec![]));
        }
    }
}
