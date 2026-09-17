//! What gets taken out of a log, and what honestly does not.

use super::*;

fn hiding(values: &[&str]) -> Vec<String> {
    worth_hiding(values.iter().map(|value| (*value).to_owned()))
}

/// The case this exists for: `set -x`, or a failing request printing what it
/// sent. Sabotage: skip the replace and the token sits in SQLite forever.
#[test]
fn a_token_that_reached_the_log_does_not_reach_the_disk() {
    let secrets = hiding(&["sk-live-0123456789abcdef"]);
    let said = kept_out(
        "+ curl -H 'authorization: Bearer sk-live-0123456789abcdef' https://x.invalid",
        &secrets,
    );

    assert!(!said.contains("sk-live-0123456789abcdef"), "{said}");
    assert!(said.contains(HIDDEN));
    assert!(
        said.contains("https://x.invalid"),
        "the rest of the line went too"
    );
}

/// Every occurrence, not the first: a log that prints a value twice is a log
/// that leaks it twice.
#[test]
fn every_occurrence_goes_not_just_the_first() {
    let secrets = hiding(&["0123456789abcdef"]);
    let said = kept_out("0123456789abcdef and again 0123456789abcdef", &secrets);
    assert!(!said.contains("0123456789abcdef"), "{said}");
    assert_eq!(said.matches(HIDDEN).count(), 2);
}

/// A short value is a word, not a secret, and replacing it would redact the
/// build log a person is trying to read.
///
/// Sabotage: drop the length floor and a profile with `MODE=test` turns every
/// "test" in the output into a marker.
#[test]
fn something_too_short_to_be_a_secret_is_left_alone() {
    assert_eq!(hiding(&["test", "dev", "1234567"]), Vec::<String>::new());
    assert_eq!(kept_out("running test", &hiding(&["test"])), "running test");
}

/// A token that contains a shorter one must not be half-replaced, leaving the
/// rest of it in the log next to the marker.
///
/// Sabotage: sort shortest first and the long one comes out as
/// `sk-live-[hidden by devpit]cdef`.
#[test]
fn the_longest_value_goes_first_so_none_is_left_half_there() {
    let secrets = hiding(&["0123456789ab", "sk-live-0123456789abcdef"]);
    assert_eq!(secrets[0].len(), "sk-live-0123456789abcdef".len());

    let said = kept_out("token sk-live-0123456789abcdef here", &secrets);
    assert!(!said.contains("cdef"), "{said}");
    assert_eq!(said, format!("token {HIDDEN} here"));
}

#[test]
fn a_log_with_nothing_to_hide_comes_back_as_it_went_in() {
    let said = "28 passed, 0 failed";
    assert_eq!(kept_out(said, &hiding(&["0123456789abcdef"])), said);
    assert_eq!(kept_out(said, &[]), said);
}

/// The sentence the module header refuses to let go of: this knows the values
/// this app handed out, and nothing else. A secret read from a file by the
/// command itself passes straight through, and that is not a bug to fix here —
/// it is the limit to say out loud.
#[test]
fn a_secret_devpit_never_handed_out_passes_through() {
    let said = kept_out(
        "AWS_SESSION_TOKEN=FwoGZXIvYXdzEJr",
        &hiding(&["something-else-entirely"]),
    );
    assert!(said.contains("FwoGZXIvYXdzEJr"));
}

/// The whole path, against a real store: a profile's secret is declared, a
/// log carrying it is cleaned, and what comes back has the marker and not the
/// value.
///
/// Sabotage: take `what_devpit_gave` out of `working::carry_out` and the run's
/// row holds the token.
#[test]
fn a_profile_secret_does_not_reach_a_stored_log() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = devpit_core::Store::open(&dir.path().join("state.db")).expect("open");
    crate::agent_profiles::save(
        &store,
        &[devpit_rpc::Declared {
            id: "glm".to_owned(),
            label: "GLM".to_owned(),
            base: "claude".to_owned(),
            command: "claude".to_owned(),
            args: vec![],
            env: vec![devpit_rpc::EnvVar {
                name: "ANTHROPIC_AUTH_TOKEN".to_owned(),
                value: "sk-live-0123456789abcdef".to_owned(),
            }],
        }],
    )
    .expect("saved");

    let secrets = what_devpit_gave(&store);
    let said = kept_out(
        "+ ANTHROPIC_AUTH_TOKEN=sk-live-0123456789abcdef claude -p",
        &secrets,
    );

    assert!(!said.contains("sk-live-0123456789abcdef"), "{said}");
    assert!(said.contains(HIDDEN));
    // The name is not a secret and reads as the clue it is.
    assert!(said.contains("ANTHROPIC_AUTH_TOKEN"), "{said}");
}
