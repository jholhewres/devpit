use devpit_rpc::RemoteIn;

use super::{admit, fragment_value, identify, identity, wait_after, Who, LONGEST_WAIT};
use crate::remote_devices::Devices;

const SECRET: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

/// [`identify`], then [`admit`] with the person's answer `allowed`.
fn through(
    devices: &mut Devices,
    peer: &[u8; 32],
    first: impl FnOnce() -> Option<RemoteIn>,
    take_secret: impl FnOnce(&str) -> Option<bool>,
    now: f64,
) -> Result<(crate::remote_devices::Device, bool), String> {
    let who = identify(devices, peer, first, take_secret, now)?;
    admit(devices, peer, who, true, now)
}

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
    let on_offer = |secret: &str| (secret == SECRET).then_some(false);

    let wrong = through(&mut devices, &phone, || pairing("guess"), on_offer, 1.0);
    assert!(wrong.is_err());
    let said_nothing = through(&mut devices, &phone, || None, on_offer, 1.0);
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
        through(&mut devices, &phone, || pairing(SECRET), on_offer, 2.0).expect("paired");
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
    let id = devices
        .pair_key("Pixel", &phone, false, 1.0)
        .expect("paired")
        .id;

    let never = || -> Option<RemoteIn> { panic!("a known device is not asked for a secret") };
    let (device, fresh) = through(&mut devices, &phone, never, |_| None, 5.0).expect("known");
    assert_eq!((device.id.as_str(), fresh), (id.as_str(), false));
    assert_eq!(
        devices.by_key(&phone).and_then(|one| one.last_seen),
        Some(5.0)
    );

    // Unpaired: the same key, at its next handshake, is a stranger again —
    // and the secret it paired with once does not bring it back.
    assert!(devices.forget(&id));
    assert!(through(&mut devices, &phone, || None, |_| None, 6.0).is_err());
    assert!(through(&mut devices, &phone, || pairing(SECRET), |_| None, 6.0).is_err());
}

#[test]
fn a_key_paired_from_an_offer_for_an_mcp_client_is_one() {
    let mut devices = Devices::default();
    let laptop = [5u8; 32];
    let for_mcp = |secret: &str| (secret == SECRET).then_some(true);
    let (device, fresh) =
        through(&mut devices, &laptop, || pairing(SECRET), for_mcp, 1.0).expect("paired");
    assert!(fresh && device.mcp);
    let (again, fresh) = through(&mut devices, &laptop, || None, |_| None, 2.0).expect("known");
    assert!(
        !fresh && again.mcp,
        "known by its key as what it was paired as"
    );
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

/// A device that proved the secret is paired only once the person at the
/// machine allows it; refused, it is not kept, and the secret is spent.
#[test]
fn a_device_with_the_secret_waits_for_the_person_and_a_no_pairs_nothing() {
    let mut devices = Devices::default();
    let phone = [6u8; 32];
    let mut offered = Some(SECRET);
    let mut take = |secret: &str| (offered.take() == Some(secret)).then_some(false);
    let who = identify(&mut devices, &phone, || pairing(SECRET), &mut take, 1.0).expect("asks");
    assert!(matches!(&who, Who::Asking { name, mcp: false } if name == "Pixel"));
    assert!(admit(&mut devices, &phone, who, false, 1.0).is_err());
    assert!(devices.devices.is_empty(), "refused, and kept anyway");
    assert!(
        identify(&mut devices, &phone, || pairing(SECRET), &mut take, 2.0).is_err(),
        "the secret pairs once, allowed or not"
    );

    let take = |secret: &str| (secret == SECRET).then_some(false);
    let who = identify(&mut devices, &phone, || pairing(SECRET), take, 3.0).expect("asks");
    let (device, fresh) = admit(&mut devices, &phone, who, true, 3.0).expect("allowed");
    assert!(fresh && device.key.is_some());
}

#[test]
fn an_mcp_client_is_not_asked_about_its_own_offer() {
    let mut devices = Devices::default();
    let who = Who::Asking {
        name: "laptop".to_owned(),
        mcp: true,
    };
    assert!(admit(&mut devices, &[7u8; 32], who, false, 1.0).is_ok());
}
