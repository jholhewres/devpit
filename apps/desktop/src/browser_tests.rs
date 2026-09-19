//! What a browser pane may be pointed at, and what it may close.

use super::*;

/// The one that matters: the capability file grants `main` every command
/// devpit has, so a browser webview must never be able to answer to it.
#[test]
fn a_browser_webview_can_never_be_called_main() {
    for pane in ["main", "leaf_01", "../main", "devpit-browser:"] {
        let label = label_for(pane).expect("a named pane gets a label");
        assert_ne!(label, "main", "pane {pane:?}");
        assert!(ours(&label), "pane {pane:?} made {label}");
    }
}

/// The one this test found on its first run: a blank pane id made the bare
/// prefix, which `ours` refuses — so the webview would have been created and
/// then nothing would have agreed to close it. An orphan inside the window.
#[test]
fn a_pane_with_no_name_gets_no_webview() {
    for blank in ["", "   ", "\t\n"] {
        assert!(label_for(blank).is_err(), "{blank:?}");
    }
}

/// And the other direction: nothing this file did not make is closed by it.
#[test]
fn a_label_we_did_not_make_is_not_ours() {
    assert!(!ours("main"));
    assert!(!ours(""));
    assert!(!ours("devpit-browser:"), "the bare prefix names no pane");
    assert!(!ours("browser:leaf_01"));
    assert!(ours("devpit-browser:leaf_01"));
}

/// An allowlist, and the test says why each refusal exists rather than only
/// that it happens.
#[test]
fn only_http_and_https_are_reachable() {
    assert!(reachable("https://example.invalid/page").is_ok());
    assert!(reachable("http://localhost:3000").is_ok());

    for (url, why) in [
        ("file:///etc/passwd", "reads this machine's disk"),
        (
            "data:text/html,<script>x</script>",
            "is script the caller wrote",
        ),
        ("javascript:alert(1)", "is script the caller wrote"),
        ("about:blank", "is nothing anybody asked to browse"),
    ] {
        let refused = reachable(url).expect_err(&format!("{url} {why}"));
        assert!(!refused.is_empty(), "{url}");
    }

    assert!(reachable("not a url at all").is_err());
    assert!(reachable("").is_err());
}

/// A pane that has not been measured yet reports zeroes, and a webview of
/// zero height is one nobody can see and nobody can close.
#[test]
fn a_pane_that_was_never_measured_still_gets_a_usable_size() {
    let none = sized(Where {
        x: -10.0,
        y: -1.0,
        width: 0.0,
        height: 0.0,
    });
    assert_eq!(none.x, 0.0);
    assert_eq!(none.y, 0.0);
    assert!(none.width >= 1.0);
    assert!(none.height >= 1.0);
}

#[test]
fn a_measured_pane_is_left_exactly_where_it_is() {
    let asked = Where {
        x: 12.5,
        y: 40.0,
        width: 800.0,
        height: 600.0,
    };
    assert_eq!(sized(asked), asked);
}

/// The pane a webview belongs to, back out of its label — which is what the
/// page-load handler has to work from, since a webview only knows its own name.
#[test]
fn a_label_says_which_pane_it_belongs_to() {
    let label = label_for("leaf_01").expect("a named pane gets a label");
    assert_eq!(pane_of(&label), Some("leaf_01"));

    /* And nothing that is not ours claims a pane. */
    assert_eq!(pane_of("main"), None);
    assert_eq!(
        pane_of("devpit-browser:"),
        None,
        "the bare prefix names no pane"
    );
    assert_eq!(pane_of(""), None);
}

/// A session name comes from a screen and becomes a path. Refused rather than
/// sanitised: a sanitised name silently becomes a *different* session, and the
/// person is quietly signed in as somebody else.
#[test]
fn a_session_name_that_is_not_plain_is_refused_and_not_cleaned_up() {
    let root = std::path::Path::new("/tmp/devpit-test");
    for bad in [
        "../escape",
        "a/b",
        "..",
        "with space",
        "quote'",
        "dot.dot",
        "",
    ] {
        assert!(session_dir(root, bad).is_err(), "{bad:?} was allowed");
    }
}

#[test]
fn a_plain_session_name_lands_under_the_store_and_nowhere_else() {
    let root = std::path::Path::new("/tmp/devpit-test");
    let at = session_dir(root, "work").expect("a plain name");
    assert!(at.starts_with(root), "{}", at.display());
    assert!(at.ends_with("browser/work"), "{}", at.display());

    /* Two names are two directories, which is what makes sessions separate at
    all: wry turns each into its own cookie file and data store. */
    let other = session_dir(root, "personal").unwrap();
    assert_ne!(at, other);
    assert_eq!(
        session_dir(root, USUAL).unwrap().file_name().unwrap(),
        USUAL
    );
}

#[test]
fn the_names_people_actually_type_are_allowed() {
    let root = std::path::Path::new("/tmp/devpit-test");
    for good in ["default", "work", "client-2", "my_account", "a1"] {
        assert!(session_dir(root, good).is_ok(), "{good:?} was refused");
    }
    /* Trimmed, because a trailing space is a typo and not a second session. */
    assert_eq!(
        session_dir(root, " work ").unwrap(),
        session_dir(root, "work").unwrap()
    );
}
