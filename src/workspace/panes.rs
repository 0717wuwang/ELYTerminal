//! Draws a session's panes, drags the divider between two of them, and closes one.

use ely_gpui_component::layout::resize_handle;
use gpui::{
    AnyElement, App, AppContext, Context, DragMoveEvent, ElementId, EmptyView, Entity, Focusable,
    InteractiveElement, IntoElement, ParentElement, Pixels, StatefulInteractiveElement, Styled,
    Window, div,
};

use super::Workspace;
use crate::{
    sessions::pane::{Axis, Branch, PaneId, PaneNode},
    term::TerminalView,
};

/// The shown session's panes as the terminal column's body: one pane alone, a split as its tree.
pub fn shown_pane(this: &Workspace, workspace: &Entity<Workspace>, cx: &App) -> Option<AnyElement> {
    let session = this.sessions.get(this.active)?;
    let view = |id| session.views.get(&id).cloned();
    match &session.panes {
        PaneNode::Leaf(_) => view(session.focus).map(|view| view.into_any_element()),
        node @ PaneNode::Split { .. } => {
            Some(draw(node, &view, workspace, &mut Vec::new(), cx).into_any_element())
        }
    }
}

/// A pane's terminal, or a split's two subtrees with the divider a drag moves between them.
fn draw(
    node: &PaneNode,
    leaf: &impl Fn(PaneId) -> Option<Entity<TerminalView>>,
    workspace: &Entity<Workspace>,
    path: &mut Vec<Branch>,
    cx: &App,
) -> AnyElement {
    match node {
        PaneNode::Leaf(id) => match leaf(*id) {
            Some(view) => view.into_any_element(),
            None => {
                log::warn!("app: session pane {id} has no view");
                div().into_any_element()
            }
        },
        PaneNode::Split {
            axis,
            first,
            second,
            ratio,
        } => {
            let row = matches!(axis, Axis::LeftRight);
            // Ely names a divider by the way it divides: a horizontal one splits left from right.
            let handle = if row {
                gpui::Axis::Horizontal
            } else {
                gpui::Axis::Vertical
            };
            let dragged = workspace.clone();
            let mine = path.clone();
            let split = if row {
                div().flex().flex_row()
            } else {
                div().flex().flex_col()
            };
            let split = split
                .id(split_id(path, "pane-split"))
                .on_drag_move::<DividerDrag>(move |event: &DragMoveEvent<DividerDrag>, _, cx| {
                    let drag = event.drag(cx);
                    // Every split hears the drag; only the one whose divider it is moves.
                    if drag.path != mine {
                        return;
                    }
                    let (origin, along, extent) = match handle {
                        gpui::Axis::Horizontal => (
                            event.bounds.origin.x,
                            event.event.position.x,
                            event.bounds.size.width,
                        ),
                        gpui::Axis::Vertical => (
                            event.bounds.origin.y,
                            event.event.position.y,
                            event.bounds.size.height,
                        ),
                    };
                    if extent <= Pixels::ZERO {
                        log::warn!("app: a pane split draws no room to drag across");
                        return;
                    }
                    let ratio = f32::from(along - origin) / f32::from(extent);
                    let (path, axis) = (drag.path.clone(), drag.axis);
                    dragged.update(cx, |this, cx| this.drag_divider(&path, axis, ratio, cx));
                });
            let mut children = Vec::with_capacity(3);
            // A zero basis and a grow of each subtree's share; the divider takes its own grab width.
            for (branch, child, share) in [
                (Branch::First, first.as_ref(), *ratio),
                (Branch::Second, second.as_ref(), 1. - ratio),
            ] {
                if matches!(branch, Branch::Second) {
                    children.push(divider(cx, handle, path, *axis));
                }
                path.push(branch);
                let pane = draw(child, leaf, workspace, path, cx);
                path.pop();
                children.push(
                    div()
                        .flex_basis(Pixels::ZERO)
                        .flex_grow(share)
                        .self_stretch()
                        .child(pane)
                        .into_any_element(),
                );
            }
            split.children(children).into_any_element()
        }
    }
}

/// The hairline between a split's two subtrees, which a drag moves.
fn divider(cx: &App, split: gpui::Axis, path: &[Branch], axis: Axis) -> AnyElement {
    let drag = DividerDrag {
        path: path.to_vec(),
        axis,
    };
    resize_handle(split_id(path, "pane-divider"), split, cx)
        .on_drag(drag, |_, _, _, cx| cx.new(|_| EmptyView))
        .into_any_element()
}

/// A split's element id, or its divider's, from the way down the tree that names it.
fn split_id(path: &[Branch], kind: &str) -> ElementId {
    let mut key = String::from(kind);
    for branch in path {
        key.push(match branch {
            Branch::First => 'a',
            Branch::Second => 'b',
        });
    }
    ElementId::Name(key.into())
}

/// A drag on a divider: which split it divides, and which way that split runs.
struct DividerDrag {
    /// The split's place in the tree from the root.
    path: Vec<Branch>,
    axis: Axis,
}

impl Workspace {
    /// Moves the divider of the shown session's split at `path` to `ratio` of it, as a drag does.
    pub fn drag_divider(
        &mut self,
        path: &[Branch],
        axis: Axis,
        ratio: f32,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.sessions.get_mut(self.active) else {
            return;
        };
        if session.panes.resize(path, axis, ratio) {
            cx.notify();
        }
    }

    /// Closes pane `target` of session `index`, its row closing when it was the row's last pane; its program hangs up.
    pub fn close_pane(
        &mut self,
        index: usize,
        target: PaneId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let dropped = self
            .sessions
            .get_mut(index)
            .and_then(|session| session.drop_pane(target));
        let Some(view) = dropped else {
            self.close(index, window, cx);
            return;
        };
        log::info!("app: session {}'s pane {target} closes", index + 1);
        // Its textures leave the window's atlas with it.
        view.update(cx, |view, _| view.set_front(false, window));
        let shown = index == self.active;
        let focus = self.sessions[index].focused();
        if shown {
            window.focus(&focus.focus_handle(cx), cx);
        }
        cx.notify();
    }

    /// Closes the pane holding the keys of the session whose row was pressed.
    pub fn close_row(&mut self, id: PaneId, window: &mut Window, cx: &mut Context<Self>) {
        let found = self
            .sessions
            .iter()
            .position(|session| session.terminal.entity_id() == id);
        if let Some(index) = found {
            let target = self.sessions[index].focus;
            self.close_pane(index, target, window, cx);
        }
    }
}
