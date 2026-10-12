use super::{decoded, file, keep, offer_of, read, Paired};

const SECRET: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

/// Where this devpit signs in, which the machine's link names too.
fn origin() -> String {
    crate::account::origin()
}

fn link(root: &std::path::Path, mcp: bool) -> String {
    crate::remote_relay::pairing_url(root, SECRET, "Jo's desk", mcp).expect("link")
}

fn refused(link: &str) -> String {
    match offer_of(link, &origin()) {
        Err(why) => why,
        Ok(_) => panic!("taken: {link}"),
    }
}

#[test]
fn the_link_the_machine_shows_for_an_mcp_client_is_read_whole() {
    let machine = tempfile::tempdir().expect("dir");
    let (host, keys) = crate::remote_relay::identity(machine.path()).expect("identity");
    let Ok(offer) = offer_of(&link(machine.path(), true), &origin()) else {
        panic!("refused");
    };
    assert_eq!(offer.host, host);
    assert_eq!(offer.machine, keys.public);
    assert_eq!(offer.secret, SECRET);
    assert_eq!(offer.name, "Jo's desk");
}

#[test]
fn a_link_for_a_phone_or_another_site_pairs_nothing_here() {
    let machine = tempfile::tempdir().expect("dir");
    assert!(refused(&link(machine.path(), false)).contains("phone"));
    // The account's session goes to the site this devpit signs in at, alone.
    let elsewhere = link(machine.path(), true).replace(&origin(), "https://devpit.example");
    assert!(refused(&elsewhere).contains("its session would go there"));
    let cut = link(machine.path(), true).replace(SECRET, &SECRET[..63]);
    assert!(refused(&cut).contains("cut short"));
    assert!(refused("not a link").contains("not a pairing link"));
}

#[test]
fn a_fragment_value_comes_back_as_it_was_escaped() {
    assert_eq!(
        decoded(&crate::remote_relay::fragment_value("Jo's Mac&co café")),
        "Jo's Mac&co café"
    );
    assert_eq!(decoded("100%"), "100%");
}

#[test]
fn this_ends_key_is_kept_for_this_user_alone() {
    let dir = tempfile::tempdir().expect("dir");
    let paired = Paired {
        host: "host_1".to_owned(),
        machine: "aa".repeat(32),
        name: "desk".to_owned(),
        secret: "bb".repeat(32),
        public: "cc".repeat(32),
    };
    keep(dir.path(), &paired).expect("kept");
    assert_eq!(
        read(dir.path()).map(|one| one.host).as_deref(),
        Some("host_1")
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(file(dir.path()))
            .expect("meta")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
