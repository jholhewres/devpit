use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{refused, request_of, taking, Pairing};

fn asked() -> Pairing {
    Pairing {
        code: "ABCD2345".to_owned(),
        name: "Pixel".to_owned(),
    }
}

/// A device pairing over the tailnet with the code on screen is still asked
/// about, by the name it gave, and paired only on a yes.
#[test]
fn a_tailnet_pairing_asks_the_person_after_the_code() {
    let mut asked_about = Vec::new();
    let no = refused(
        &asked(),
        |code| code == "ABCD2345",
        |name| {
            asked_about.push(name.to_owned());
            false
        },
    );
    assert!(no.is_some());
    assert_eq!(asked_about, ["Pixel"]);
    assert!(refused(&asked(), |_| true, |_| true).is_none());
}

#[test]
fn a_wrong_code_is_refused_before_anyone_is_asked() {
    let refusal = refused(&asked(), |_| false, |_| panic!("asked with a wrong code"));
    assert!(refusal.is_some_and(|why| String::from_utf8_lossy(why).contains("code")));
}

/// A request sent a byte at a time is dropped at its deadline: a read
/// timeout alone starts over with every byte, and the tailnet reaches this.
#[test]
fn a_tailnet_request_that_trickles_in_is_dropped_at_its_deadline() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let at = listener.local_addr().expect("addr");
    let sender = std::thread::spawn(move || {
        let mut client = TcpStream::connect(at).expect("connect");
        let _ = client.write_all(b"GET / HTTP/1.1\r\n");
        for _ in 0..60 {
            if client.write_all(b"x").is_err() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    });
    let (stream, _) = listener.accept().expect("accept");
    let started = Instant::now();
    assert!(request_of(&stream, Duration::from_millis(300)).is_none());
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
    drop(stream);
    let _ = sender.join();
}

/// What a new connection hears first, `in` when it is served and nothing
/// when it is closed unserved, and the connection, still open.
fn heard(at: SocketAddr) -> (Vec<u8>, TcpStream) {
    let mut client = TcpStream::connect(at).expect("connect");
    let _ = client.set_read_timeout(Some(Duration::from_secs(5)));
    let mut got = [0u8; 2];
    let read = client.read(&mut got).unwrap_or(0);
    (got[..read].to_vec(), client)
}

/// Past the most connections served at once, a new one is closed unserved
/// until one of them ends.
#[test]
fn a_tailnet_connection_past_the_most_is_closed_until_one_ends() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let at = listener.local_addr().expect("addr");
    let stop = Arc::new(AtomicBool::new(false));
    taking(listener, stop.clone(), 2, |mut stream: TcpStream, _| {
        let _ = stream.set_nonblocking(false);
        let _ = stream.write_all(b"in");
        let _ = stream.read(&mut [0u8; 1]);
    });
    let (first, one) = heard(at);
    let (second, _two) = heard(at);
    assert_eq!(
        (first.as_slice(), second.as_slice()),
        (&b"in"[..], &b"in"[..])
    );
    assert_eq!(heard(at).0, b"", "a third is closed while two are open");
    drop(one);
    let until = Instant::now() + Duration::from_secs(5);
    while heard(at).0 != b"in" {
        assert!(Instant::now() < until, "a closed one frees its place");
        std::thread::sleep(Duration::from_millis(50));
    }
    stop.store(true, Ordering::SeqCst);
}
