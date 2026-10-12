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

#[test]
fn a_decision_is_labelled_by_what_the_person_did_and_read_back_by_card_and_project() {
    let (_dir, store) = store();
    let done = |card: &'static str, outcome: &'static str| DecisionWrite {
        card_id: Some(card),
        gate: "devflow-done",
        rubric: Some("devflow-done"),
        thresholds: Some(r#"{"criterion1":{"min":0.85}}"#),
        ..decision(outcome, 0.001)
    };
    let older = store
        .log_decision(&done("card_1", "pass"), 1_000)
        .expect("log");
    let newer = store
        .log_decision(&done("card_1", "fail"), 2_000)
        .expect("log");
    store
        .log_decision(&done("card_2", "pass"), 3_000)
        .expect("log");
    store
        .log_decision(&done("card_1", "skipped"), 4_000)
        .expect("log");

    let of_card = store
        .decisions_of_card("card_1", "devflow-done")
        .expect("read");
    let ids: Vec<&str> = of_card.iter().map(|row| row.id.as_str()).collect();
    assert_eq!(
        ids,
        [newer.as_str(), older.as_str()],
        "newest first, and no skip"
    );
    assert_eq!(of_card[0].person_verdict, None);
    assert!(of_card[0].thresholds.is_some());

    assert!(store
        .label_decision(&older, "fail", "reopened")
        .expect("label"));
    assert!(!store
        .label_decision("dcn_none", "fail", "reopened")
        .expect("label"));
    let of_project = store.decisions_of_project("prj_api").expect("read");
    assert_eq!(of_project.len(), 3);
    let labelled = of_project
        .iter()
        .find(|row| row.id == older)
        .expect("older");
    assert_eq!(labelled.person_verdict.as_deref(), Some("fail"));
}
