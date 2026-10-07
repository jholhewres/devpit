use super::*;

fn seen<'a>(id: &'a str, name: &'a str) -> SessionSeen<'a> {
    SessionSeen {
        session_id: id,
        name,
        cwd: "/w/api",
        project_id: Some("prj_api"),
        card_id: None,
        status: "busy",
    }
}

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    (dir, store)
}

#[test]
fn a_session_no_longer_running_has_ended_and_is_kept() {
    let (_dir, store) = store();
    store
        .saw_sessions("claude", &[seen("s1", "api-a"), seen("s2", "api-b")], 10)
        .expect("saw");
    store
        .saw_sessions("claude", &[seen("s2", "api-b")], 20)
        .expect("saw");

    let ended = store.ended_sessions("claude", 10).expect("ended");
    assert_eq!(ended.len(), 1);
    assert_eq!(
        (
            ended[0].name.as_str(),
            ended[0].ended_at,
            ended[0].ended_by.as_deref()
        ),
        ("api-a", Some(20), None)
    );
    assert_eq!(ended[0].last_status.as_deref(), Some("busy"));
    assert_eq!(ended[0].cwd, "/w/api");
}

#[test]
fn ended_sessions_say_who_stopped_them_and_come_back_when_resumed() {
    let (_dir, store) = store();
    store
        .saw_sessions("claude", &[seen("s1", "api-a"), seen("s2", "api-b")], 10)
        .expect("saw");
    store
        .session_stopped("s1", "orchestrator", 15)
        .expect("stopped");
    store
        .saw_sessions("claude", &[seen("s2", "api-b")], 20)
        .expect("saw");
    store.saw_sessions("claude", &[], 30).expect("saw");

    let ended = store.ended_sessions("claude", 10).expect("ended");
    let how: Vec<_> = ended
        .iter()
        .map(|one| (one.name.as_str(), one.ended_at, one.ended_by.as_deref()))
        .collect();
    assert_eq!(
        how,
        [
            ("api-b", Some(30), None),
            ("api-a", Some(15), Some("orchestrator"))
        ]
    );

    store
        .saw_sessions("claude", &[seen("s1", "api-a")], 40)
        .expect("resumed");
    assert_eq!(store.ended_sessions("claude", 10).expect("ended").len(), 1);
}

#[test]
fn a_kept_session_is_found_by_id_or_by_its_latest_name() {
    let (_dir, store) = store();
    store
        .saw_sessions("claude", &[seen("s1", "api")], 10)
        .expect("saw");
    store
        .saw_sessions("claude", &[seen("s2", "api")], 20)
        .expect("saw");
    assert_eq!(
        store
            .seen_session("claude", "api")
            .expect("read")
            .map(|one| one.session_id),
        Some("s2".into())
    );
    assert_eq!(
        store
            .seen_session("claude", "s1")
            .expect("read")
            .map(|one| one.session_id),
        Some("s1".into())
    );
    assert_eq!(store.seen_session("other", "api").expect("read"), None);
}
