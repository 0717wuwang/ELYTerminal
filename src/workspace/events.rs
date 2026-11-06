use gpui::{Context, EntityId, Window};

use super::Workspace;
use crate::term::{State, TerminalEvent};

impl Workspace {
    /// What session `id`'s terminal reported.
    pub(super) fn session_event(
        &mut self,
        id: EntityId,
        event: &TerminalEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.session_index(id) else {
            log::debug!("app: a pane that went reported");
            return;
        };
        // The row follows every pane; only the first names the session, moves it or ends it.
        let first = self.sessions[index].terminal.entity_id() == id;
        let session = &mut self.sessions[index];
        match event {
            TerminalEvent::Title(title) if first => {
                log::debug!("app: session {}'s program names it {title:?}", index + 1);
                session.title.clone_from(title);
            }
            TerminalEvent::Folder(folder) => {
                session.folders.insert(id, folder.clone());
                if first {
                    session.folder.clone_from(folder);
                    session.reports = true;
                    if index == self.active {
                        self.files.update(cx, |files, cx| files.show(folder, cx));
                        self.git.update(cx, |git, cx| git.show(folder, cx));
                    }
                }
            }
            TerminalEvent::Bell if index != self.active => {
                if !session.bell {
                    log::debug!("app: a bell in hidden session {}", index + 1);
                }
                session.bell = true;
            }
            TerminalEvent::Bell => {}
            // A hidden session that starts waiting marks its row until it is shown.
            TerminalEvent::Shows(Some(State::Blocked)) if index != self.active => {
                log::debug!("app: hidden session {} waits", index + 1);
                session.waiting = true;
            }
            TerminalEvent::Shows(_) | TerminalEvent::Ended(_) => {}
            TerminalEvent::Dragged(key) if first => {
                if index != self.active {
                    return;
                }
                match self.files.read(cx).path(key) {
                    Some(path) => {
                        log::info!("app: {} dragged onto the terminal", path.display());
                        session.views[&id].update(cx, |view, cx| view.type_paths(&[path], cx));
                    }
                    None => log::warn!("app: a tree drag of {key} types nothing"),
                }
            }
            TerminalEvent::Exited(status) => {
                if session.panes.leaves().len() > 1 {
                    log::info!("app: session {}'s pane {id} ended, {status}", index + 1);
                    self.close_pane(index, id, window, cx);
                } else {
                    let pid = session.focused().read(cx).pid();
                    log::info!("app: session {pid}'s program ended, {status}");
                    self.close(index, window, cx);
                }
                return;
            }
            // A pane beside the first keeps these to itself; the row still draws again below.
            TerminalEvent::Title(_) | TerminalEvent::Dragged(_) => {
                log::debug!("app: session {}'s pane {id} reports alone", index + 1);
            }
        }
        if index == self.active {
            self.retitle(window);
        }
        cx.notify();
    }

    /// A press on notification `tag`: its session hears of it and shows, unless it asked not to; false when no session here posted it.
    pub fn pressed(&mut self, tag: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let found = self
            .sessions
            .iter()
            .enumerate()
            .find_map(|(index, session)| {
                // Any pane of the session may have posted it.
                session
                    .panes
                    .leaves()
                    .into_iter()
                    .find_map(|id| {
                        session
                            .views
                            .get(&id)?
                            .update(cx, |view, _| view.pressed(tag))
                    })
                    .map(|focus| (index, focus))
            });
        let Some((index, focus)) = found else {
            return false;
        };
        if focus {
            cx.activate(true);
            window.activate_window();
            self.select(index, window, cx);
        }
        true
    }
}
