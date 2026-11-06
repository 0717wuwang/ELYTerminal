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

    /// The pane a key steps to from `id` toward `along`, or the far edge when nothing lies that way.
    pub fn neighbour(&self, id: PaneId, along: Axis, forward: bool) -> Option<PaneId> {
        let way = self.way(id)?;
        let mut node = self;
        let mut splits = Vec::with_capacity(way.len());
        for branch in way {
            let PaneNode::Split { first, second, .. } = node else {
                break;
            };
            splits.push((node, branch));
            node = match branch {
                Branch::First => first,
                Branch::Second => second,
            };
        }
        // The nearest split first: a pane steps to its sibling before crossing an outer split.
        for (split, branch) in splits.into_iter().rev() {
            let PaneNode::Split {
                axis,
                first,
                second,
                ..
            } = split
            else {
                continue;
            };
            if *axis != along || (branch == Branch::First) != forward {
                continue;
            }
            let side = if forward { second } else { first };
            // A key entering the pane from its own side lands on the edge nearest it.
            return Some(if forward {
                side.first_leaf()
            } else {
                side.last_leaf()
            });
        }
        self.round_about(id, forward)
    }

    /// The pane the keys run round to at the far edge: the whole tree's first leaf going forward, its last coming back.
    fn round_about(&self, id: PaneId, forward: bool) -> Option<PaneId> {
        let leaves = self.leaves();
        if leaves.len() < 2 {
            return None;
        }
        let far = if forward {
            self.first_leaf()
        } else {
            self.last_leaf()
        };
        if far != id {
            return Some(far);
        }
        let at = leaves.iter().position(|leaf| *leaf == id)?;
        let step = if forward { 1 } else { leaves.len() - 1 };
        Some(leaves[(at + step) % leaves.len()])
    }

    /// The branches from the root down to `id`, or nothing when it holds no such pane.
    fn way(&self, id: PaneId) -> Option<Vec<Branch>> {
        match self {
            PaneNode::Leaf(leaf) => (*leaf == id).then(Vec::new),
            PaneNode::Split { first, second, .. } => {
                let (branch, mut way) = match first.way(id) {
                    Some(way) => (Branch::First, way),
                    None => (Branch::Second, second.way(id)?),
                };
                way.insert(0, branch);
                Some(way)
            }
        }
    }

    /// The leaf drawn last: the rightmost, then the bottommost.
    fn last_leaf(&self) -> PaneId {
        match self {
            PaneNode::Leaf(id) => *id,
            PaneNode::Split { second, .. } => second.last_leaf(),
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
mod tests;
