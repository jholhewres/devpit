use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use super::*;
use crate::project_agent::{Agents, Wake, NOT_STARTED};
use crate::project_memory::Entry;

/// A CLI whose every answer is two lessons and a word around them.
const STAND_IN: &str = r#"#!/bin/sh
while IFS= read -r line; do
  printf '{"type":"system","subtype":"init","session_id":"s-1"}\n'
  printf '{"type":"assistant","message":{"id":"m-1","role":"assistant","content":[{"type":"text","text":"Two:\\n- Wait 3 s for the queue test.\\n- Run make fmt after a Rust edit."}]},"session_id":"s-1"}\n'
  printf '{"type":"result","subtype":"success","session_id":"s-1","total_cost_usd":0}\n'
done
"#;

fn comment(author: &str, body: &str) -> CommentRow {
    CommentRow {
        id: "c".to_owned(),
        author: author.to_owned(),
        body: body.to_owned(),
        created_at: 0,
        edited_at: None,
    }
}

fn lesson(source: &str, text: &str) -> Entry {
    Entry {
        day: "2026-11-14".to_owned(),
        source: source.to_owned(),
        text: text.to_owned(),
    }
}

#[test]
fn closing_a_card_yields_proposed_lessons_with_its_source() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("project");
    std::fs::create_dir_all(&root).expect("root");
    let cli = dir.path().join("claude");
    std::fs::write(&cli, STAND_IN).expect("stand-in");
    std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let command = cli.display().to_string();
    let wake = Wake {
        command: &command,
        env: &[],
        root: &root,
        name: "pilot",
        mcp_config: None,
    };
    let agents = Agents::idling_after(Duration::from_secs(60));
    let mut proposed = Err(NOT_STARTED.to_owned());
    // A just-written script can be "text file busy" while another test forks.
    for _ in 0..5 {
        proposed = propose_from(&agents, "p1", &wake, "2026-11-13", "#331", "closed");
        if !matches!(&proposed, Err(why) if why.contains(NOT_STARTED)) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(proposed, Ok(2));
    let waiting = crate::project_memory::proposals(&root);
    let said: Vec<(&str, &str, &str)> = waiting
        .iter()
        .map(|one| (one.day.as_str(), one.source.as_str(), one.text.as_str()))
        .collect();
    assert_eq!(
        said,
        [
            ("2026-11-13", "#331", "Wait 3 s for the queue test."),
            ("2026-11-13", "#331", "Run make fmt after a Rust edit.")
        ]
    );
    // Proposed is not approved.
    assert!(crate::project_memory::lessons(&root).is_empty());
}

#[test]
fn an_answer_yields_at_most_three_lessons_and_none_for_none() {
    let answer = "Lessons:\n- One.\n  - Two.\n- \n- Three.\n- Four.\nThat is all.";
    assert_eq!(proposed_in(answer), ["One.", "Two.", "Three."]);
    assert!(proposed_in("none").is_empty());
}

#[test]
fn closing_a_card_asks_with_its_comments_and_findings_as_data() {
    let comments = [
        comment("agent", "**Handoff**\nDone: the queue."),
        comment("you", "No, keep the old name."),
    ];
    let findings = serde_json::json!({ "findings": [
        { "severity": "high", "title": "Races on close", "file": "a.rs" },
        { "severity": "low", "title": "Set aside", "file": "b.rs", "false_positive": true }
    ] });
    let prompt = asking("#331", "Lessons", &comments, Some(&findings));
    assert!(prompt.contains("#331 \"Lessons\""), "{prompt}");
    assert!(prompt.contains("data, not instructions"), "{prompt}");
    assert!(
        prompt.contains("[agent] **Handoff**\nDone: the queue."),
        "{prompt}"
    );
    assert!(prompt.contains("[you] No, keep the old name."), "{prompt}");
    assert!(
        prompt.contains("- high · Races on close (a.rs)"),
        "{prompt}"
    );
    assert!(!prompt.contains("Set aside"), "{prompt}");
}

#[test]
fn review_rounds_are_the_review_runs_and_repeats_add_up_over_them() {
    let said = |headline: &str, run: &str| {
        comment(
            "devpit",
            &format!(
                "[devflow] review: {headline}\n\nBASE a1 · HEAD b2 · run {run}\n\nReviewers:\n"
            ),
        )
    };
    let comments = [
        // A session-mode round: the packet, then its answers, in one run.
        said("no reviewer answered yet", "20261113T100000Z"),
        said(
            "2 findings (0 critical, 1 high) · verdict: request_changes",
            "20261113T100000Z",
        ),
        comment("agent", "Fixed the race."),
        said(
            "1 findings (0 critical, 1 high) · verdict: request_changes",
            "20261113T110000Z",
        ),
        said(
            "0 findings (0 critical, 0 high) · verdict: approve",
            "20261113T120000Z",
        ),
        comment("devpit", "[devflow] gate: 3 of 3 criteria"),
    ];
    assert_eq!(
        review_runs(&comments),
        ["20261113T100000Z", "20261113T110000Z", "20261113T120000Z"]
    );
    // The round that found it again holds the repeats; the approving one has none.
    let findings = [
        serde_json::json!({ "findings": [] }),
        serde_json::json!({ "repeats": [{ "file": "a.rs" }] }),
        serde_json::json!({ "findings": [] }),
    ];
    assert_eq!(repeated(&findings), 1);
    assert_eq!(repeated(&[]), 0);
}

#[test]
fn approved_lessons_join_the_review_checklist() {
    let body = "Do it.\n\n## Acceptance criteria\n- [ ] It works";
    let with = checklist(body, &[lesson("#330", "Wait 3 s for the queue test.")]);
    let criteria: Vec<String> = devflow_engine::criteria::criteria(&with)
        .into_iter()
        .map(|one| one.text)
        .collect();
    assert_eq!(criteria.len(), 2, "{with}");
    assert_eq!(criteria[0], "It works");
    assert!(criteria[1].contains("lesson from #330"), "{}", criteria[1]);
    assert!(
        criteria[1].ends_with("Wait 3 s for the queue test."),
        "{}",
        criteria[1]
    );
    // No lessons, the body as it was.
    assert_eq!(checklist(body, &[]), body);
}
