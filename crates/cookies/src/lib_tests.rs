//! The wrapper that keeps a credential out of every log.

use super::*;

/// The whole reason `Secret` exists. A struct with a plain `String` in it is
/// one `dbg!` or one `format!("{err:?}")` away from a session token in a file.
#[test]
fn a_debug_line_never_carries_the_value() {
    let secret = Secret::new("sk-a-real-looking-session-token");
    let printed = format!("{secret:?}");
    assert!(
        !printed.contains("sk-a-real-looking-session-token"),
        "{printed}"
    );
    assert!(!printed.contains("token"), "{printed}");
    /* The length is all a log has any business knowing. */
    assert!(printed.contains("31 bytes"), "{printed}");
}

/// And a whole cookie, which is the struct that actually gets printed.
#[test]
fn a_cookie_printed_whole_still_hides_its_value() {
    let cookie = Cookie {
        host: ".example.com".to_owned(),
        name: "session".to_owned(),
        value: Secret::new("the-value-that-must-not-appear"),
        path: "/".to_owned(),
        expires: Some(1_700_000_000),
        secure: true,
        http_only: true,
    };
    let printed = format!("{cookie:?}");
    assert!(
        !printed.contains("the-value-that-must-not-appear"),
        "{printed}"
    );
    /* Everything else is there, because a log with no cookie in it is a log
    that cannot say which import went wrong. */
    assert!(printed.contains("example.com"), "{printed}");
    assert!(printed.contains("session"), "{printed}");
}

#[test]
fn the_value_comes_out_when_somebody_writes_that_it_should() {
    let secret = Secret::new("value");
    assert_eq!(secret.seen(), "value");
    assert!(Secret::new("").is_empty());
}

/// Each refusal sends somebody somewhere different, which is why they are not
/// one string.
#[test]
fn every_refusal_says_a_different_thing_to_do() {
    let said = [
        Refused::NoStore("/nowhere/Cookies".into()).to_string(),
        Refused::InUse("/here/Cookies".into()).to_string(),
        Refused::NoKey("the keyring said no".to_owned()).to_string(),
        Refused::WrongKey.to_string(),
    ];
    assert_eq!(
        said.iter().collect::<std::collections::HashSet<_>>().len(),
        4
    );
    assert!(said[1].contains("close the browser"), "{}", said[1]);
}
