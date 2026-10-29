use super::*;

/// Every spelling of one repository a clone may carry.
const ONE_REPOSITORY: &[&str] = &[
    "git@github.com:jholhewres/devpit.git",
    "git@github.com:jholhewres/devpit",
    "https://github.com/jholhewres/devpit",
    "https://github.com/jholhewres/devpit.git",
    "https://github.com/jholhewres/devpit.git/",
    "https://github.com/JholHewres/devpit/",
    "https://jhol@github.com/jholhewres/devpit.git",
    "https://jhol:<REDACTED>@github.com/jholhewres/devpit.git",
    "https://github.com:443/jholhewres/devpit.git",
    "http://github.com/jholhewres/devpit",
    "ssh://git@github.com/jholhewres/devpit.git",
    "ssh://git@github.com:22/jholhewres/devpit.git",
    "git://github.com/jholhewres/devpit.git",
    "  git@github.com:jholhewres/devpit.git\n",
];

#[test]
fn every_spelling_of_one_repository_hashes_the_same() {
    for url in ONE_REPOSITORY {
        assert_eq!(normalised(url), "github.com/jholhewres/devpit", "{url}");
    }
}

#[test]
fn the_hash_of_a_plain_origin_is_the_one_devpit_already_keeps() {
    // Kept by every install since projects had origins: moving it would
    // orphan the synced record of every project.
    assert_eq!(
        origin_hash("git@github.com:jholhewres/devpit.git"),
        "da3b9997c45194efc9a8998be0ac81b3"
    );
}

#[test]
fn different_repositories_hash_apart() {
    assert_ne!(
        origin_hash("git@github.com:jholhewres/devpit.git"),
        origin_hash("git@github.com:jholhewres/devpit-app.git")
    );
    assert_ne!(
        origin_hash("git@github.com:jholhewres/devpit.git"),
        origin_hash("git@gitlab.com:jholhewres/devpit.git")
    );
}

#[test]
fn a_shared_origin_keeps_no_password_or_token() {
    assert_eq!(
        shareable("https://jhol:<REDACTED>@github.com/jholhewres/devpit.git"),
        "https://github.com/jholhewres/devpit.git"
    );
    assert_eq!(
        shareable("https://x-access-token:<REDACTED>@github.com/o/r"),
        "https://github.com/o/r"
    );
    assert_eq!(
        shareable("https://jhol@github.com/o/r"),
        "https://github.com/o/r"
    );
}

#[test]
fn a_shared_ssh_origin_keeps_what_cloning_needs() {
    assert_eq!(
        shareable("git@github.com:jholhewres/devpit.git"),
        "git@github.com:jholhewres/devpit.git"
    );
    assert_eq!(
        shareable("ssh://git@host.example:2222/o/r.git"),
        "ssh://git@host.example:2222/o/r.git"
    );
}

#[test]
fn two_ipv6_hosts_are_two_repositories() {
    assert_ne!(
        normalised("ssh://git@[2001:db8::1]:22/o/r.git"),
        normalised("ssh://git@[2001:db8::2]/o/r.git")
    );
    assert_eq!(
        normalised("ssh://git@[2001:db8::1]:22/o/r.git"),
        "[2001:db8::1]/o/r"
    );
}

#[test]
fn an_origin_that_names_nothing_is_no_identity() {
    for url in ["", "  ", "/", ".git", "file://", "https://"] {
        assert_eq!(identity(url), None, "{url:?}");
    }
    assert_eq!(
        identity("git@github.com:jholhewres/devpit.git").as_deref(),
        Some("da3b9997c45194efc9a8998be0ac81b3")
    );
}

#[test]
fn a_local_path_origin_keeps_the_hash_it_had() {
    assert_eq!(
        origin_hash("/srv/git/repo.git"),
        "5e32d59440204430af7768db4a5b99c0"
    );
}

#[test]
fn a_token_as_a_user_leaves_on_no_scheme() {
    for (url, shared) in [
        (
            "HTTPS://tok@github.com/o/r.git",
            "HTTPS://github.com/o/r.git",
        ),
        ("ftps://tok@host/o/r.git", "ftps://host/o/r.git"),
        ("git://tok@host/o/r.git", "git://host/o/r.git"),
        (
            "ssh://deploy:<REDACTED>@host/o/r.git",
            "ssh://deploy@host/o/r.git",
        ),
        ("git:<REDACTED>@host:o/r.git", "git@host:o/r.git"),
    ] {
        assert_eq!(shareable(url), shared, "{url}");
    }
}
