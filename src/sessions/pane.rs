//! A session's panes: one terminal, or a split of two subtrees a drag divides.

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

/// A step from a split down into one of its subtrees, naming where a split stands in the tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Branch {
    /// The subtree drawn first.
    First,
    /// The subtree drawn second.
    Second,
}

/// The least share of a split a drag leaves either side, so neither collapses and the divider stays grabbable.
pub const MIN_RATIO: f32 = 0.1;

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

    /// Moves the divider of the split at `path` to `ratio` of it, keeping each side at least `MIN_RATIO`; false when `path` names no such split of `axis`.
    pub fn resize(&mut self, path: &[Branch], axis: Axis, ratio: f32) -> bool {
        let PaneNode::Split {
            axis: split,
            first,
            second,
            ratio: share,
        } = self
        else {
            return false;
        };
        match path.split_first() {
            None if *split != axis || !ratio.is_finite() => false,
            None => {
                *share = ratio.clamp(MIN_RATIO, 1. - MIN_RATIO);
                true
            }
            Some((Branch::First, rest)) => first.resize(rest, axis, ratio),
            Some((Branch::Second, rest)) => second.resize(rest, axis, ratio),
        }
    }

    /// The tree and the pane its keys go to once `closed` goes, its parent split giving way to its sibling, focus falling back to the first pane; `None` while it is the only leaf, whose row closes instead.
    pub fn without(&self, closed: PaneId, focus: PaneId) -> Option<(PaneNode, PaneId)> {
        if self.leaves().len() <= 1 {
            return None;
        }
        let mut tree = self.clone();
        if !tree.remove(closed) {
            return None;
        }
        let first = *tree.leaves().first().expect("a pane tree keeps a leaf");
        let focus = if focus == closed { first } else { focus };
        Some((tree, focus))
    }

    /// The leaf `id` names, else the first leaf, so a focus that outlived its pane still names one.
    pub fn focus_of(&self, id: PaneId) -> PaneId {
        if self.holds(id) {
            id
        } else {
            self.first_leaf()
        }
    }

    /// Whether `id` is one of its leaves.
    fn holds(&self, id: PaneId) -> bool {
        match self {
            PaneNode::Leaf(leaf) => *leaf == id,
            PaneNode::Split { first, second, .. } => first.holds(id) || second.holds(id),
        }
    }

    /// The leaf drawn first: the leftmost, then the topmost.
    fn first_leaf(&self) -> PaneId {
        match self {
            PaneNode::Leaf(id) => *id,
            PaneNode::Split { first, .. } => first.first_leaf(),
        }
    }
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

    /// Two panes side by side, each at half.
    fn side_by_side() -> PaneNode {
        let mut root = PaneNode::leaf(a());
        root.split(a(), b(), Axis::LeftRight);
        root
    }

    /// `a` beside a stacked `b` over `c`, every divider at half.
    fn stacked_beside() -> PaneNode {
        let mut root = side_by_side();
        root.split(b(), c(), Axis::TopBottom);
        root
    }

    /// Every divider's place in the tree, the root's first.
    fn ratios(node: &PaneNode) -> Vec<f32> {
        match node {
            PaneNode::Leaf(_) => Vec::new(),
            PaneNode::Split {
                first,
                second,
                ratio,
                ..
            } => {
                let mut all = vec![*ratio];
                all.extend(ratios(first));
                all.extend(ratios(second));
                all
            }
        }
    }

    /// A drag puts the divider where the pointer stands, and no side goes under `MIN_RATIO`.
    #[test]
    fn a_drag_moves_a_divider_within_its_bounds() {
        let mut root = side_by_side();
        assert!(root.resize(&[], Axis::LeftRight, 0.75));
        assert_eq!(ratios(&root), vec![0.75]);
        assert!(root.resize(&[], Axis::LeftRight, 1.5));
        assert_eq!(ratios(&root), vec![1. - MIN_RATIO]);
        assert!(root.resize(&[], Axis::LeftRight, -0.5));
        assert_eq!(ratios(&root), vec![MIN_RATIO]);
    }

    /// A drag names its split by its way down the tree, so only that split moves.
    #[test]
    fn a_drag_reaches_a_nested_split() {
        let mut root = stacked_beside();
        assert!(root.resize(&[Branch::Second], Axis::TopBottom, 0.3));
        assert_eq!(ratios(&root), vec![0.5, 0.3]);
        assert!(!root.resize(&[Branch::Second], Axis::LeftRight, 0.2));
        assert!(!root.resize(&[Branch::First], Axis::TopBottom, 0.2));
        assert!(!root.resize(&[Branch::Second, Branch::First], Axis::TopBottom, 0.2));
        assert!(!root.resize(&[], Axis::LeftRight, f32::NAN));
        assert_eq!(
            ratios(&root),
            vec![0.5, 0.3],
            "a refused drag moves nothing"
        );
    }

    /// A closed pane lets its sibling take its parent's place, and the keys fall back to a pane still there.
    #[test]
    fn closing_a_pane_promotes_its_sibling() {
        let root = side_by_side();
        let (tree, focus) = root.without(a(), a()).expect("a pane is left");
        assert_eq!(tree, PaneNode::leaf(b()));
        assert_eq!(focus, b(), "the only pane left takes the keys");
        let (tree, focus) = root.without(b(), a()).expect("a pane is left");
        assert_eq!(tree, PaneNode::leaf(a()));
        assert_eq!(focus, a(), "a pane that kept the keys keeps them");
    }

    /// A pane closed beside the focused one leaves the keys where they were.
    #[test]
    fn closing_another_pane_keeps_the_keys() {
        let root = stacked_beside();
        let (tree, focus) = root.without(c(), b()).expect("panes are left");
        assert_eq!(tree.leaves(), vec![a(), b()]);
        assert_eq!(focus, b());
    }

    /// A closed pane deep in the tree gives its room to its sibling, the keys falling back to the tree's first pane.
    #[test]
    fn closing_a_focused_pane_falls_back_to_the_first() {
        let root = stacked_beside();
        let (tree, focus) = root.without(b(), b()).expect("panes are left");
        assert_eq!(tree.leaves(), vec![a(), c()]);
        assert_eq!(focus, a(), "the tree's first pane takes the keys");
    }

    /// The only pane never leaves its tree: its program's exit closes the row instead.
    #[test]
    fn the_last_pane_never_leaves_its_tree() {
        let root = PaneNode::leaf(a());
        assert_eq!(root.without(a(), a()), None);
    }

    /// The focused leaf keeps the keys while it stands.
    #[test]
    fn focus_stays_on_a_leaf_that_stands() {
        let mut root = PaneNode::leaf(a());
        root.split(a(), b(), Axis::LeftRight);
        assert_eq!(root.focus_of(b()), b());
    }

    /// A focus whose leaf went takes the first leaf, so the keys land in a pane that stands.
    #[test]
    fn focus_falls_back_to_the_first_leaf() {
        let mut root = PaneNode::leaf(a());
        root.split(a(), b(), Axis::LeftRight);
        assert!(root.remove(a()));
        assert_eq!(root.focus_of(a()), b());
    }

    /// The first leaf of a split tree is its leftmost, however deep the split that holds it.
    #[test]
    fn focus_falls_back_to_the_leftmost_leaf() {
        let mut root = PaneNode::leaf(a());
        root.split(a(), b(), Axis::LeftRight);
        root.split(b(), c(), Axis::TopBottom);
        assert_eq!(root.focus_of(c()), c());
        assert!(root.remove(c()));
        assert_eq!(root.focus_of(c()), a());
    }

    /// One pane alone is every focus, even one naming a pane it never held.
    #[test]
    fn one_pane_is_the_focus_of_any_id() {
        let root = PaneNode::leaf(a());
        assert_eq!(root.focus_of(b()), a());
    }
}
