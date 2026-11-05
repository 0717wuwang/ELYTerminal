use gpui::{App, KeyBinding};

use super::*;

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
}
