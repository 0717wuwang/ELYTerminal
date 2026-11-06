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
fn d() -> EntityId {
    4u64.into()
}

/// The walk reported: A | A1 beside B | B1, where the key must reach B before A1.
#[test]
fn the_keys_step_pane_by_pane_and_round_at_the_edge() {
    let mut root = PaneNode::leaf(a());
    assert!(root.split(a(), b(), Axis::LeftRight));
    assert!(root.split(b(), c(), Axis::LeftRight));
    assert!(root.split(a(), d(), Axis::LeftRight));
    assert_eq!(root.leaves(), vec![a(), d(), b(), c()]);
    // Left: B1 (c) reaches B (b) in its own column first, then A1 (d), then A (a).
    assert_eq!(root.neighbour(c(), Axis::LeftRight, false), Some(b()));
    assert_eq!(root.neighbour(b(), Axis::LeftRight, false), Some(d()));
    assert_eq!(root.neighbour(d(), Axis::LeftRight, false), Some(a()));
    // A lies at the left edge, so its key runs round to the last pane.
    assert_eq!(root.neighbour(a(), Axis::LeftRight, false), Some(c()));
    // Right walks the same way back, and round at the right edge.
    assert_eq!(root.neighbour(a(), Axis::LeftRight, true), Some(d()));
    assert_eq!(root.neighbour(c(), Axis::LeftRight, true), Some(a()));
}

#[test]
fn steps_across_a_split() {
    let mut root = PaneNode::leaf(a());
    assert!(root.split(a(), b(), Axis::LeftRight));
    assert_eq!(root.neighbour(a(), Axis::LeftRight, true), Some(b()));
    assert_eq!(root.neighbour(b(), Axis::LeftRight, false), Some(a()));
    // Nothing lies to the left of the first pane, so the key runs round to the last one.
    assert_eq!(root.neighbour(a(), Axis::LeftRight, false), Some(b()));
    assert_eq!(root.neighbour(b(), Axis::LeftRight, true), Some(a()));
}

#[test]
fn steps_into_a_nested_split() {
    // a on the left; c on the right with b below it.
    let mut root = PaneNode::leaf(a());
    assert!(root.split(a(), c(), Axis::LeftRight));
    assert!(root.split(c(), b(), Axis::TopBottom));
    assert_eq!(root.leaves(), vec![a(), c(), b()]);
    // The key enters the pane it meets on the edge nearest the one it left.
    assert_eq!(root.neighbour(a(), Axis::LeftRight, true), Some(c()));
    assert_eq!(root.neighbour(c(), Axis::LeftRight, false), Some(a()));
    // a stands as tall as the column, so the key runs round: past the top to the pane it meets.
    assert_eq!(root.neighbour(a(), Axis::TopBottom, true), Some(c()));
    assert_eq!(root.neighbour(c(), Axis::TopBottom, true), Some(b()));
    assert_eq!(root.neighbour(b(), Axis::TopBottom, false), Some(c()));
}

#[test]
fn a_pane_that_is_gone_has_no_neighbour() {
    let root = PaneNode::leaf(a());
    assert_eq!(root.neighbour(b(), Axis::LeftRight, true), None);
    assert_eq!(root.neighbour(b(), Axis::TopBottom, false), None);
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
