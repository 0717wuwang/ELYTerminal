use gpui::{AppContext, Context, Window};

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
        if self.too_narrow_to_split(index, axis, cx) {
            return;
        }
        let session = &self.sessions[index];
        // The second pane is the same program at the folder the first reports, over the variables it started over.
        let mut launch = session.launch.clone();
        launch.cwd = session.folder.clone();
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
        let _events = cx.subscribe_in(&terminal, window, move |this, _, event, window, cx| {
            this.session_event(id, event, window, cx)
        });
        let session = &mut self.sessions[index];
        session.views.insert(id, terminal);
        if !session.panes.split(target, id, axis) {
            log::error!("app: session pane {target} is gone, dropping its split pane {id}");
        }
        session.focus = id;
        cx.notify();
        log::info!("app: session pane {target} splits {axis:?} into pane {id}");
    }

    /// Splits the pane of the session whose row was pressed, dividing the pane that holds the keys.
    pub fn split_row(
        &mut self,
        id: PaneId,
        axis: Axis,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let found = self
            .sessions
            .iter()
            .position(|session| session.terminal.entity_id() == id);
        if let Some(index) = found {
            let target = self.sessions[index].focused_pane();
            self.split(index, target, axis, window, cx);
        }
    }

    /// True when the pane can't hold two grids across the divide, each pane drawing at least MIN_COLUMNS by MIN_LINES cells.
    fn too_narrow_to_split(&self, index: usize, axis: Axis, cx: &Context<Self>) -> bool {
        let (across, needed) = match axis {
            Axis::LeftRight => (
                self.sessions[index].focused().read(cx).grid().0,
                MIN_COLUMNS * 2,
            ),
            Axis::TopBottom => (
                self.sessions[index].focused().read(cx).grid().1,
                MIN_LINES * 2,
            ),
        };
        if across >= needed {
            return false;
        }
        log::info!(
            "app: session pane holds {across} {} cells, too few to split {}: need {needed}",
            match axis {
                Axis::LeftRight => "columns",
                Axis::TopBottom => "lines",
            },
            match axis {
                Axis::LeftRight => "left and right",
                Axis::TopBottom => "above and below",
            },
        );
        true
    }
}
