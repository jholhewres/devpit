//! Firefox, whose only trap is the epoch.

use super::*;

fn a_store(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("devpit-ff-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let store = dir.join("cookies.sqlite");
    let _ = std::fs::remove_file(&store);
    let db = rusqlite::Connection::open(&store).unwrap();
    db.execute_batch(
        "CREATE TABLE moz_cookies (host TEXT, name TEXT, value TEXT, path TEXT, \
         expiry INTEGER, isSecure INTEGER, isHttpOnly INTEGER);",
    )
    .unwrap();
    store
}

/// Seconds since 1970, where Chromium counts microseconds since 1601. A store
/// read with the other epoch gives every cookie an expiry in the wrong
/// millennium, and the webview drops all of them — an import that looks like
/// it did nothing at all.
#[test]
fn an_expiry_is_read_as_seconds_and_not_as_microseconds() {
    let store = a_store("epoch");
    {
        let db = rusqlite::Connection::open(&store).unwrap();
        db.execute(
            "INSERT INTO moz_cookies VALUES ('example.com', 'a', 'v', '/', 1704067200, 0, 0)",
            [],
        )
        .unwrap();
    }
    let found = read(&store).expect("the store reads");
    assert_eq!(found[0].expires, Some(1_704_067_200));
}

#[test]
fn values_are_read_as_they_stand_because_firefox_stores_them_that_way() {
    let store = a_store("plain");
    {
        let db = rusqlite::Connection::open(&store).unwrap();
        db.execute(
            "INSERT INTO moz_cookies VALUES ('.example.com', 'session', 'a-value', '/', 0, 1, 1)",
            [],
        )
        .unwrap();
    }
    let found = read(&store).expect("the store reads");
    assert_eq!(found[0].value.seen(), "a-value");
    assert_eq!(found[0].expires, None, "zero is a session cookie");
    assert!(found[0].secure && found[0].http_only);
    assert_eq!(found[0].host, ".example.com", "the leading dot is kept");
}

#[test]
fn a_profile_that_was_never_opened_says_so() {
    assert!(matches!(
        read(std::path::Path::new("/nowhere/cookies.sqlite")),
        Err(Refused::NoStore(_))
    ));
}

/// This machine is the case: firefox installed and never run, so there is no
/// profile and nothing to offer. Offering it would be offering an import that
/// finds nothing.
#[test]
fn a_home_with_no_firefox_profile_offers_nothing() {
    let empty = std::env::temp_dir().join(format!("devpit-ff-empty-{}", std::process::id()));
    std::fs::create_dir_all(&empty).unwrap();
    assert!(stores_in(&empty).is_empty());
}
