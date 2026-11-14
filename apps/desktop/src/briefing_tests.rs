use super::{ended_since, lines, Gathered};

#[test]
fn the_comeback_briefing_says_the_sessions_the_merged_prs_and_the_decisions_waiting() {
    let gathered = Gathered {
        running: 2,
        waiting: 1,
        ended: 3,
        merged: vec![
            ("api".to_owned(), 12, "Fix the login".to_owned()),
            ("web".to_owned(), 3, "Dark theme".to_owned()),
            ("web".to_owned(), 4, "Footer".to_owned()),
            ("api".to_owned(), 13, "Rate limits".to_owned()),
        ],
        memory: 2,
        schedules: 1,
        drafts: 1,
        plans: 0,
    };
    let said = lines(&gathered);
    assert!(said.len() <= 5, "{said:?}");
    assert_eq!(
        said,
        [
            "Sessions: 2 running, 1 waiting on you, 3 ended.",
            "Merged: api #12 Fix the login; web #3 Dark theme; web #4 Footer, and 1 more.",
            "Decisions waiting: 2 memory proposals, 1 schedule proposal, 1 draft to send.",
        ]
    );
}

#[test]
fn a_quiet_absence_is_said_as_quiet() {
    assert_eq!(
        lines(&Gathered::default()),
        [
            "Sessions: none ran.",
            "Merged: no pull request.",
            "Decisions waiting: none.",
        ]
    );
}

/// Ended sessions are kept in milliseconds; an absence is counted in seconds.
#[test]
fn only_a_session_that_ended_during_the_absence_counts_as_ended() {
    // Left at 2026-11-13 12:00 UTC.
    let left = 1_794_571_200;
    let three_days_before = (left - 3 * 86_400) as f64 * 1000.0;
    let ten_minutes_after = (left + 600) as f64 * 1000.0;
    assert!(!ended_since(three_days_before, left));
    assert!(ended_since(ten_minutes_after, left));
}
