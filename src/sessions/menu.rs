use ely_gpui_component::{
    i18n::text,
    menus::{Menu, MenuItem},
    primitives::IconName,
};
use gpui::{App, Entity, EntityId};

use crate::{sessions::pane::Axis, workspace::Workspace};
/// What a right press on the row of session `id` offers.
pub fn row(workspace: &Entity<Workspace>, id: EntityId, cx: &App) -> Menu {
    let (duplicate, split_right, split_down, close_pane, close) = (
        workspace.clone(),
        workspace.clone(),
        workspace.clone(),
        workspace.clone(),
        workspace.clone(),
    );
    Menu::new()
        .item(
            MenuItem::new(text(cx, "et.session.duplicate", &[]))
                .icon(IconName::Copy)
                .on_click(move |window, cx| {
                    duplicate.update(cx, |this, cx| this.duplicate(id, window, cx))
                }),
        )
        .separator()
        .item(
            MenuItem::new(text(cx, "et.session.split_right", &[]))
                .icon(IconName::Columns2)
                .on_click(move |window, cx| {
                    split_right.update(cx, |this, cx| {
                        this.split_row(id, Axis::LeftRight, window, cx)
                    })
                }),
        )
        .item(
            MenuItem::new(text(cx, "et.session.split_down", &[]))
                .icon(IconName::Rows2)
                .on_click(move |window, cx| {
                    split_down.update(cx, |this, cx| {
                        this.split_row(id, Axis::TopBottom, window, cx)
                    })
                }),
        )
        .item(
            MenuItem::new(text(cx, "et.session.close_pane", &[]))
                .icon(IconName::X)
                .on_click(move |window, cx| {
                    close_pane.update(cx, |this, cx| this.close_row(id, window, cx))
                }),
        )
        .item(
            MenuItem::new(text(cx, "et.session.close", &[]))
                .icon(IconName::X)
                .on_click(move |window, cx| {
                    close.update(cx, |this, cx| {
                        let found = this
                            .sessions()
                            .iter()
                            .position(|session| session.terminal.entity_id() == id);
                        if let Some(index) = found {
                            this.close(index, window, cx);
                        }
                    })
                }),
        )
}
