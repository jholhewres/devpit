//! Moving panes between trees, kept beside the moves.

use crate::session::{LayoutNode, SplitDirection};

fn leaf_ids(tree: &LayoutNode) -> Vec<&str> {
    tree.leaves().into_iter().map(|(id, _)| id).collect()
}

fn two() -> LayoutNode {
    LayoutNode::leaf("a", "s:a")
        .split_leaf(
            "a",
            SplitDirection::Horizontal,
            LayoutNode::leaf("b", "s:b"),
        )
        .expect("split a")
}

#[test]
fn joining_keeps_every_leaf_of_both_with_this_side_first() {
    let joined = two().joined(
        LayoutNode::leaf("c", "s:c"),
        SplitDirection::Vertical,
        "sp_join",
    );

    assert_eq!(leaf_ids(&joined), ["a", "b", "c"]);
    let LayoutNode::Split {
        id,
        direction,
        ratio,
        ..
    } = &joined
    else {
        panic!("a join is a split");
    };
    assert_eq!(id, "sp_join");
    assert_eq!(*direction, SplitDirection::Vertical);
    assert_eq!(*ratio, 0.5);
}

#[test]
fn a_joined_leaf_keeps_its_tmux_window() {
    /* The window is the process. A move that renamed the target would draw
    the pane and lose what runs in it. */
    let joined = LayoutNode::leaf("a", "s:a").joined(
        LayoutNode::leaf("c", "s:c"),
        SplitDirection::Horizontal,
        "sp",
    );
    assert!(joined.leaves().contains(&("c", "s:c")));
}

#[test]
fn detaching_answers_the_rest_and_the_leaf() {
    let (rest, leaf) = two().detached("b").expect("detach b");
    assert_eq!(leaf_ids(&rest), ["a"]);
    assert_eq!(leaf.leaves(), [("b", "s:b")]);
}

#[test]
fn detaching_deep_in_the_tree_collapses_only_its_split() {
    let tree = two()
        .split_leaf("b", SplitDirection::Vertical, LayoutNode::leaf("c", "s:c"))
        .expect("split b");
    let (rest, leaf) = tree.detached("c").expect("detach c");
    assert_eq!(leaf_ids(&rest), ["a", "b"]);
    assert!(matches!(rest, LayoutNode::Split { .. }));
    assert_eq!(leaf.leaves(), [("c", "s:c")]);
}

#[test]
fn the_last_leaf_or_a_stranger_cannot_be_detached() {
    assert!(LayoutNode::leaf("a", "s:a").detached("a").is_none());
    assert!(two().detached("zz").is_none());
}
