use super::activity_in;

#[test]
fn the_remote_log_reads_latest_first_and_skips_what_it_cannot_read() {
    let log = concat!(
        r#"{"at":1.0,"device":"Pixel","id":"d1","what":"watching leaf_1"}"#,
        "\n",
        "half a line\n",
        r#"{"at":2.0,"device":"iPad","id":"d2","what":"sent a draft to api-a"}"#,
        "\n",
    );
    let read = activity_in(log, 10);
    let said: Vec<_> = read
        .iter()
        .map(|one| (one.at, one.device.as_str(), one.what.as_str()))
        .collect();
    assert_eq!(
        said,
        [
            (2.0, "iPad", "sent a draft to api-a"),
            (1.0, "Pixel", "watching leaf_1")
        ]
    );
    assert_eq!(activity_in(log, 1).len(), 1);
}
