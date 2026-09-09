//! The tree's tests, kept beside it.

use super::*;
use crate::session::LayoutNode;

/// `a` split against `b`, then `b` split against `c`, with named boundaries.
fn nested() -> LayoutNode {
    let mut tree = LayoutNode::leaf("a", "s:a")
        .split_leaf(
            "a",
            SplitDirection::Horizontal,
            LayoutNode::leaf("b", "s:b"),
        )
        .expect("split a")
        .split_leaf("b", SplitDirection::Vertical, LayoutNode::leaf("c", "s:c"))
        .expect("split b");
    let mut next = 0;
    tree.name_the_splits(&mut || {
        next += 1;
        format!("sp_{next}")
    });
    tree
}

fn leaf_ids(tree: &LayoutNode) -> Vec<&str> {
    tree.leaves().into_iter().map(|(id, _)| id).collect()
}

#[test]
fn a_fresh_tree_is_one_leaf() {
    let tree = LayoutNode::leaf("leaf_a", "sess:leaf_a");
    assert_eq!(tree.leaves(), vec![("leaf_a", "sess:leaf_a")]);
    assert_eq!(tree.first_leaf_id(), "leaf_a");
}

#[test]
fn split_leaf_puts_the_new_pane_on_the_second_side() {
    let tree = LayoutNode::leaf("leaf_a", "sess:leaf_a");
    let split = tree
        .split_leaf(
            "leaf_a",
            SplitDirection::Horizontal,
            LayoutNode::leaf("leaf_b", "sess:leaf_b"),
        )
        .expect("split");

    match &split {
        LayoutNode::Split {
            direction,
            ratio,
            first,
            second,
            ..
        } => {
            assert_eq!(*direction, SplitDirection::Horizontal);
            assert_eq!(*ratio, 0.5);
            assert_eq!(first.first_leaf_id(), "leaf_a");
            assert_eq!(second.first_leaf_id(), "leaf_b");
        }
        LayoutNode::Leaf { .. } => panic!("expected a split"),
    }

    assert_eq!(split.leaves().len(), 2);
}

#[test]
fn nested_split_finds_the_inner_leaf() {
    assert_eq!(leaf_ids(&nested()), vec!["a", "b", "c"]);
}

#[test]
fn json_roundtrip_keeps_the_tag() {
    let tree = LayoutNode::leaf("leaf_a", "sess:leaf_a");
    let json = serde_json::to_value(&tree).expect("serialize");
    assert_eq!(json["type"], "leaf");
    assert_eq!(json["tmuxTarget"], "sess:leaf_a");
    let back: LayoutNode = serde_json::from_value(json).expect("deserialize");
    assert_eq!(back.first_leaf_id(), "leaf_a");
}

// ─── closing ────────────────────────────────────────────────────────

/// The hole this fills: the tree could only ever grow.
#[test]
fn closing_one_side_leaves_the_other_where_the_split_was() {
    let closed = nested().close_leaf("c").expect("close c");
    assert_eq!(leaf_ids(&closed), vec!["a", "b"]);

    // And the split that held b and c is gone with it, rather than standing
    // with an empty half.
    assert_eq!(closed.split_ids().len(), 1);
}

#[test]
fn closing_the_outer_leaf_keeps_the_whole_inner_split() {
    let closed = nested().close_leaf("a").expect("close a");
    assert_eq!(leaf_ids(&closed), vec!["b", "c"]);
    assert_eq!(closed.split_ids().len(), 1);
}

/// A session with no pane is not a layout. The caller turns this into a
/// sentence rather than an empty tree.
#[test]
fn the_last_leaf_will_not_close() {
    assert!(LayoutNode::leaf("only", "s:only")
        .close_leaf("only")
        .is_none());
}

#[test]
fn closing_a_leaf_that_is_not_here_changes_nothing() {
    assert!(nested().close_leaf("nowhere").is_none());
}

// ─── renaming ───────────────────────────────────────────────────────

#[test]
fn a_renamed_leaf_carries_the_name_and_the_others_do_not() {
    let named = nested().rename_leaf("b", "  the tests  ").expect("rename");
    let titles: Vec<&str> = collect_titles(&named);
    assert_eq!(titles, vec!["", "the tests", ""], "{titles:?}");
}

/// Clearing the name is how a pane goes back to showing what the program
/// calls itself.
#[test]
fn renaming_to_nothing_clears_the_name() {
    let named = nested().rename_leaf("b", "mine").expect("rename");
    let cleared = named.rename_leaf("b", "").expect("clear");
    assert_eq!(collect_titles(&cleared), vec!["", "", ""]);
}

#[test]
fn renaming_a_leaf_that_is_not_here_changes_nothing() {
    assert!(nested().rename_leaf("nowhere", "x").is_none());
}

fn collect_titles(tree: &LayoutNode) -> Vec<&str> {
    fn walk<'a>(node: &'a LayoutNode, out: &mut Vec<&'a str>) {
        match node {
            LayoutNode::Leaf { title, .. } => out.push(title),
            LayoutNode::Split { first, second, .. } => {
                walk(first, out);
                walk(second, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(tree, &mut out);
    out
}

// ─── boundaries ─────────────────────────────────────────────────────

#[test]
fn a_dragged_boundary_moves_only_its_own_split() {
    let tree = nested();
    let ids: Vec<String> = tree.split_ids().iter().map(|id| id.to_string()).collect();
    let moved = tree.set_ratio(&ids[1], 0.3).expect("set ratio");

    let ratios = collect_ratios(&moved);
    assert_eq!(ratios, vec![0.5, 0.3], "{ratios:?}");
}

/// A pane at ratio 0 holds a running process nobody can see or grab back.
#[test]
fn a_boundary_cannot_be_dragged_until_a_pane_disappears() {
    let tree = nested();
    let outer = tree.split_ids()[0].to_owned();

    let squashed = tree.set_ratio(&outer, 0.0).expect("set ratio");
    assert_eq!(collect_ratios(&squashed)[0], NARROWEST);

    let stretched = tree.set_ratio(&outer, 1.0).expect("set ratio");
    assert_eq!(collect_ratios(&stretched)[0], WIDEST);
}

/// NaN compares false against everything, so it walks through `clamp`
/// untouched and would be written to disk as a boundary position.
#[test]
fn a_boundary_that_is_not_a_number_is_refused() {
    let tree = nested();
    let outer = tree.split_ids()[0].to_owned();
    assert!(tree.set_ratio(&outer, f64::NAN).is_none());
    assert!(tree.set_ratio(&outer, f64::INFINITY).is_none());
}

#[test]
fn a_boundary_that_is_not_here_changes_nothing() {
    assert!(nested().set_ratio("sp_nowhere", 0.4).is_none());
}

fn collect_ratios(tree: &LayoutNode) -> Vec<f64> {
    fn walk(node: &LayoutNode, out: &mut Vec<f64>) {
        if let LayoutNode::Split {
            ratio,
            first,
            second,
            ..
        } = node
        {
            out.push(*ratio);
            walk(first, out);
            walk(second, out);
        }
    }
    let mut out = Vec::new();
    walk(tree, &mut out);
    out
}

// ─── the trees already on disk ──────────────────────────────────────

/// Splits gained ids after trees were persisted. One that predates them has
/// to load, not fail — and the boundary has to be draggable afterwards.
#[test]
fn a_tree_saved_before_splits_had_ids_still_loads_and_drags() {
    let stored = r#"{
        "type": "split",
        "direction": "horizontal",
        "ratio": 0.5,
        "first":  {"type":"leaf","id":"a","tmuxTarget":"s:a","kind":"terminal","agent":"none"},
        "second": {"type":"leaf","id":"b","tmuxTarget":"s:b","kind":"terminal","agent":"none"}
    }"#;

    let mut tree: LayoutNode = serde_json::from_str(stored).expect("an old tree still loads");
    assert_eq!(tree.split_ids(), vec![""], "the id should start empty");

    let mut next = 0;
    assert!(
        tree.name_the_splits(&mut || {
            next += 1;
            format!("sp_{next}")
        }),
        "it did not report that it had to name one"
    );
    assert_eq!(tree.split_ids(), vec!["sp_1"]);
    assert!(tree.set_ratio("sp_1", 0.25).is_some());
}

/// The second read must not rename what the first one named, or every restart
/// would hand the screen new handles for the same boundaries.
#[test]
fn naming_the_splits_twice_names_nothing_the_second_time() {
    let mut tree = nested();
    let before = tree.split_ids().join(",");
    assert!(!tree.name_the_splits(&mut || "sp_new".to_owned()));
    assert_eq!(tree.split_ids().join(","), before);
}
