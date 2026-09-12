//! A card that moves for ever.

use super::*;

fn flow(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(from, to)| ((*from).to_owned(), (*to).to_owned()))
        .collect()
}

#[test]
fn a_lane_that_sends_nowhere_does_not_loop() {
    assert!(!loops(&flow(&[]), "a"));
}

#[test]
fn a_straight_line_does_not_loop() {
    assert!(!loops(&flow(&[("a", "b"), ("b", "c")]), "a"));
}

#[test]
fn a_lane_pointing_at_itself_loops() {
    // Refused by `column_set_flow` before it reaches here, but this is the
    // rule and the rule should hold on its own.
    assert!(loops(&flow(&[("a", "a")]), "a"));
}

#[test]
fn two_lanes_pointing_at_each_other_loop() {
    // The one that actually happens: A approves into B, B approves back into
    // A, and a card ping-pongs between them spending money on every hop.
    let both = flow(&[("a", "b"), ("b", "a")]);
    assert!(loops(&both, "a"));
    assert!(loops(&both, "b"));
}

#[test]
fn a_long_way_round_still_loops() {
    assert!(loops(
        &flow(&[("a", "b"), ("b", "c"), ("c", "d"), ("d", "b")]),
        "a"
    ));
}

#[test]
fn a_lane_that_only_leads_into_a_loop_is_reported_from_where_it_starts() {
    // `a` never returns to itself, but a card starting there never stops.
    // The question is "does this end", not "does this come back to me".
    assert!(loops(&flow(&[("a", "b"), ("b", "c"), ("c", "b")]), "a"));
}

#[test]
fn a_diamond_that_rejoins_is_not_a_loop() {
    // Two lanes both approving into the same third one is ordinary, and
    // reporting it as a cycle would refuse a board people actually build.
    let rejoin = flow(&[("a", "c"), ("b", "c")]);
    assert!(!loops(&rejoin, "a"));
    assert!(!loops(&rejoin, "b"));
}
