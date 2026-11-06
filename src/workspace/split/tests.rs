use super::*;

fn shortfall(grid: (usize, usize), axis: Axis) -> Option<(usize, usize)> {
    Shortage::of(grid, axis).map(|shortage| (shortage.across, shortage.needed))
}

#[test]
fn a_pane_holding_two_least_grids_across_splits() {
    assert_eq!(shortfall((20, 16), Axis::LeftRight), None);
    assert_eq!(shortfall((20, 16), Axis::TopBottom), None);
    assert_eq!(shortfall((200, 200), Axis::LeftRight), None);
}

#[test]
fn a_short_side_names_what_two_panes_need() {
    assert_eq!(shortfall((19, 100), Axis::LeftRight), Some((19, 20)));
    assert_eq!(shortfall((100, 15), Axis::TopBottom), Some((15, 16)));
}

#[test]
fn each_axis_counts_its_own_side() {
    assert_eq!(shortfall((19, 30), Axis::LeftRight), Some((19, 20)));
    assert_eq!(shortfall((19, 30), Axis::TopBottom), None);
    assert_eq!(shortfall((30, 15), Axis::LeftRight), None);
    assert_eq!(shortfall((30, 15), Axis::TopBottom), Some((15, 16)));
}
