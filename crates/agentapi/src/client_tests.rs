use std::io::{Read, Write};
use std::net::TcpListener;

use super::*;

#[test]
fn the_address_comes_out_of_the_endpoint_the_hooks_use() {
    assert_eq!(
        address_of("http://127.0.0.1:4321/hook").as_deref(),
        Some("127.0.0.1:4321")
    );
    assert_eq!(address_of("ftp://x/hook"), None);
    assert_eq!(address_of("http:///hook"), None);
}

#[test]
fn a_devpit_that_is_not_open_is_said_plainly() {
    let dir = tempfile::tempdir().expect("tempdir");
    let why = ask(dir.path(), "context", json!({}), dir.path()).expect_err("closed");
    assert!(why.contains("not open"), "{why}");
}

/// The whole round trip against a listener that behaves like the app's:
/// the secret travels as the header, the path is `/agent`, the answer is
/// unwrapped.
#[test]
fn a_question_carries_the_secret_and_gets_the_answer() {
    let dir = tempfile::tempdir().expect("tempdir");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    std::fs::write(
        dir.path().join("hook-endpoint"),
        format!("http://127.0.0.1:{port}/hook"),
    )
    .expect("write");
    std::fs::write(dir.path().join("hook-auth"), "x-devpit-hook: s3cret\n").expect("write");

    let served = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut seen = vec![0u8; 4096];
        let read = stream.read(&mut seen).expect("read");
        let request = String::from_utf8_lossy(&seen[..read]).into_owned();
        let body = r#"{"ok":{"project":"app"}}"#;
        let _ = stream.write_all(
            format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{body}",
                body.len()
            )
            .as_bytes(),
        );
        request
    });

    let answer = ask(dir.path(), "context", json!({}), Path::new("/work")).expect("answer");
    assert_eq!(answer, json!({ "project": "app" }));
    let request = served.join().expect("served");
    assert!(request.starts_with("POST /agent "), "{request}");
    assert!(request.contains("x-devpit-hook: s3cret"), "{request}");
    assert!(request.contains(r#""cwd":"/work""#), "{request}");
}
