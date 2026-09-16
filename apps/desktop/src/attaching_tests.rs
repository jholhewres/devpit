use std::path::Path;

use super::*;

#[test]
fn a_busy_terminal_is_not_typed_over() {
    let refused = attach_target(
        Path::new("/w/card"),
        "a1b2",
        None,
        &Ready::Busy("vim".to_owned()),
    )
    .expect_err("refused");
    assert_eq!(refused.code, ErrorCode::Conflict);
    assert!(refused.message.contains("vim"), "{}", refused.message);
}

#[test]
fn the_session_is_attached_in_the_cards_checkout() {
    for ready in [Ready::Prompt, Ready::Unknown] {
        assert_eq!(
            attach_target(Path::new("/w/card one"), "a1b2", None, &ready).expect("a line"),
            "cd '/w/card one' && claude attach a1b2"
        );
    }
}

#[test]
fn a_folder_or_id_that_cannot_be_typed_is_refused() {
    assert!(attach_target(Path::new("/w/it's"), "a1b2", None, &Ready::Prompt).is_err());
    assert!(attach_target(Path::new("/w/card"), "../a1b2", None, &Ready::Prompt).is_err());
}

/// Attached by the build that started it.
///
/// A session started under a profile answers to that profile's binary and its
/// account; attaching it with the default one asks a different CLI about a
/// session it has never heard of. The variables go in front exactly as the
/// launch line puts them — and, exactly as there, a token in one of them is in
/// this terminal's scrollback (`running::line`, SECURITY.md).
#[test]
fn the_attach_line_names_the_profile_binary() {
    let runner = devpit_agentcli::running::Runner {
        program: "/opt/claw/bin/claw".to_owned(),
        args: vec!["--profile".to_owned(), "work".to_owned()],
        env: vec![(
            "ANTHROPIC_BASE_URL".to_owned(),
            "https://example.invalid".to_owned(),
        )],
    };

    let line =
        attach_target(Path::new("/w/card"), "a1b2", Some(&runner), &Ready::Prompt).expect("a line");

    assert_eq!(
        line,
        "cd '/w/card' && ANTHROPIC_BASE_URL='https://example.invalid' \
         /opt/claw/bin/claw --profile work attach a1b2"
            .replace("\\\n         ", ""),
        "{line}"
    );
}
