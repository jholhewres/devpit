use super::*;

/// A write stamped, and the clock moved past it, as a round does.
fn tick(clock: &mut Clock, wall: u64) -> Stamp {
    let stamp = clock.next(wall);
    clock.observe(&stamp);
    stamp
}

#[test]
fn a_wall_clock_that_goes_back_does_not_take_the_stamps_with_it() {
    let mut clock = Clock::new("m_a");
    let first = tick(&mut clock, 10_000);
    let behind = tick(&mut clock, 4_000);
    let same = tick(&mut clock, 10_000);
    assert!(behind > first, "{behind:?} after {first:?}");
    assert!(same > behind);
    assert_eq!(behind.ms, 10_000, "the clock held where it was");
}

#[test]
fn a_stamp_seen_from_another_machine_is_followed_not_preceded() {
    let mut clock = Clock::new("m_a");
    tick(&mut clock, 1_000);
    let theirs = Stamp {
        ms: 50_000,
        n: 3,
        by: "m_b".to_owned(),
    };
    clock.observe(&theirs);
    let mine = tick(&mut clock, 2_000);
    assert!(mine > theirs, "{mine:?} after {theirs:?}");
}

#[test]
fn writes_found_in_one_look_keep_their_own_moments() {
    let mut clock = Clock::new("m_a");
    tick(&mut clock, 1_000);
    let early = clock.next(5_000);
    let late = clock.next(3_000);
    let behind = clock.next(500);
    assert!(early > late, "measured from the same floor");
    assert!(
        behind.ms == 1_000 && behind.n == 1,
        "{behind:?} still after the floor"
    );
}

#[test]
fn a_tie_is_settled_by_the_machine_the_same_way_everywhere() {
    let a = Stamp {
        ms: 7,
        n: 0,
        by: "m_a".to_owned(),
    };
    let b = Stamp {
        ms: 7,
        n: 0,
        by: "m_b".to_owned(),
    };
    assert!(b > a);
    assert_eq!(a.clone().max(b.clone()), b.clone().max(a.clone()));
}

#[test]
fn a_stamp_is_written_as_three_values() {
    let stamp = Stamp {
        ms: 12,
        n: 1,
        by: "m_a".to_owned(),
    };
    let text = serde_json::to_string(&stamp).expect("json");
    assert_eq!(text, r#"[12,1,"m_a"]"#);
    assert_eq!(serde_json::from_str::<Stamp>(&text).expect("back"), stamp);
}

#[test]
fn a_kept_clock_carries_on_where_it_stopped() {
    let mut clock = Clock::new("m_a");
    let before = tick(&mut clock, 9_000);
    let kept: Clock =
        serde_json::from_str(&serde_json::to_string(&clock).expect("json")).expect("read");
    let mut kept = kept;
    assert!(tick(&mut kept, 1_000) > before);
}
