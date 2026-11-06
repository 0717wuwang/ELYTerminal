//! Draws a session's panes, drags the divider between two of them, and closes one.

use ely_gpui_component::{layout::resize_handle, theme::ActiveTheme as _};
use gpui::{
    AnyElement, App, AppContext as _, Context, Div, DragMoveEvent, ElementId, EmptyView, Entity,
    Focusable, Hsla, InteractiveElement, IntoElement, MouseDownEvent, ParentElement, Pixels,
    StatefulInteractiveElement, Styled, Window, div,
};

use super::{
    ClosePane, FocusPaneDown, FocusPaneLeft, FocusPaneRight, FocusPaneUp, SplitDown, SplitRight,
    Workspace,
};
use crate::sessions::{
    Session,
    pane::{Axis, Branch, PaneId, PaneNode},
};

/// The pane that holds the keys, and the colors its hairline and the dividers take.
#[derive(Clone, Copy)]
struct Lines {
    focus: PaneId,
    accent: Hsla,
}

/// The shown session's panes as the terminal column's body: one pane alone, a split as its tree.
pub fn shown_pane(workspace: &Workspace, cx: &mut Context<Workspace>) -> Option<AnyElement> {
    let index = workspace.active;
    let session = workspace.sessions.get(index)?;
    let lines = Lines {
        focus: session.focused_pane(),
        accent: cx.theme().colors.accent,
    };
    let this = cx.entity();
    match &session.panes {
        // One pane alone draws as it did before panes, without a frame or a press of its own.
        PaneNode::Leaf(id) => session
            .views
            .get(id)
            .map(|view| view.clone().into_any_element()),
        node @ PaneNode::Split { .. } => Some(draw(
            node,
            session,
            index,
            lines,
            &this,
            &mut Vec::new(),
            cx,
        )),
    }
}

/// A pane's terminal under the frame of the pane holding the keys, or a split's two subtrees with the divider a drag moves between them.
fn draw(
    node: &PaneNode,
    session: &Session,
    index: usize,
    lines: Lines,
    this: &Entity<Workspace>,
    path: &mut Vec<Branch>,
    cx: &mut Context<Workspace>,
) -> AnyElement {
    match node {
        PaneNode::Leaf(id) => leaf(session, *id, index, lines, cx),
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
            let drag_to = SplitDrag {
                dragged: this.clone(),
                mine: path.clone(),
                handle,
            };
            let axis = *axis;
            let split = if row {
                div().flex().flex_row().size_full()
            } else {
                div().flex().flex_col().size_full()
            };
            let split = split
                .id(split_id(path, "pane-split"))
                .on_drag_move::<DividerDrag>(
                    move |event: &DragMoveEvent<DividerDrag>, window, cx| {
                        drag_to.moved(event, window, cx)
                    },
                );
            let mut children = Vec::with_capacity(3);
            // A zero basis and a grow of each subtree's share; the divider takes its own grab width.
            for (branch, child, share) in [
                (Branch::First, first.as_ref(), *ratio),
                (Branch::Second, second.as_ref(), 1. - ratio),
            ] {
                if matches!(branch, Branch::Second) {
                    children.push(divider(cx, handle, path, axis));
                }
                path.push(branch);
                let pane = draw(child, session, index, lines, this, path, cx);
                path.pop();
                children.push(
                    div()
                        .flex_basis(Pixels::ZERO)
                        .flex_grow(share)
                        .self_stretch()
                        .min_w_0()
                        .min_h_0()
                        .child(pane)
                        .into_any_element(),
                );
            }
            split.children(children).into_any_element()
        }
    }
}

/// A pane's terminal, its frame when it holds the keys, and the press that gives it them.
fn leaf(
    session: &Session,
    id: PaneId,
    index: usize,
    lines: Lines,
    cx: &mut Context<Workspace>,
) -> AnyElement {
    let Some(view) = session.views.get(&id).cloned() else {
        log::warn!("app: session pane {id} has no view");
        return div().into_any_element();
    };
    let mut pane = div()
        .size_full()
        .on_any_mouse_down(cx.listener(move |this, _: &MouseDownEvent, window, cx| {
            this.focus_pane(index, id, window, cx)
        }))
        .child(view);
    if id == lines.focus {
        // Over the pane, so the grid keeps the cells it had.
        pane = pane.child(
            div()
                .absolute()
                .inset_0()
                .border_1()
                .border_color(lines.accent),
        );
    }
    pane.into_any_element()
}

/// The hairline between a split's two subtrees, which a drag moves.
fn divider(cx: &gpui::App, split: gpui::Axis, path: &[Branch], axis: Axis) -> AnyElement {
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

/// The listener a split keeps for its own divider's drag.
struct SplitDrag {
    /// The workspace a drag tells the new ratio to.
    dragged: Entity<Workspace>,
    /// The split's place in the tree from the root, which names the divider dragged.
    mine: Vec<Branch>,
    /// The way the divider divides, as the layout measures across it.
    handle: gpui::Axis,
}

impl SplitDrag {
    /// Moves this split's divider to the ratio the pointer stands at, the divider's own grab width left out.
    fn moved(&self, event: &DragMoveEvent<DividerDrag>, window: &mut Window, cx: &mut App) {
        let drag = event.drag(cx);
        // Every split hears the drag; only the one whose divider it is moves.
        if drag.path != self.mine {
            return;
        }
        let (origin, along, extent) = match self.handle {
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
        // The divider keeps its own grab width, so the panes share what lies past it.
        let panes = extent - cx.theme().handle_hit().to_pixels(window.rem_size());
        if panes <= Pixels::ZERO {
            log::warn!("app: a pane split draws no room to drag across");
            return;
        }
        let ratio = f32::from(along - origin) / f32::from(panes);
        let (path, axis) = (drag.path.clone(), drag.axis);
        self.dragged
            .update(cx, |this, cx| this.drag_divider(&path, axis, ratio, cx));
    }
}

impl Workspace {
    /// Gives pane `id` of session `index` the keys, showing that session first.
    pub fn focus_pane(
        &mut self,
        index: usize,
        id: PaneId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(view) = self
            .sessions
            .get(index)
            .and_then(|session| session.views.get(&id))
            .cloned()
        else {
            log::error!("app: session {index} has no pane {id}");
            return;
        };
        if index != self.active {
            self.select(index, window, cx);
        }
        self.sessions[index].focus = id;
        window.focus(&view.focus_handle(cx), cx);
        log::debug!("app: session {} focuses pane {id}", index + 1);
        cx.notify();
    }

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
    /// Moves the keys to the pane nearest the one holding them, if one lies that way.
    pub fn focus_side(
        &mut self,
        along: Axis,
        forward: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let index = self.active;
        let from = self.sessions[index].focused_pane();
        match self.sessions[index].panes.neighbour(from, along, forward) {
            Some(to) => self.focus_pane(index, to, window, cx),
            None => log::debug!(
                "app: session {} has no pane {along:?} of pane {from}",
                index + 1
            ),
        }
    }
}

/// The pane commands' handlers, on the content so their keys work wherever focus is.
pub fn pane_actions(div: Div, cx: &mut Context<Workspace>) -> Div {
    div.on_action(cx.listener(|this, _: &SplitRight, window, cx| {
        this.split_focused(Axis::LeftRight, window, cx)
    }))
    .on_action(cx.listener(|this, _: &SplitDown, window, cx| {
        this.split_focused(Axis::TopBottom, window, cx)
    }))
    .on_action(cx.listener(|this, _: &ClosePane, window, cx| {
        let (index, target) = (this.active, this.sessions[this.active].focus);
        this.close_pane(index, target, window, cx)
    }))
    .on_action(cx.listener(|this, _: &FocusPaneLeft, window, cx| {
        this.focus_side(Axis::LeftRight, false, window, cx)
    }))
    .on_action(cx.listener(|this, _: &FocusPaneRight, window, cx| {
        this.focus_side(Axis::LeftRight, true, window, cx)
    }))
    .on_action(cx.listener(|this, _: &FocusPaneUp, window, cx| {
        this.focus_side(Axis::TopBottom, false, window, cx)
    }))
    .on_action(cx.listener(|this, _: &FocusPaneDown, window, cx| {
        this.focus_side(Axis::TopBottom, true, window, cx)
    }))
}
