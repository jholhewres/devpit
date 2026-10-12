//! `noise-xx.json` is a whole conversation — handshake and frames — made by
//! snow with every key fixed. This test makes it again and holds it to the
//! file; the viewer's own Noise is held to the same file, byte for byte, in
//! `web/src/remote/noise.test.ts`. Two implementations that both reproduce
//! one transcript speak the same protocol.
//!
//! `DEVPIT_WRITE_NOISE_VECTOR=1` writes the file again, after a change to the
//! protocol that is meant.

use devpit_remote::noise::{hex, Handshake, Keys, Session, PATTERN};

const HOST: &str = "host_01VECTOR";
const FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/noise-xx.json");

fn bytes(from: u8) -> [u8; 32] {
    std::array::from_fn(|at| from + at as u8)
}

fn party(secret: [u8; 32]) -> Keys {
    // The public half is not needed to speak; the peer reads it off the wire.
    Keys {
        secret,
        public: [0; 32],
    }
}

fn vector() -> serde_json::Value {
    let (device_static, device_ephemeral) = (bytes(0x01), bytes(0x21));
    let (host_static, host_ephemeral) = (bytes(0x41), bytes(0x61));
    let mut device =
        Handshake::with_ephemeral(&party(device_static), HOST, true, &device_ephemeral)
            .expect("device");
    let mut host =
        Handshake::with_ephemeral(&party(host_static), HOST, false, &host_ephemeral).expect("host");
    let first = device.write().expect("first");
    host.read(&first).expect("first read");
    let second = host.write().expect("second");
    device.read(&second).expect("second read");
    let third = device.write().expect("third");
    host.read(&third).expect("third read");
    let (device_public, host_public) = (host.peer().expect("seen"), device.peer().expect("seen"));
    let mut device: Session = device.into_session(1 << 20).expect("device session");
    let mut host: Session = host.into_session(1 << 20).expect("host session");

    let mut frames = Vec::new();
    let mut say = |from: &str, by: &mut Session, to: &mut Session, message: &str| {
        let frame = by.seal(message.as_bytes()).expect("sealed").remove(0);
        let opened = to.open(&frame).expect("opened").expect("whole");
        assert_eq!(opened, message.as_bytes());
        frames.push(serde_json::json!({ "from": from, "message": message, "frame": hex(&frame) }));
    };
    say(
        "initiator",
        &mut device,
        &mut host,
        r#"{"t":"pair","secret":"00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff","name":"Pixel 9"}"#,
    );
    say(
        "responder",
        &mut host,
        &mut device,
        r#"{"t":"welcome","device":"Pixel 9","host":"notebook","typing":false,"answering":false}"#,
    );
    say("initiator", &mut device, &mut host, r#"{"t":"projects"}"#);
    say("initiator", &mut device, &mut host, "");

    serde_json::json!({
        "pattern": PATTERN,
        "host": HOST,
        "prologue": hex(&devpit_remote::noise::prologue(HOST)),
        "initiator": {
            "static": hex(&device_static),
            "ephemeral": hex(&device_ephemeral),
            "public": hex(&device_public),
        },
        "responder": {
            "static": hex(&host_static),
            "ephemeral": hex(&host_ephemeral),
            "public": hex(&host_public),
        },
        "handshake": [hex(&first), hex(&second), hex(&third)],
        "frames": frames,
    })
}

#[test]
fn snow_still_speaks_the_conversation_the_viewer_is_held_to() {
    let made = format!(
        "{}\n",
        serde_json::to_string_pretty(&vector()).expect("json")
    );
    if std::env::var("DEVPIT_WRITE_NOISE_VECTOR").is_ok_and(|value| value == "1") {
        std::fs::write(FILE, &made).expect("written");
    }
    let kept = std::fs::read_to_string(FILE).expect("tests/noise-xx.json");
    assert!(
        made == kept,
        "snow no longer makes tests/noise-xx.json; if the protocol changed on purpose, \
         write it again with DEVPIT_WRITE_NOISE_VECTOR=1 and hold the viewer to it"
    );
}
