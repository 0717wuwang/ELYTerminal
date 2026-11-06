use ely_gpui_component::i18n::text;
use gpui::{AppContext, Context, PromptLevel, Window};

use crate::{
    sessions::pane::{Axis, PaneId},
    settings::SettingsStore,
    term::{TerminalView, config, spawn},
};

use super::{CELL, MIN_COLUMNS, MIN_LINES, Workspace};

impl Workspace {
    /// Splits the pane `target` of the session `index`, spawning a second program beside it. `axis` sets the divide.
    pub fn split(
        &mut self,
        index: usize,
        target: PaneId,
        axis: Axis,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(shortage) = self.split_shortage(index, axis, cx) {
            self.refuse_split(shortage, window, cx);
            return;
        }
        let session = &self.sessions[index];
        // The second pane is the same program where the target's own program stands, over the variables it started over.
        let mut launch = session.launch.clone();
        launch.cwd = session
            .folders
            .get(&target)
            .cloned()
            .unwrap_or_else(|| session.folder.clone());
        let (columns, lines) = session.focused().read(cx).grid();
        let (pty, events) = match spawn(
            &launch,
            (columns, lines),
            CELL,
            config(SettingsStore::get(cx)),
        ) {
            Ok(spawned) => spawned,
            Err(error) => {
                log::error!("app: session pane {target}'s split failed to start: {error:#}");
                return;
            }
        };
        let terminal = cx.new(|cx| TerminalView::new(pty, events, window, cx));
        let id = terminal.entity_id();
        let subscribed = cx.subscribe_in(&terminal, window, move |this, _, event, window, cx| {
            this.session_event(id, event, window, cx)
        });
        let session = &mut self.sessions[index];
        // A target that left the tree leaves no half-made pane behind.
        if !session.panes.split(target, id, axis) {
            log::error!("app: session pane {target} is gone, dropping its split pane {id}");
            return;
        }
        session.views.insert(id, terminal);
        session.register_pane(id, subscribed);
        session.focus = id;
        // The new program takes the keys as it starts.
        self.focus_pane(index, id, window, cx);
        log::info!("app: session pane {target} splits {axis:?} into pane {id}");
    }

    /// Splits the pane that holds the keys, as its row's menu does.
    pub fn split_focused(&mut self, axis: Axis, window: &mut Window, cx: &mut Context<Self>) {
        let index = self.active;
        let target = self.sessions[index].focused_pane();
        self.split(index, target, axis, window, cx);
    }

    /// Splits the pane of the session whose row was pressed, dividing the pane that holds the keys.
    pub fn split_row(
        &mut self,
        id: PaneId,
        axis: Axis,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(index) = self.session_index(id) {
            let target = self.sessions[index].focused_pane();
            self.split(index, target, axis, window, cx);
        }
    }

    /// The cells a split of session `index` would leave short, `None` when both panes fit.
    fn split_shortage(&self, index: usize, axis: Axis, cx: &Context<Self>) -> Option<Shortage> {
        Shortage::of(self.sessions[index].focused().read(cx).grid(), axis)
    }

    /// Tells the user why the split is refused, and logs it.
    fn refuse_split(&self, shortage: Shortage, window: &mut Window, cx: &mut Context<Self>) {
        let (across, needed) = (shortage.across.to_string(), shortage.needed.to_string());
        log::info!(
            "app: session pane holds {across} cells across, too few to split: need {needed}"
        );
        let values = [("across", across.as_str()), ("needed", needed.as_str())];
        let title = text(cx, "et.pane.too_small", &[]);
        let body = text(cx, "et.pane.too_small_body", &values);
        let close = text(cx, "et.files.dismiss", &[]);
        let answer = window.prompt(
            PromptLevel::Warning,
            &title,
            Some(&body),
            &[close.as_ref()],
            cx,
        );
        cx.spawn(async move |_, _| {
            let _ = answer.await;
        })
        .detach();
    }
}

/// How far a pane falls short of the two least grids a split needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Shortage {
    /// The cells the pane holds across the divide.
    across: usize,
    /// The cells two panes need there.
    needed: usize,
}

impl Shortage {
    /// The shortfall a split along `axis` would leave in `grid`'s cells, `None` when both panes fit the least size.
    fn of(grid: (usize, usize), axis: Axis) -> Option<Self> {
        let (across, each) = match axis {
            Axis::LeftRight => (grid.0, MIN_COLUMNS),
            Axis::TopBottom => (grid.1, MIN_LINES),
        };
        let needed = each * 2;
        (across < needed).then_some(Self { across, needed })
    }
}

#[cfg(test)]
mod tests;
