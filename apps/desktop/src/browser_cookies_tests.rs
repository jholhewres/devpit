//! What a person agreed to hand over, and nothing beside it.

use devpit_cookies::Secret;

use super::*;

fn cookie(host: &str, name: &str) -> Cookie {
    Cookie {
        host: host.to_owned(),
        name: name.to_owned(),
        value: Secret::new("v"),
        path: "/".to_owned(),
        expires: None,
        secure: true,
        http_only: true,
    }
}

/// The subtle one, and the one that leaks: asking for `example.com` must not
/// match `notexample.com`. A naive `ends_with` does exactly that, and the
/// cookie it hands over is somebody's session on a site they never named.
#[test]
fn a_domain_does_not_match_a_site_that_merely_ends_with_it() {
    assert!(!belongs("notexample.com", "example.com"));
    assert!(!belongs("evilexample.com", "example.com"));
    assert!(!belongs("example.com.attacker.test", "example.com"));
}

#[test]
fn a_leading_dot_covers_the_subdomains_and_nothing_else_does() {
    assert!(belongs(".example.com", "example.com"));
    assert!(belongs("api.example.com", "example.com"));
    assert!(belongs("deep.api.example.com", "example.com"));
    assert!(belongs("example.com", "example.com"));
}

/// A cookie is sent UP the tree too, and this is the one the reviewer found:
/// a pane on `app.example.com` needs the `.example.com` cookie, because that
/// is the one that signs you in. Without it the import took nothing useful
/// and the screen said "Brought 0 cookies".
#[test]
fn asking_for_a_subdomain_takes_the_parent_cookie_that_signs_you_in() {
    assert!(belongs(".example.com", "app.example.com"));
    assert!(belongs("example.com", "deep.api.example.com"));
    /* And sideways is still refused in both directions. */
    assert!(!belongs("other.com", "app.example.com"));
    assert!(!belongs("notexample.com", "app.example.com"));
    assert!(!belongs("example.com.attacker.test", "app.example.com"));
}

#[test]
fn a_trailing_root_dot_and_an_empty_name_are_handled() {
    assert!(belongs("example.com.", "example.com"));
    assert!(!belongs("", "example.com"));
    assert!(!belongs("example.com", ""));
    assert!(!belongs(".", "example.com"));
}

#[test]
fn a_host_is_matched_whatever_its_case() {
    assert!(belongs("API.Example.COM", "example.com"));
    assert!(belongs(".Example.com", "EXAMPLE.COM"));
}

/// The rule the module exists for: a store is read whole because the file
/// gives no cheaper way, and only what was asked for leaves.
#[test]
fn only_the_domains_asked_for_leave_the_store() {
    let found = vec![
        cookie(".github.com", "session"),
        cookie("api.github.com", "token"),
        cookie(".mybank.example", "auth"),
        cookie("notgithub.com", "fake"),
    ];
    let keep = wanted(found, &["github.com".to_owned()]);
    let names: Vec<&str> = keep.iter().map(|one| one.name.as_str()).collect();
    assert_eq!(names, ["session", "token"]);
    /* The bank was in the same file and did not move. */
    assert!(!keep.iter().any(|one| one.name == "auth"));
    assert!(!keep.iter().any(|one| one.name == "fake"));
}

/// `wanted` narrows and never widens: no domains means no domains matched.
///
/// The *command* reads this the other way round — an empty list there means
/// the profile whole, which is what picking a browser out of the menu does —
/// and that decision lives in `browser_import`, not here, so that the
/// narrowing itself stays a function with one meaning.
#[test]
fn narrowing_to_nothing_keeps_nothing() {
    let found = vec![cookie(".github.com", "session")];
    assert!(wanted(found, &[]).is_empty());
}

#[test]
fn several_domains_are_all_honoured() {
    let found = vec![
        cookie(".github.com", "a"),
        cookie(".gitlab.com", "b"),
        cookie(".elsewhere.test", "c"),
    ];
    let keep = wanted(found, &["github.com".to_owned(), "gitlab.com".to_owned()]);
    assert_eq!(keep.len(), 2);
    assert!(!keep.iter().any(|one| one.name == "c"));
}

/// The dispatch `browser_import` does is an exact match on the browser's name
/// now that the profile is a field of its own — so every name the crate can
/// produce has to be one of the three arms, and a Firefox read must never be
/// handed to the Chromium reader because the label gained a word.
///
/// Read off the crate rather than restated: a list written here would agree
/// with the code on the day it was written and never again.
#[test]
fn every_browser_the_crate_names_reaches_the_reader_that_understands_it() {
    let home = tempfile::tempdir().expect("a temporary home");
    let home = home.path();

    std::fs::create_dir_all(home.join(".mozilla/firefox/abc.default")).expect("a firefox profile");
    std::fs::write(
        home.join(".mozilla/firefox/abc.default/cookies.sqlite"),
        b"",
    )
    .expect("a store");
    std::fs::create_dir_all(home.join(".config/google-chrome/Profile 1"))
        .expect("a chrome profile");
    std::fs::write(home.join(".config/google-chrome/Profile 1/Cookies"), b"").expect("a store");

    for one in devpit_cookies::firefox::stores_in(home) {
        assert_eq!(
            one.family, "Firefox",
            "a firefox store called itself {}",
            one.family
        );
        assert!(!one.name.is_empty(), "a firefox profile has no name");
    }
    for one in devpit_cookies::chromium::stores_in(home) {
        assert!(
            one.family != "Firefox" && one.family != "Safari",
            "{} would be read by the wrong reader",
            one.family
        );
        assert!(!one.name.is_empty(), "a chromium profile has no name");
    }
}

/// Two fields for the menu, one sentence for a report. Safari keeps one jar
/// and names no profile, so its sentence must not end in a dangling separator.
#[test]
fn a_store_names_itself_in_one_line_without_an_empty_half() {
    let with = Store {
        family: "Google Chrome".to_owned(),
        profile: "Profile 1".to_owned(),
        path: "/x".to_owned(),
        warning: String::new(),
    };
    assert_eq!(with.named(), "Google Chrome · Profile 1");

    let alone = Store {
        family: "Safari".to_owned(),
        profile: String::new(),
        path: "/x".to_owned(),
        warning: String::new(),
    };
    assert_eq!(alone.named(), "Safari");
}
