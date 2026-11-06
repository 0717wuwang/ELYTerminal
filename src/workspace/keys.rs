use gpui::{App, KeyBinding};

use super::*;
use crate::settings::{PaneKey, SettingsStore};

/// Ghostty's keys: Cmd on macOS; off it, Ctrl-Shift for keys a terminal takes, Alt for digits.
pub fn bind_keys(cx: &mut App) {
    let (sessions, panel, new, close, digit) = if cfg!(target_os = "macos") {
        ("cmd-b", "cmd-alt-b", "cmd-t", "cmd-w", "cmd")
    } else {
        (
            "ctrl-shift-b",
            "ctrl-shift-alt-b",
            "ctrl-shift-t",
            "ctrl-shift-w",
            "alt",
        )
    };
    let context = Some(CONTEXT);
    cx.bind_keys([
        KeyBinding::new(sessions, ToggleSessions, context),
        KeyBinding::new(panel, TogglePanel, context),
        KeyBinding::new(new, NewSession, context),
        KeyBinding::new(close, CloseSession, context),
        KeyBinding::new("ctrl-tab", NextSession, context),
        KeyBinding::new("ctrl-shift-tab", PreviousSession, context),
        KeyBinding::new(&format!("{digit}-9"), LastSession, context),
    ]);
    cx.bind_keys(
        (1..=8).map(|n| KeyBinding::new(&format!("{digit}-{n}"), SelectSession(n - 1), context)),
    );
    apply_pane_keys(cx);
}

/// Binds the pane commands to the keys the settings keep, the keys they held before going.
pub fn apply_pane_keys(cx: &mut App) {
    let settings = SettingsStore::get(cx).clone();
    let mut bindings = Vec::with_capacity(PaneKey::ALL.len());
    for which in PaneKey::ALL {
        let Some(key) = which.keystroke(&settings) else {
            continue;
        };
        bindings.push(match which {
            PaneKey::SplitRight => KeyBinding::new(&key, SplitRight, Some(CONTEXT)),
            PaneKey::SplitDown => KeyBinding::new(&key, SplitDown, Some(CONTEXT)),
            PaneKey::ClosePane => KeyBinding::new(&key, ClosePane, Some(CONTEXT)),
            PaneKey::FocusLeft => KeyBinding::new(&key, FocusPaneLeft, Some(CONTEXT)),
            PaneKey::FocusRight => KeyBinding::new(&key, FocusPaneRight, Some(CONTEXT)),
            PaneKey::FocusUp => KeyBinding::new(&key, FocusPaneUp, Some(CONTEXT)),
            PaneKey::FocusDown => KeyBinding::new(&key, FocusPaneDown, Some(CONTEXT)),
        });
    }
    // Every binding but the pane commands', which the keys just made replace: gpui takes no one binding back.
    let kept: Vec<KeyBinding> = cx
        .key_bindings()
        .borrow()
        .bindings()
        .filter(|binding| !pane_action(binding.action().name()))
        .cloned()
        .collect();
    cx.clear_key_bindings();
    cx.bind_keys(kept);
    log::info!("app: {} pane keys bound", bindings.len());
    cx.bind_keys(bindings);
}

/// True for a command whose key this rebinding owns.
fn pane_action(name: &str) -> bool {
    [
        SplitRight.name(),
        SplitDown.name(),
        ClosePane.name(),
        FocusPaneLeft.name(),
        FocusPaneRight.name(),
        FocusPaneUp.name(),
        FocusPaneDown.name(),
    ]
    .contains(&name)
}
