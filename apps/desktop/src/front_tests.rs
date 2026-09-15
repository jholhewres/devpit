use super::*;

#[test]
fn a_running_run_refuses_even_when_forced() {
    let refused = delete_refusal(1, None, 0, true).expect("refused");
    assert!(!refused.forcible);
}

#[test]
fn an_agent_in_the_cards_terminal_refuses_even_when_forced() {
    let refused = delete_refusal(0, Some("Claude Code"), 0, true).expect("refused");
    assert!(!refused.forcible);
    assert!(refused.reason.contains("Claude Code"));
}

#[test]
fn unsaved_work_asks_first_and_force_goes_ahead() {
    let refused = delete_refusal(0, None, 2, false).expect("refused");
    assert!(refused.forcible);
    assert!(refused.reason.starts_with("2 changes"));
    assert_eq!(delete_refusal(0, None, 2, true), None);
}

#[test]
fn a_card_with_nothing_going_on_goes() {
    assert_eq!(delete_refusal(0, None, 0, false), None);
}
