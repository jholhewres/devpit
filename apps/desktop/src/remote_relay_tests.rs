use devpit_rpc::RemoteIn;

use super::{fragment_value, identify, identity, wait_after, LONGEST_WAIT};
use crate::remote_devices::Devices;

const SECRET: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

fn pairing(secret: &str) -> Option<RemoteIn> {
    Some(RemoteIn::Pair {
        secret: secret.to_owned(),
        name: "Pixel".to_owned(),
    })
}

#[test]
fn an_unknown_key_pairs_only_with_the_secret_on_offer() {
    let mut devices = Devices::default();
    let phone = [3u8; 32];
    let on_offer = |secret: &str| secret == SECRET;

    let wrong = identify(&mut devices, &phone, || pairing("guess"), on_offer, 1.0);
    assert!(wrong.is_err());
    let said_nothing = identify(&mut devices, &phone, || None, on_offer, 1.0);
    assert!(said_nothing.is_err());
    let asked_first = identify(
        &mut devices,
        &phone,
        || Some(RemoteIn::Projects),
        on_offer,
        1.0,
    );
    assert!(
        asked_first.is_err(),
        "a stranger is not served before it pairs"
    );
    assert!(devices.devices.is_empty());

    let (device, fresh) =
        identify(&mut devices, &phone, || pairing(SECRET), on_offer, 2.0).expect("paired");
    assert!(fresh);
    assert_eq!(device.name, "Pixel");
    assert!(
        !device.typing && !device.answering,
        "a new device only sees"
    );
}

#[test]
fn a_paired_key_is_its_device_without_a_word_until_it_is_forgotten() {
    let mut devices = Devices::default();
    let phone = [4u8; 32];
    let id = devices.pair_key("Pixel", &phone, 1.0).expect("paired").id;

    let never = || -> Option<RemoteIn> { panic!("a known device is not asked for a secret") };
    let (device, fresh) = identify(&mut devices, &phone, never, |_| false, 5.0).expect("known");
    assert_eq!((device.id.as_str(), fresh), (id.as_str(), false));
    assert_eq!(
        devices.by_key(&phone).and_then(|one| one.last_seen),
        Some(5.0)
    );

    // Unpaired: the same key, at its next handshake, is a stranger again —
    // and the secret it paired with once does not bring it back.
    assert!(devices.forget(&id));
    assert!(identify(&mut devices, &phone, || None, |_| false, 6.0).is_err());
    assert!(identify(&mut devices, &phone, || pairing(SECRET), |_| false, 6.0).is_err());
}

#[test]
fn this_machine_keeps_one_id_and_key_on_the_relay() {
    let dir = tempfile::tempdir().expect("dir");
    let (host, keys) = identity(dir.path()).expect("made");
    assert!(host.starts_with("host_") && devpit_remote::relay::id_ok(&host));
    let (again, same) = identity(dir.path()).expect("read");
    assert_eq!((again, same), (host, keys.clone()));

    let text = std::fs::read_to_string(dir.path().join("remote-relay.json")).expect("kept");
    assert!(text.contains(&devpit_remote::noise::hex(&keys.public)));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(dir.path().join("remote-relay.json"))
            .expect("meta")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "the key is this user's alone");
    }
}

#[test]
fn reaching_the_relay_again_waits_longer_each_time_up_to_a_minute() {
    assert_eq!(wait_after(0, 0), std::time::Duration::from_secs(1));
    assert_eq!(wait_after(3, 0), std::time::Duration::from_secs(8));
    assert_eq!(wait_after(30, 0), LONGEST_WAIT);
    assert!(wait_after(30, 999) < LONGEST_WAIT + std::time::Duration::from_secs(1));
}

#[test]
fn a_machines_name_goes_into_the_pairing_link_escaped() {
    assert_eq!(fragment_value("notebook"), "notebook");
    assert_eq!(fragment_value("Jo's Mac&co"), "Jo%27s%20Mac%26co");
    assert_eq!(fragment_value("café"), "caf%C3%A9");
}
