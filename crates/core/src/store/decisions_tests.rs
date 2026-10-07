use super::*;

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    (dir, store)
}

fn decision(outcome: &str, cost_usd: f64) -> DecisionWrite<'_> {
    DecisionWrite {
        project_id: Some("prj_api"),
        card_id: None,
        gate: "mcp.agent",
        mode: "shadow",
        rubric: None,
        rubric_version: None,
        questions: r#"{"done":{"type":"noul","instructions":"Is it done?"}}"#,
        answers: Some(r#"{"done":{"type":"noul","noul":0.9}}"#),
        thresholds: None,
        outcome,
        cost_usd,
        latency_ms: 120,
        state_digest: "abc:10",
    }
}

#[test]
fn a_decision_is_kept_and_read_back_without_its_state() {
    let (_dir, store) = store();
    let id = store
        .log_decision(&decision("answered", 0.002), 1_000)
        .expect("log");
    let kept = store.decisions_latest(10).expect("read");
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].id, id);
    assert_eq!(kept[0].outcome, "answered");
    assert_eq!(kept[0].state_digest, "abc:10");
}

#[test]
fn the_spend_counts_only_what_was_decided_since() {
    let (_dir, store) = store();
    store
        .log_decision(&decision("answered", 0.25), 100)
        .expect("log");
    store
        .log_decision(&decision("pass", 0.5), 1_000)
        .expect("log");
    store
        .log_decision(&decision("skipped", 0.0), 1_001)
        .expect("log");
    let (spent, answered) = store.decision_spend_since(1_000).expect("spend");
    assert!((spent - 0.5).abs() < 1e-9, "{spent}");
    // A skip cost nothing and decided nothing.
    assert_eq!(answered, 1);
}

#[test]
fn a_decision_past_the_retention_is_pruned_on_the_next_write() {
    let (_dir, store) = store();
    store
        .log_decision(&decision("answered", 0.1), 0)
        .expect("log");
    let later = DECISIONS_KEPT_DAYS * 86_400 + 1;
    store
        .log_decision(&decision("answered", 0.1), later)
        .expect("log");
    let kept = store.decisions_latest(10).expect("read");
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].at, later);
}
