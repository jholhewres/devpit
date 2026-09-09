use super::*;

#[test]
fn a_session_that_did_not_ask_to_be_consulted_is_not_held() {
    let asking = Asking::default();
    assert!(!asking.asks("sess_1"));
}

#[test]
fn a_session_can_ask_to_be_consulted_and_stop_again() {
    let asking = Asking::default();
    asking.ask_from_now("sess_1", true);
    assert!(asking.asks("sess_1"));
    assert!(!asking.asks("sess_2"), "one session spoke for another");

    asking.ask_from_now("sess_1", false);
    assert!(!asking.asks("sess_1"));
}

#[test]
fn asking_twice_does_not_leave_two_of_the_same_session() {
    let asking = Asking::default();
    asking.ask_from_now("sess_1", true);
    asking.ask_from_now("sess_1", true);
    asking.ask_from_now("sess_1", false);
    assert!(!asking.asks("sess_1"), "one of the two was left behind");
}

#[test]
fn an_answer_reaches_the_question_that_is_waiting() {
    let asking = Asking::default();
    let hear = asking.opened("q1");
    assert!(asking.answer("q1", Answer::Allow));
    assert_eq!(asking.wait("q1", hear), Answer::Allow);
}

#[test]
fn answering_a_question_nobody_asked_says_so() {
    let asking = Asking::default();
    assert!(!asking.answer("q_nothing", Answer::Allow));
}

/// A question nobody answered is not a question someone said yes to.
#[test]
fn a_question_that_is_never_answered_ends_as_a_refusal() {
    let asking = Asking::default();
    let (_tell, hear) = channel::<Answer>();
    // The sender is dropped, so the wait ends at once rather than after the
    // real two minutes — the rule under test is which way it falls, not how
    // long it takes to get there.
    drop(_tell);
    assert_eq!(asking.wait("q1", hear), Answer::Deny);
}

#[test]
fn a_refusal_goes_back_as_a_decision_the_cli_reads() {
    let said: serde_json::Value = serde_json::from_str(&decision(Answer::Deny)).expect("json");
    assert_eq!(said["hookSpecificOutput"]["permissionDecision"], "deny");
    assert_eq!(said["hookSpecificOutput"]["hookEventName"], "PreToolUse");
}

#[test]
fn an_allowance_goes_back_the_same_way() {
    let said: serde_json::Value = serde_json::from_str(&decision(Answer::Allow)).expect("json");
    assert_eq!(said["hookSpecificOutput"]["permissionDecision"], "allow");
}
