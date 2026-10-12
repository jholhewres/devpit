use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::channel;

use super::*;

const HOST: &str = "host_01PAIRINGLOOPBACK";

type Plain = Link<TcpStream>;

/// The offering machine on loopback, answering one session with `keys`;
/// what `hand_over` came to arrives on the receiver.
fn offering(
    keys: Keys,
    secret: [u8; 32],
    key: SyncKey,
) -> (u16, std::sync::mpsc::Receiver<Result<String, NotHanded>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (tell, heard) = channel();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let ws = tungstenite::accept(stream).expect("ws");
        ws.get_ref()
            .set_read_timeout(Some(READ_WAIT))
            .expect("timeout");
        let Ok(mut link) = Link::answer(ws, &keys, HOST, MOST, WITHIN) else {
            let _ = tell.send(Err(NotHanded::Gone("handshake".to_owned())));
            return;
        };
        let _ = tell.send(hand_over(&mut link, &secret, &key));
        link.close();
    });
    (port, heard)
}

fn calling(port: u16, expected: &[u8; 32]) -> Result<Plain, devpit_remote::LinkError> {
    let stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let (ws, _) = tungstenite::client("ws://127.0.0.1/", stream).expect("ws");
    ws.get_ref()
        .set_read_timeout(Some(READ_WAIT))
        .expect("timeout");
    let keys = Keys::generate().expect("keys");
    Link::call(ws, &keys, HOST, expected, MOST, WITHIN)
}

#[test]
fn a_machine_with_the_offers_secret_takes_the_key() {
    let key = SyncKey::generate().expect("key");
    let (offer, keys) = Offer::new("laptop").expect("offer");
    let (port, handed) = offering(keys, offer.secret, key.clone());
    let mut link = calling(port, &offer.machine).expect("the offer's machine");
    let taken = take_over(&mut link, &offer.secret, "desktop").expect("the key");
    assert_eq!(taken.to_hex(), key.to_hex());
    assert_eq!(handed.recv().expect("handed"), Ok("desktop".to_owned()));
}

#[test]
fn a_wrong_secret_is_refused_and_no_key_is_sent() {
    let key = SyncKey::generate().expect("key");
    let (offer, keys) = Offer::new("laptop").expect("offer");
    let (port, handed) = offering(keys, offer.secret, key);
    let mut link = calling(port, &offer.machine).expect("the offer's machine");
    let mut guessed = offer.secret;
    guessed[0] ^= 1;
    let refused = take_over(&mut link, &guessed, "desktop").expect_err("no key");
    assert!(refused.contains("secret"), "{refused}");
    assert_eq!(handed.recv().expect("handed"), Err(NotHanded::WrongSecret));
}

#[test]
fn a_machine_that_is_not_the_one_offering_is_never_told_the_secret() {
    let key = SyncKey::generate().expect("key");
    let (offer, _) = Offer::new("laptop").expect("offer");
    // Something else answers where the offer said: devpit.app, say.
    let impostor = Keys::generate().expect("keys");
    let (port, handed) = offering(impostor, offer.secret, key);
    let refused = calling(port, &offer.machine).err().expect("refused");
    assert!(
        matches!(refused, devpit_remote::LinkError::Stranger),
        "{refused}"
    );
    assert!(matches!(
        handed.recv().expect("ended"),
        Err(NotHanded::Gone(_))
    ));
}

#[test]
fn an_offer_is_carried_as_text_and_read_back_whole() {
    let (offer, _) = Offer::new("Jo's laptop · Linux").expect("offer");
    let text = offer.text();
    assert!(text.starts_with(PREFIX));
    let read = Offer::parse(&format!("  {text}\n")).expect("read");
    assert_eq!(read.host, offer.host);
    assert_eq!(read.machine, offer.machine);
    assert_eq!(read.secret, offer.secret);
    assert_eq!(read.name, "Jo's laptop · Linux");
    assert!(Offer::parse(&text[..text.len() - 70]).is_err());
    assert!(Offer::parse("https://devpit.app/remote#relay=x").is_err());
    assert!(!format!("{offer:?}").contains(&hex(&offer.secret)));
}
