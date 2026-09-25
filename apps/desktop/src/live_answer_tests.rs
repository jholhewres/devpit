use devpit_rpc::{ErrorCode, PendingPrompt, PromptOption};
use devpit_tmux::Key::{Down, Up};

use super::{keys_for, on_choice, still_asked};

fn asked(question: &str, labels: &[&str], cursor: u32) -> PendingPrompt {
    PendingPrompt {
        question: question.to_owned(),
        options: labels
            .iter()
            .map(|label| PromptOption {
                label: (*label).to_owned(),
                hint: None,
            })
            .collect(),
        cursor,
    }
}

#[test]
fn a_choice_is_reached_by_arrows_from_where_the_cursor_is() {
    assert_eq!(keys_for(0, 2), [Down, Down]);
    assert_eq!(keys_for(0, 1), [Down]);
    assert_eq!(keys_for(2, 1), [Up]);
    assert_eq!(keys_for(3, 1), [Up, Up]);
    assert_eq!(keys_for(1, 1), []);
}

#[test]
fn the_same_question_is_answered() {
    let seen = asked(
        "Bash command git status\nDo you want to proceed?",
        &["Yes", "No"],
        0,
    );
    assert!(still_asked(Some(&seen.clone()), &seen, Some(1)).is_ok());
    assert!(still_asked(Some(&seen.clone()), &seen, None).is_ok());
}

#[test]
fn a_click_meant_for_one_permission_does_not_approve_the_next() {
    let seen = asked(
        "Bash command git status\nDo you want to proceed?",
        &["Yes", "No"],
        0,
    );
    let now = asked(
        "Bash command rm -rf build\nDo you want to proceed?",
        &["Yes", "No"],
        0,
    );
    let refused = still_asked(Some(&now), &seen, Some(0)).expect_err("refused");
    assert_eq!(refused.code, ErrorCode::Conflict);
    // Escape too: it dismisses whatever is on screen, so it is checked the same.
    assert!(still_asked(Some(&now), &seen, None).is_err());
}

#[test]
fn a_question_no_longer_there_or_a_choice_it_never_had_is_refused() {
    let seen = asked("Proceed?", &["Yes", "No"], 0);
    assert_eq!(
        still_asked(None, &seen, Some(0)).expect_err("gone").code,
        ErrorCode::Conflict
    );
    assert_eq!(
        still_asked(Some(&seen.clone()), &seen, Some(2))
            .expect_err("out of range")
            .code,
        ErrorCode::Invalid
    );
    let moved = asked("Proceed?", &["Yes", "No"], 1);
    assert!(still_asked(Some(&moved), &seen, Some(0)).is_err());
}

#[test]
fn enter_waits_for_the_cursor_on_the_choice_of_the_same_question() {
    let seen = asked("Proceed?", &["Yes", "No"], 0);
    assert!(!on_choice(Some(&seen), &seen, 1));
    assert!(on_choice(
        Some(&asked("Proceed?", &["Yes", "No"], 1)),
        &seen,
        1
    ));
    assert!(!on_choice(
        Some(&asked("Other?", &["Yes", "No"], 1)),
        &seen,
        1
    ));
    assert!(!on_choice(None, &seen, 1));
}
