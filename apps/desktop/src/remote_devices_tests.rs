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

#[test]
fn a_device_through_the_relay_is_known_by_its_key_and_by_nothing_else() {
    let dir = tempfile::tempdir().expect("dir");
    let mut devices = Devices::default();
    let key = [7u8; 32];
    let phone = devices.pair_key("Pixel", &key, false, 5.0).expect("paired");
    assert!(!phone.typing && !phone.answering, "a new device only sees");
    devices.write(dir.path()).expect("write");

    let mut read = Devices::read(dir.path());
    assert_eq!(
        read.by_key(&key).map(|one| one.id.as_str()),
        Some(phone.id.as_str())
    );
    assert!(read.by_key(&[8u8; 32]).is_none());
    // No token was ever given, so none — not even an empty one — finds it.
    assert!(read.by_token("").is_none());

    // Paired again, the key is one device, not two.
    let again = read.pair_key("Pixel 9", &key, false, 6.0).expect("again");
    assert_eq!(read.devices.len(), 1);
    assert!(read.forget(&again.id));
    assert!(
        read.by_key(&key).is_none(),
        "forgotten, the key opens nothing"
    );
}

#[test]
fn an_mcp_client_reads_and_is_never_granted_more() {
    let dir = tempfile::tempdir().expect("dir");
    let mut devices = Devices::default();
    let laptop = devices
        .pair_key("devpit MCP on laptop", &[9u8; 32], true, 1.0)
        .expect("paired");
    assert!(laptop.mcp && !laptop.typing && !laptop.answering);
    assert!(!devices.allow(&laptop.id, true, true));
    devices.write(dir.path()).expect("write");
    let read = Devices::read(dir.path());
    assert!(read
        .by_key(&[9u8; 32])
        .is_some_and(|one| one.mcp && !one.typing && !one.answering));

    // A device kept before MCP clients existed is a viewer.
    let old = r#"{"devices":[{"id":"dev_1","name":"Pixel","key":"00","pairedAt":1.0}]}"#;
    let old: Devices = serde_json::from_str(old).expect("read");
    assert!(!old.devices[0].mcp);
}
