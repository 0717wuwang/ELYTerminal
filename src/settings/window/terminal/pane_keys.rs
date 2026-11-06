//! Settings › Terminal's pane keys: one recorder per pane command.

use ely_gpui_component::{
    forms::HotkeyInput,
    i18n::text,
    settings::{SettingsRow, SettingsSection},
};
use gpui::{App, Context, ElementId, Entity, Keystroke, SharedString};

use super::listed;
use crate::{
    settings::{PaneKey, SettingsStore, window::set},
    workspace,
};

use super::super::SettingsWindow;

/// The pane commands' keys, each recorded in its own row.
pub(super) fn section(
    this: &Entity<SettingsWindow>,
    cx: &Context<SettingsWindow>,
) -> SettingsSection {
    let mut section = SettingsSection::new(text(cx, "et.terminal.panes", &[])).description(text(
        cx,
        "et.terminal.panes_hint",
        &[],
    ));
    for which in PaneKey::ALL {
        section = section.row(row(this, which, cx));
    }
    section
}

/// One command's row: its name and a recorder holding the key in force.
fn row(this: &Entity<SettingsWindow>, which: PaneKey, cx: &Context<SettingsWindow>) -> SettingsRow {
    let held = which
        .keystroke(SettingsStore::get(cx))
        .and_then(|key| Keystroke::parse(&key).ok());
    let name = text(cx, name_key(which), &[]);
    let this = this.clone();
    let recorder = HotkeyInput::new(
        ElementId::Name(format!("pane-key-{}", which.name()).into()),
        held,
    )
    .on_change(move |stroke: Option<Keystroke>, _, cx| record(&this, which, stroke, cx));
    SettingsRow::new(name).control(recorder)
}

/// Saves the key recorded, or says why it is not saved.
fn record(this: &Entity<SettingsWindow>, which: PaneKey, stroke: Option<Keystroke>, cx: &mut App) {
    let key = stroke.map(|stroke| stroke.unparse()).unwrap_or_default();
    if let Some(holder) = holds(&key, which, cx) {
        let why = text(
            cx,
            "et.terminal.key_taken",
            &[("key", key.as_str()), ("name", holder.as_ref())],
        );
        log::warn!(
            "settings: {} keeps its key, {holder} holds {key:?}",
            which.name()
        );
        this.update(cx, |this, cx| {
            this.failed = Some(why);
            cx.notify();
        });
        return;
    }
    let change = format!("{} is {key:?}", which.name());
    let written = key.clone();
    if set(this, change, cx, |settings| {
        which.set(settings, Some(written))
    }) {
        workspace::apply_pane_keys(cx);
    }
}

/// The command already holding `key`, if the settings list one.
fn holds(key: &str, which: PaneKey, cx: &App) -> Option<SharedString> {
    // The command's own key is not taken: recording it again keeps it.
    if key.is_empty() {
        return None;
    }
    let keymap = cx.key_bindings();
    let keymap = keymap.borrow();
    listed(cx).into_iter().find_map(|(_, name, action)| {
        if action.name() == which.action() {
            return None;
        }
        let holds = keymap.bindings_for_action(action.as_ref()).any(|binding| {
            binding
                .keystrokes()
                .iter()
                .any(|stroke| stroke.unparse() == key)
        });
        holds.then_some(name)
    })
}

/// The interface's name for a pane command.
fn name_key(which: PaneKey) -> &'static str {
    match which {
        PaneKey::SplitRight => "et.pane.split_right",
        PaneKey::SplitDown => "et.pane.split_down",
        PaneKey::ClosePane => "et.pane.close",
        PaneKey::FocusLeft => "et.pane.focus_left",
        PaneKey::FocusRight => "et.pane.focus_right",
        PaneKey::FocusUp => "et.pane.focus_up",
        PaneKey::FocusDown => "et.pane.focus_down",
    }
}
