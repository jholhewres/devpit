//! The ownership rule, tested by calling it.

use super::*;

/// Ids are minted as an ever-increasing counter, zero-padded so they compare
/// as strings the way they compare as numbers. Fixed width matters: without
/// the padding, "99" sorts above "100".
fn client(n: u64) -> String {
    format!("{n:016}")
}

/// The race this exists for: a React remount attaches before the old attach
/// has noticed it is over, and an old `invoke` still in flight must not take
/// the pane back from the client that replaced it.
#[test]
fn an_older_attach_cannot_replace_a_newer_claim() {
    let claims = Claims::new();
    assert!(claims
        .take_for("leaf", &client(100))
        .expect("first claim")
        .is_none());

    let stale = claims.take_for("leaf", &client(99));
    assert!(
        matches!(
            stale,
            Err(RpcError {
                code: ErrorCode::Conflict,
                ..
            })
        ),
        "an older client took the pane"
    );

    assert!(claims
        .take_for("leaf", &client(101))
        .expect("newer claim")
        .is_none());
    assert!(claims.release("leaf", &client(101)).expect("release"));
    assert!(
        claims.take_for("leaf", &client(100)).is_err(),
        "an old invoke arrived after detach and reclaimed the pane"
    );
}

/// Releasing is only for the client that holds it. A detach arriving late
/// from a client that has already been replaced must not free the pane for
/// the one that replaced it.
#[test]
fn only_the_holder_releases_the_claim() {
    let claims = Claims::new();
    claims.take_for("leaf", &client(1)).expect("claim");
    claims.take_for("leaf", &client(2)).expect("newer claim");

    assert!(
        !claims.release("leaf", &client(1)).expect("release"),
        "a replaced client released a claim it no longer held"
    );
    assert!(claims.release("leaf", &client(2)).expect("release"));
}

/// Two panes are two claims. A handover on one must not disturb the other.
#[test]
fn panes_are_claimed_independently() {
    let claims = Claims::new();
    claims.take_for("one", &client(5)).expect("claim one");
    claims.take_for("two", &client(3)).expect("claim two");

    // Lower than "one" holds, but this is a different pane.
    assert!(claims.take_for("two", &client(4)).is_ok());
    assert!(claims.take_for("one", &client(4)).is_err());
}

/// A pane nobody has attached is a sentence, not a panic.
#[test]
fn a_pane_with_no_client_says_so() {
    let claims = Claims::new();
    let missing = claims.live("never-attached");
    assert!(matches!(
        missing,
        Err(RpcError {
            code: ErrorCode::NotFound,
            ..
        })
    ));
}
