use super::*;

#[test]
fn a_range_is_read_as_a_span_within_the_file_and_one_chunk() {
    assert_eq!(span("bytes=0-99", 1000), Some((0, 99)));
    assert_eq!(span("bytes=900-", 1000), Some((900, 999)));
    assert_eq!(span("bytes=-100", 1000), Some((900, 999)));
    assert_eq!(span("bytes=0-5000", 1000), Some((0, 999)));
    assert_eq!(span("bytes=0-", 10 * CHUNK), Some((0, CHUNK - 1)));
    assert_eq!(span("bytes=1000-", 1000), None);
    assert_eq!(span("bytes=5-1", 1000), None);
    assert_eq!(span("bytes=0-1,5-9", 1000), None);
    assert_eq!(span("items=0-1", 1000), None);
}

#[test]
fn a_token_serves_its_file_in_ranges_and_nothing_else() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("clip.mp4");
    let mut bytes = b"\x00\x00\x00\x20ftypisom".to_vec();
    bytes.extend((0..1000u32).map(|n| (n % 251) as u8));
    std::fs::write(&path, &bytes).expect("write");
    let token = token_for(path.clone()).expect("token");
    assert_eq!(
        token_for(path).expect("again"),
        token,
        "one file, one token"
    );

    let part = serve(&format!("/{token}"), Some("bytes=4-11"));
    assert_eq!(part.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(part.body().as_slice(), b"ftypisom");
    assert_eq!(
        part.headers()["content-range"],
        format!("bytes 4-11/{}", bytes.len())
    );
    assert_eq!(part.headers()["content-type"], "video/mp4");

    let whole = serve(&format!("/{token}"), None);
    assert_eq!(
        (whole.status(), whole.body().len()),
        (StatusCode::OK, bytes.len())
    );

    assert_eq!(
        serve(&format!("/{token}"), Some("bytes=99999-")).status(),
        StatusCode::RANGE_NOT_SATISFIABLE
    );
    assert_eq!(serve("/not-a-token", None).status(), StatusCode::NOT_FOUND);
    assert_eq!(
        serve("/../../etc/passwd", None).status(),
        StatusCode::NOT_FOUND
    );
}
