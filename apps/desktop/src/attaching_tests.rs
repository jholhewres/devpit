use std::path::Path;

use super::*;

#[test]
fn a_busy_terminal_is_not_typed_over() {
    let refused = attach_target(Path::new("/w/card"), "a1b2", &Ready::Busy("vim".to_owned()))
        .expect_err("refused");
    assert_eq!(refused.code, ErrorCode::Conflict);
    assert!(refused.message.contains("vim"), "{}", refused.message);
}

#[test]
fn the_session_is_attached_in_the_cards_checkout() {
    for ready in [Ready::Prompt, Ready::Unknown] {
        assert_eq!(
            attach_target(Path::new("/w/card one"), "a1b2", &ready).expect("a line"),
            "cd '/w/card one' && claude attach a1b2"
        );
    }
}

#[test]
fn a_folder_or_id_that_cannot_be_typed_is_refused() {
    assert!(attach_target(Path::new("/w/it's"), "a1b2", &Ready::Prompt).is_err());
    assert!(attach_target(Path::new("/w/card"), "../a1b2", &Ready::Prompt).is_err());
}
