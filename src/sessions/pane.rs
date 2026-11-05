use gpui::EntityId;

/// A pane of a session's terminal grid, named by its `TerminalView`'s entity id.
pub type PaneId = EntityId;

/// Which way a split divides its two subtrees: side by side, or stacked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// `first` on the left, `second` on the right.
    LeftRight,
    /// `first` above, `second` below.
    TopBottom,
}

/// A session's panes: one terminal, or a split of two subtrees along an axis.
#[derive(Clone, Debug)]
pub enum PaneNode {
    /// A single terminal pane.
    Leaf(PaneId),
    /// Two subtrees divided by `axis`, `ratio` the first's share, from 0 to 1.
    Split {
        axis: Axis,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
        ratio: f32,
    },
}

impl PaneNode {
    /// One terminal pane.
    pub fn leaf(id: PaneId) -> Self {
        PaneNode::Leaf(id)
    }
}
