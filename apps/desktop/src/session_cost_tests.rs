use super::*;

/// Found in whichever installation wrote it, and read on as it grows.
#[test]
fn a_session_is_found_by_its_id_and_read_as_it_grows() {
    let home = tempfile::tempdir().expect("tempdir");
    let config = home.path().join(".claude");
    let folder = config.join("projects").join("-work-demo");
    std::fs::create_dir_all(&folder).expect("dirs");
    let id = "0f1e2d3c-test-session";
    let line = |msg: &str, out: u64| {
        format!(
            r#"{{"type":"assistant","requestId":"r-{msg}","sessionId":"{id}","cwd":"/w","timestamp":"2026-10-05T10:00:00Z","message":{{"id":"{msg}","model":"claude-sonnet-4-5","usage":{{"input_tokens":10,"output_tokens":{out}}}}}}}"#
        )
    };
    std::fs::write(
        folder.join(format!("{id}.jsonl")),
        format!("{}\n", line("m1", 100)),
    )
    .expect("write");

    let configs = vec![PathBuf::from("/nowhere"), config];
    let first = cost_of(&configs, id).expect("a cost");
    assert_eq!(first.tokens.output, 100.0);
    assert_eq!(first.since_seen_usd, 0.0, "history counted as spent here");

    let mut more = std::fs::read_to_string(folder.join(format!("{id}.jsonl"))).expect("read");
    more.push_str(&format!("{}\n", line("m2", 50)));
    std::fs::write(folder.join(format!("{id}.jsonl")), more).expect("write");
    let then = cost_of(&configs, id).expect("a cost");
    assert_eq!(then.tokens.output, 150.0);
    assert!(then.since_seen_usd > 0.0);
}

#[test]
fn an_id_that_is_not_one_is_not_a_path() {
    assert!(cost_of(&[PathBuf::from("/")], "../../etc/passwd").is_none());
    assert!(cost_of(&[PathBuf::from("/")], "").is_none());
}
