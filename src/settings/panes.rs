//! The keys that split a pane and step between panes: what each setting holds, and what it takes when the file leaves it out.

use gpui::Keystroke;

use super::Settings;

/// A pane command whose key the settings keep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaneKey {
    SplitRight,
    SplitDown,
    ClosePane,
    FocusLeft,
    FocusRight,
    FocusUp,
    FocusDown,
}

impl PaneKey {
    /// Every command, in the order the settings page and the keymap take them.
    pub const ALL: [Self; 7] = [
        Self::SplitRight,
        Self::SplitDown,
        Self::ClosePane,
        Self::FocusLeft,
        Self::FocusRight,
        Self::FocusUp,
        Self::FocusDown,
    ];

    /// The place this command holds in [`PaneKey::ALL`], which names its row.
    pub fn place(self) -> usize {
        Self::ALL
            .iter()
            .position(|which| *which == self)
            .expect("every pane command is in ALL")
    }

    /// The setting's name, as a refusal names it.
    pub fn name(self) -> &'static str {
        match self {
            Self::SplitRight => "key_split_right",
            Self::SplitDown => "key_split_down",
            Self::ClosePane => "key_close_pane",
            Self::FocusLeft => "key_pane_left",
            Self::FocusRight => "key_pane_right",
            Self::FocusUp => "key_pane_up",
            Self::FocusDown => "key_pane_down",
        }
    }

    /// The key a fresh settings file takes; an empty field leaves the command unbound.
    pub fn unset(self) -> &'static str {
        match self {
            Self::SplitRight => "alt-\\",
            Self::SplitDown => "alt-/",
            Self::ClosePane => "alt-w",
            Self::FocusLeft => "alt-left",
            Self::FocusRight => "alt-right",
            Self::FocusUp => "alt-up",
            Self::FocusDown => "alt-down",
        }
    }

    /// The action this key runs, named as gpui names it.
    pub fn action(self) -> &'static str {
        match self {
            Self::SplitRight => "elyterminal::SplitRight",
            Self::SplitDown => "elyterminal::SplitDown",
            Self::ClosePane => "elyterminal::ClosePane",
            Self::FocusLeft => "elyterminal::FocusPaneLeft",
            Self::FocusRight => "elyterminal::FocusPaneRight",
            Self::FocusUp => "elyterminal::FocusPaneUp",
            Self::FocusDown => "elyterminal::FocusPaneDown",
        }
    }

    /// The key in force: the setting's, or the default while the file leaves the setting out; nothing once cleared.
    pub fn keystroke(self, settings: &Settings) -> Option<String> {
        match self.setting(settings) {
            Some(key) if key.is_empty() => None,
            Some(key) => Some(key.clone()),
            None => Some(self.unset().into()),
        }
    }

    /// Writes this command's key; an empty one leaves the command unbound.
    pub fn set(self, settings: &mut Settings, key: Option<String>) {
        // An empty key is written as one: `None` would take the default back.
        let key = key.map(|key| key.trim().to_string());
        match self {
            Self::SplitRight => settings.key_split_right = key,
            Self::SplitDown => settings.key_split_down = key,
            Self::ClosePane => settings.key_close_pane = key,
            Self::FocusLeft => settings.key_pane_left = key,
            Self::FocusRight => settings.key_pane_right = key,
            Self::FocusUp => settings.key_pane_up = key,
            Self::FocusDown => settings.key_pane_down = key,
        }
    }

    /// The setting as the file wrote it, before the default fills in.
    fn setting(self, settings: &Settings) -> &Option<String> {
        match self {
            Self::SplitRight => &settings.key_split_right,
            Self::SplitDown => &settings.key_split_down,
            Self::ClosePane => &settings.key_close_pane,
            Self::FocusLeft => &settings.key_pane_left,
            Self::FocusRight => &settings.key_pane_right,
            Self::FocusUp => &settings.key_pane_up,
            Self::FocusDown => &settings.key_pane_down,
        }
    }
}

/// Every key parses, and no two pane commands share one.
pub fn validate(settings: &Settings) -> anyhow::Result<()> {
    let mut seen: Vec<(String, PaneKey)> = Vec::with_capacity(PaneKey::ALL.len());
    for which in PaneKey::ALL {
        let Some(key) = which.keystroke(settings) else {
            continue;
        };
        let stroke = Keystroke::parse(&key)
            .map_err(|_| anyhow::anyhow!("{} is no keystroke ({key})", which.name()))?;
        let written = stroke.unparse();
        if let Some((_, other)) = seen.iter().find(|(held, _)| *held == written) {
            anyhow::bail!("{} and {} are both {written}", other.name(), which.name());
        }
        seen.push((written, which));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_file_takes_the_defaults() {
        let settings = Settings::default();
        assert_eq!(
            PaneKey::SplitRight.keystroke(&settings).as_deref(),
            Some("alt-\\")
        );
        assert_eq!(
            PaneKey::FocusDown.keystroke(&settings).as_deref(),
            Some("alt-down")
        );
        validate(&settings).expect("the defaults parse and differ");
    }

    #[test]
    fn a_cleared_key_binds_nothing() {
        let mut settings = Settings::default();
        PaneKey::ClosePane.set(&mut settings, Some(String::new()));
        assert_eq!(PaneKey::ClosePane.keystroke(&settings), None);
        validate(&settings).expect("a command with no key breaks nothing");
    }

    #[test]
    fn refuses_a_key_two_commands_hold() {
        let mut settings = Settings::default();
        PaneKey::SplitDown.set(&mut settings, Some("alt-\\".into()));
        let why = validate(&settings).expect_err("both hold alt-\\");
        assert!(why.to_string().contains("key_split_down"), "{why}");
    }

    #[test]
    fn a_recorded_key_stands_as_it_was_written() {
        let mut settings = Settings::default();
        PaneKey::SplitRight.set(&mut settings, Some("cmd-k".into()));
        assert_eq!(
            PaneKey::SplitRight.keystroke(&settings).as_deref(),
            Some("cmd-k")
        );
        validate(&settings).expect("cmd-k parses and no other command holds it");
    }

    #[test]
    fn every_command_has_a_place_of_its_own() {
        let mut seen: Vec<usize> = PaneKey::ALL.iter().map(|which| which.place()).collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..PaneKey::ALL.len()).collect::<Vec<_>>());
    }
}
