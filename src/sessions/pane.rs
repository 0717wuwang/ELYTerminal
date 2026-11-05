use gpui::{Bounds, EntityId, Pixels, point, size};

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
#[derive(Clone, Debug, PartialEq)]
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

    /// Splits the leaf `target` in place into it and a new leaf `added`, divided by `axis`; false when no leaf is `target`.
    pub fn split(&mut self, target: PaneId, added: PaneId, axis: Axis) -> bool {
        if !self.leaves().contains(&target) {
            return false;
        }
        match self {
            PaneNode::Leaf(_) => {
                *self = PaneNode::Split {
                    axis,
                    first: Box::new(PaneNode::Leaf(target)),
                    second: Box::new(PaneNode::Leaf(added)),
                    ratio: 0.5,
                };
                true
            }
            PaneNode::Split { first, second, .. } => {
                first.split(target, added, axis) || second.split(target, added, axis)
            }
        }
    }

    /// Every leaf, in layout order: `first` before `second` at each split.
    pub fn leaves(&self) -> Vec<PaneId> {
        match self {
            PaneNode::Leaf(id) => vec![*id],
            PaneNode::Split { first, second, .. } => {
                let mut leaves = first.leaves();
                leaves.extend(second.leaves());
                leaves
            }
        }
    }


    /// Removes the leaf `target`, its parent split giving way to its sibling; false when no leaf is `target`.
    pub fn remove(&mut self, target: PaneId) -> bool {
        let sibling = match self {
            PaneNode::Leaf(_) => return false,
            PaneNode::Split { first, second, .. } => {
                if first.remove(target) || second.remove(target) {
                    return true;
                }
                match (&**first, &**second) {
                    (PaneNode::Leaf(id), _) if *id == target => second,
                    (_, PaneNode::Leaf(id)) if *id == target => first,
                    _ => return false,
                }
            }
        };
        *self = sibling.as_ref().clone();
        true
    }

    /// Projects the tree into `available`, cutting each split by its `ratio` and leaving a `divider` wide between the two subtrees.
    pub fn layout(&self, available: Bounds<Pixels>, divider: Pixels) -> Layout {
        let mut layout = Layout::default();
        self.place(available, divider, &mut layout);
        layout
    }

    fn place(&self, available: Bounds<Pixels>, divider: Pixels, layout: &mut Layout) {
        match self {
            PaneNode::Leaf(id) => layout.panes.push((*id, available)),
            PaneNode::Split {
                axis,
                first,
                second,
                ratio,
            } => {
                let (along, extent) = match axis {
                    Axis::LeftRight => (true, available.size.width),
                    Axis::TopBottom => (false, available.size.height),
                };
                let head = (extent - divider).max(Pixels::ZERO) * ratio.clamp(0., 1.);
                let cross = if along {
                    available.size.height
                } else {
                    available.size.width
                };
                let tail = (extent - head - divider).max(Pixels::ZERO);
                let (head_size, divider_size, tail_size) = if along {
                    (size(head, cross), size(divider, cross), size(tail, cross))
                } else {
                    (size(cross, head), size(cross, divider), size(cross, tail))
                };
                let at = |offset: Pixels| {
                    if along {
                        available.origin + point(offset, Pixels::ZERO)
                    } else {
                        available.origin + point(Pixels::ZERO, offset)
                    }
                };
                first.place(Bounds::new(available.origin, head_size), divider, layout);
                if divider > Pixels::ZERO {
                    layout.dividers.push(Bounds::new(at(head), divider_size));
                }
                second.place(Bounds::new(at(head + divider), tail_size), divider, layout);
            }
        }
    }
}

/// A tree laid out in a box: where each pane draws, and the separators between them.
///
/// The render goes through gpui flex (`workspace::panes`); this absolute projection serves
/// a drag's hit-testing of which pane a point lands in.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    /// Each leaf's rectangle, in layout order.
    pub panes: Vec<(PaneId, Bounds<Pixels>)>,
    /// One rectangle per split, between its two subtrees.
    pub dividers: Vec<Bounds<Pixels>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a() -> EntityId {
        1u64.into()
    }
    fn b() -> EntityId {
        2u64.into()
    }
    fn c() -> EntityId {
        3u64.into()
    }
    /// A box at `x`,`y` of `w` by `h`.
    fn bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(w), px(h)))
    }
    fn px(v: f32) -> Pixels {
        gpui::px(v)
    }
    /// Splits the root leaf; the added pane becomes the second child.
    #[test]
    fn splits_root_leaf_right() {
        let mut root = PaneNode::leaf(a());
        assert!(root.split(a(), b(), Axis::LeftRight));
        assert_eq!(
            root,
            PaneNode::Split {
                axis: Axis::LeftRight,
                first: Box::new(PaneNode::leaf(a())),
                second: Box::new(PaneNode::leaf(b())),
                ratio: 0.5,
            }
        );
        assert_eq!(root.leaves(), vec![a(), b()]);
    }

    /// A nested split inserts the new pane after its sibling leaf.
    #[test]
    fn splits_nested_leaf_down() {
        let mut root = PaneNode::leaf(a());
        root.split(a(), b(), Axis::LeftRight);
        assert!(root.split(b(), c(), Axis::TopBottom));
        assert_eq!(root.leaves(), vec![a(), b(), c()]);
        // The b()/c() split is the second child's right subtree.
        let PaneNode::Split { second, .. } = &root else {
            panic!("root not a split");
        };
        assert!(matches!(
            second.as_ref(),
            PaneNode::Split {
                axis: Axis::TopBottom,
                ..
            }
        ));
    }

    /// A nested leaf splits in place, inserting the new pane beside it.
    #[test]
    fn splits_a_nested_leaf() {
        let mut root = PaneNode::leaf(a());
        root.split(a(), b(), Axis::LeftRight);
        assert!(root.split(a(), c(), Axis::TopBottom));
        assert_eq!(root.leaves(), vec![a(), c(), b()]);
    }

    /// Removing the first child lets the second take the parent's place.
    #[test]
    fn removing_first_leaf_promotes_second() {
        let mut root = PaneNode::leaf(a());
        root.split(a(), b(), Axis::LeftRight);
        assert!(root.remove(a()));
        assert_eq!(root, PaneNode::leaf(b()));
    }

    /// Removing the only leaf is refused: the last pane never leaves by tree surgery.
    #[test]
    fn refuses_removing_the_only_leaf() {
        let mut root = PaneNode::leaf(a());
        assert!(!root.remove(a()));
    }

    /// A leaf missing from the tree removes nothing.
    #[test]
    fn refuses_removing_a_missing_leaf() {
        let mut root = PaneNode::leaf(a());
        root.split(a(), b(), Axis::LeftRight);
        assert!(!root.remove(c()));
    }

    /// A 200-wide box split in two at half leaves two 99-wide panes and a 2-wide divider between them.
    #[test]
    fn lays_out_a_split_side_by_side() {
        let mut root = PaneNode::leaf(a());
        root.split(a(), b(), Axis::LeftRight);
        let layout = root.layout(bounds(0., 0., 200., 100.), px(2.));
        assert_eq!(
            layout.panes,
            vec![
                (a(), bounds(0., 0., 99., 100.)),
                (b(), bounds(101., 0., 99., 100.))
            ]
        );
        assert_eq!(layout.dividers, vec![bounds(99., 0., 2., 100.)]);
    }

    /// A stacked split cuts the height instead, the divider between the two.
    #[test]
    fn lays_out_a_split_stacked() {
        let mut root = PaneNode::leaf(a());
        root.split(a(), b(), Axis::TopBottom);
        let layout = root.layout(bounds(0., 0., 100., 202.), px(2.));
        assert_eq!(
            layout.panes,
            vec![
                (a(), bounds(0., 0., 100., 100.)),
                (b(), bounds(0., 102., 100., 100.))
            ]
        );
        assert_eq!(layout.dividers, vec![bounds(0., 100., 100., 2.)]);
    }

    /// A split of a split places every leaf within the box, nested cuts inside their subtree's share.
    #[test]
    fn lays_out_a_split_of_a_split() {
        let mut root = PaneNode::leaf(a());
        root.split(a(), b(), Axis::LeftRight);
        root.split(b(), c(), Axis::TopBottom);
        let layout = root.layout(bounds(0., 0., 202., 100.), px(2.));
        // a: left half whole; b: right half's top; c: right half's bottom.
        assert_eq!(layout.panes[0], (a(), bounds(0., 0., 100., 100.)));
        assert_eq!(layout.panes[1], (b(), bounds(102., 0., 100., 49.)));
        assert_eq!(layout.panes[2], (c(), bounds(102., 51., 100., 49.)));
        // One divider per split: the outer between a and the right half, the inner between b and c.
        assert_eq!(layout.dividers.len(), 2);
    }

    /// A zero divider leaves no separator rectangle and the two panes touching.
    #[test]
    fn lays_out_without_a_divider() {
        let mut root = PaneNode::leaf(a());
        root.split(a(), b(), Axis::LeftRight);
        let layout = root.layout(bounds(0., 0., 200., 100.), px(0.));
        assert!(layout.dividers.is_empty());
        assert_eq!(
            layout.panes,
            vec![
                (a(), bounds(0., 0., 100., 100.)),
                (b(), bounds(100., 0., 100., 100.))
            ]
        );
    }

    /// A box too small for even the divider clamps both panes to nothing rather than going negative.
    #[test]
    fn clamps_a_box_smaller_than_the_divider() {
        let mut root = PaneNode::leaf(a());
        root.split(a(), b(), Axis::LeftRight);
        let layout = root.layout(bounds(0., 0., 1., 100.), px(2.));
        assert_eq!(layout.panes[0].1.size.width, px(0.));
        assert_eq!(layout.panes[1].1.size.width, px(0.));
    }

    /// One pane alone fills the box whole.
    #[test]
    fn lays_out_a_single_pane_whole() {
        let root = PaneNode::leaf(a());
        let layout = root.layout(bounds(0., 0., 200., 100.), px(2.));
        assert_eq!(layout.panes, vec![(a(), bounds(0., 0., 200., 100.))]);
        assert!(layout.dividers.is_empty());
    }
}
