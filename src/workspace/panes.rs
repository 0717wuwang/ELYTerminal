//! Draws a session's pane tree: each leaf's terminal in its own share of the column, a hairline between them.

use ely_gpui_component::theme::ActiveTheme as _;
use gpui::{AnyElement, App, Entity, Hsla, IntoElement, ParentElement, Pixels, Styled, div};

use super::Workspace;
use crate::{
    sessions::pane::{Axis, PaneId, PaneNode},
    term::TerminalView,
};
/// The shown session's panes as the terminal column's body: one pane alone, a split as its tree.
pub fn shown_pane(workspace: &Workspace, cx: &App) -> Option<AnyElement> {
    let session = workspace.sessions.get(workspace.active)?;
    let border = cx.theme().colors.border;
    let view = |id| session.views.get(&id).cloned();
    match &session.panes {
        PaneNode::Leaf(_) => view(session.focus).map(|view| view.into_any_element()),
        node @ PaneNode::Split { .. } => Some(draw(node, &view, border).into_any_element()),
    }
}

fn draw(
    node: &PaneNode,
    leaf: &impl Fn(PaneId) -> Option<Entity<TerminalView>>,
    border: Hsla,
) -> AnyElement {
    match node {
        PaneNode::Leaf(id) => match leaf(*id) {
            Some(view) => view.into_any_element(),
            None => div().into_any_element(),
        },
        PaneNode::Split {
            axis,
            first,
            second,
            ratio,
        } => {
            let row = matches!(axis, Axis::LeftRight);
            // A zero basis and a grow of each subtree's share; the second carries the divider's pixel.
            let pane = |child: &PaneNode, edge: bool| {
                let share = if edge { 1. - ratio } else { *ratio };
                let mut pane = div()
                    .flex_basis(Pixels::ZERO)
                    .flex_grow(share)
                    .self_stretch()
                    .border_color(border);
                if edge {
                    pane = match axis {
                        Axis::LeftRight => pane.border_l_1(),
                        Axis::TopBottom => pane.border_t_1(),
                    };
                }
                let pane = pane.child(draw(child, leaf, border));
                pane.into_any_element()
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
