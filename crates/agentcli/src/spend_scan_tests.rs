use std::time::{Duration, SystemTime};

use super::*;

fn line(id: &str, request: &str, at: &str, output: u64) -> String {
    format!(
        r#"{{"type":"assistant","timestamp":"{at}","sessionId":"s1","cwd":"/w/p","requestId":"{request}","uuid":"u-{id}-{output}","message":{{"id":"{id}","model":"claude-opus-5","usage":{{"input_tokens":2,"output_tokens":{output},"cache_read_input_tokens":100,"cache_creation_input_tokens":30,"cache_creation":{{"ephemeral_5m_input_tokens":20,"ephemeral_1h_input_tokens":10}}}}}}}}"#
    )
}

#[test]
fn a_message_written_on_several_lines_is_one_record_per_line_until_deduplicated() {
    let text = [
        line("m1", "r1", "2026-09-07T11:17:43.176Z", 5),
        line("m1", "r1", "2026-09-07T11:17:44.000Z", 5),
        r#"{"type":"user","message":{"content":"hi"}}"#.to_owned(),
        r#"{"type":"assistant","timestamp":"2026-09-07T11:18:00Z","requestId":"r0","message":{"id":"m0","model":"<synthetic>","usage":{"input_tokens":0,"output_tokens":0}}}"#.to_owned(),
    ]
    .join("\n");
    let records = records_in(&text);
    assert_eq!(
        records.len(),
        2,
        "two lines of one message, no user line, no empty one"
    );
    assert_eq!(records[0].key, "m1:r1");
    assert_eq!(records[0].key, records[1].key);
    assert_eq!(
        (records[0].cache_write_5m, records[0].cache_write_1h),
        (20, 10)
    );
    assert_eq!(records[0].tokens(), 2 + 5 + 100 + 30);
    assert_eq!(records[0].session_id, "s1");
    assert_eq!(records[0].model, "claude-opus-5");
}

#[test]
fn times_read_as_utc_seconds_and_days() {
    assert_eq!(epoch_of("1970-01-02T00:00:00Z"), Some(86_400));
    assert_eq!(
        epoch_of("2026-09-07T11:17:43.176Z"),
        epoch_of("2026-09-07T08:17:43-03:00")
    );
    assert_eq!(
        day_of(epoch_of("2026-02-28T23:59:59Z").expect("time") + 1),
        "2026-03-01"
    );
    assert_eq!(
        day_of(epoch_of("2024-02-29T12:00:00Z").expect("leap")),
        "2024-02-29"
    );
    assert_eq!(epoch_of("not a time"), None);
}

#[test]
fn transcripts_are_found_in_every_folder_and_old_ones_are_skipped() {
    let dir = tempfile::tempdir().expect("tempdir");
    let nested = dir.path().join("projects/-w-p/s1/subagents");
    std::fs::create_dir_all(&nested).expect("folders");
    std::fs::write(dir.path().join("projects/-w-p/s1.jsonl"), "").expect("main");
    std::fs::write(nested.join("agent-a.jsonl"), "").expect("subagent");
    std::fs::write(dir.path().join("projects/-w-p/notes.txt"), "").expect("other");
    let old = dir.path().join("projects/-w-p/old.jsonl");
    std::fs::write(&old, "").expect("old");
    let long_ago = SystemTime::now() - Duration::from_secs(40 * 86_400);
    std::fs::File::options()
        .write(true)
        .open(&old)
        .expect("open")
        .set_modified(long_ago)
        .expect("age it");

    let since = SystemTime::now() - Duration::from_secs(7 * 86_400);
    let found: Vec<String> = transcripts(dir.path(), since)
        .iter()
        .map(|path| {
            path.strip_prefix(dir.path())
                .expect("inside")
                .display()
                .to_string()
        })
        .collect();
    assert_eq!(
        found,
        [
            "projects/-w-p/s1/subagents/agent-a.jsonl",
            "projects/-w-p/s1.jsonl"
        ]
    );
}
