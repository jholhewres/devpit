//! What may be stored as an app's command.
//!
//! The value is written to disk by one session and executed by another, so
//! the question is not "does this work" but "what is the worst thing someone
//! could put here". There is no shell in the way, which is the real defence;
//! the field refuses anyway, so the answer arrives while they are typing.

use super::*;

#[test]
fn a_plain_program_is_a_program() {
    for command in [
        "code",
        "cursor",
        "zed",
        "subl",
        "/usr/local/bin/code",
        "nvim",
    ] {
        assert!(is_a_program(command), "{command} was refused");
    }
}

#[test]
fn a_command_line_is_not_a_program() {
    // `code --wait` is two things and this field holds one. Accepting it
    // would run a program literally called "code --wait", which does not
    // exist, and the app would simply never open with no reason given.
    for command in ["code --wait", "code\t-n", " ", ""] {
        assert!(!is_a_program(command), "{command:?} was accepted");
    }
}

#[test]
fn shell_punctuation_is_refused() {
    // None of these can become two commands — `Command::new` has no shell to
    // split them. They are refused so that the day someone puts a shell back
    // in the middle, the value that would have been dangerous is not already
    // sitting in everybody's settings file.
    for command in [
        "code; rm -rf ~",
        "code&&curl evil",
        "code|tee",
        "$(whoami)",
        "`id`",
        "code\nrm",
        "code>out",
        "co*de",
    ] {
        assert!(!is_a_program(command), "{command:?} was accepted");
    }
}

#[test]
fn a_command_longer_than_any_path_is_refused() {
    assert!(!is_a_program(&"a".repeat(513)));
}

#[test]
fn a_custom_label_becomes_an_id_that_can_key_a_menu() {
    assert_eq!(id_from("My Editor"), "my-editor");
    assert_eq!(id_from("  Zed  "), "zed");
    // Punctuation collapses rather than surviving into the id.
    assert_eq!(id_from("C++ IDE!"), "c-ide");
}

#[test]
fn a_label_with_nothing_usable_in_it_still_gets_an_id() {
    // Otherwise two apps named "!!!" and "???" would share an empty id and
    // adding the second would silently replace the first.
    let one = id_from("!!!");
    let two = id_from("???");
    assert!(one.starts_with("app-"));
    assert_ne!(one, two);
}
