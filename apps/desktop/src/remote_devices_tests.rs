use super::{fresh_token, hash, Devices, MOST};

#[test]
fn a_device_is_known_by_its_token_and_kept_only_as_a_hash() {
    let dir = tempfile::tempdir().expect("dir");
    let mut devices = Devices::default();
    let token = fresh_token().expect("token");
    let phone = devices.pair("  iPhone\n", &token, 10.0).expect("paired");
    assert_eq!(phone.name, "iPhone");
    assert!(!phone.typing && !phone.answering, "a new device only sees");
    devices.write(dir.path()).expect("write");

    let text = std::fs::read_to_string(super::file(dir.path())).expect("read");
    assert!(!text.contains(&token), "the token itself is never written");
    assert!(text.contains(&hash(&token)));

    let read = Devices::read(dir.path());
    assert_eq!(
        read.by_token(&token).map(|one| one.id.as_str()),
        Some(phone.id.as_str())
    );
    assert!(read.by_token("not it").is_none());
}

#[test]
fn what_a_device_may_do_is_set_here_and_forgetting_it_ends_it() {
    let mut devices = Devices::default();
    let token = fresh_token().expect("token");
    let id = devices.pair("Pixel", &token, 1.0).expect("paired").id;
    assert!(devices.allow(&id, true, false));
    assert!(devices
        .by_token(&token)
        .is_some_and(|one| one.typing && !one.answering));
    assert!(devices.forget(&id));
    assert!(devices.by_token(&token).is_none());
    assert!(!devices.forget(&id));
}

#[test]
fn only_a_few_devices_are_paired() {
    let mut devices = Devices::default();
    for at in 0..MOST {
        devices
            .pair(&format!("d{at}"), &format!("t{at}"), 0.0)
            .expect("paired");
    }
    assert!(devices.pair("one more", "t", 0.0).is_err());
}
