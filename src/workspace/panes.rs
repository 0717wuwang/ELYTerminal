//! Draws a session's pane tree: each leaf's terminal in its own share of the column, a hairline between them.

use ely_gpui_component::theme::ActiveTheme as _;
use gpui::{
    AnyElement, Context, Focusable, Hsla, InteractiveElement, IntoElement, MouseDownEvent,
    ParentElement, Pixels, Styled, Window, div,
};

use super::Workspace;
use crate::sessions::{
    Session,
    pane::{Axis, PaneId, PaneNode},
};

/// The pane that holds the keys, and the colors its hairline and the dividers take.
#[derive(Clone, Copy)]
struct Lines {
    focus: PaneId,
    accent: Hsla,
    border: Hsla,
}

/// The shown session's panes as the terminal column's body: one pane alone, a split as its tree.
pub fn shown_pane(workspace: &Workspace, cx: &mut Context<Workspace>) -> Option<AnyElement> {
    let index = workspace.active;
    let session = workspace.sessions.get(index)?;
    let lines = Lines {
        focus: session.focused_pane(),
        accent: cx.theme().colors.accent,
        border: cx.theme().colors.border,
    };
    Some(draw(&session.panes, session, index, lines, cx))
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
}

fn draw(
    node: &PaneNode,
    session: &Session,
    index: usize,
    lines: Lines,
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
            // A zero basis and a grow of each subtree's share; the second carries the divider's pixel.
            let mut pane = |child: &PaneNode, edge: bool| {
                let share = if edge { 1. - ratio } else { *ratio };
                let mut pane = div()
                    .flex_basis(Pixels::ZERO)
                    .flex_grow(share)
                    .self_stretch()
                    .border_color(lines.border);
                if edge {
                    pane = match axis {
                        Axis::LeftRight => pane.border_l_1(),
                        Axis::TopBottom => pane.border_t_1(),
                    };
                }
                pane.child(draw(child, session, index, lines, cx))
            };
            let split = if row {
                div().flex().flex_row()
            } else {
                div().flex().flex_col()
            };
            split
                .child(pane(first, false))
                .child(pane(second, true))
                .into_any_element()
        }
    }
}

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
