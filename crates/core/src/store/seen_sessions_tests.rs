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

#[test]
fn a_card_has_the_sessions_seen_in_its_checkout_of_every_account_running_or_ended() {
    let (_dir, store) = store();
    let on = |id, card| SessionSeen {
        card_id: card,
        ..seen(id, id)
    };
    store
        .saw_sessions(
            "claude",
            &[
                on("s1", Some("card_a")),
                on("s2", Some("card_b")),
                on("s3", None),
            ],
            10,
        )
        .expect("saw");
    store
        .saw_sessions("work", &[on("s4", Some("card_a"))], 20)
        .expect("saw");
    // s1 ends; still the card's.
    store.saw_sessions("claude", &[], 30).expect("saw");

    let ids: Vec<_> = store
        .sessions_on_card("card_a", 10)
        .expect("read")
        .into_iter()
        .map(|one| (one.session_id, one.profile_id, one.ended_at))
        .collect();
    assert_eq!(
        ids,
        [
            ("s4".into(), "work".into(), None),
            ("s1".into(), "claude".into(), Some(30))
        ]
    );
    assert!(store
        .sessions_on_card("card_c", 10)
        .expect("read")
        .is_empty());
}

#[test]
fn a_poll_that_saw_what_is_kept_takes_no_write_lock() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("state.db");
    let store = Store::open(&path).expect("open");
    store
        .saw_sessions("claude", &[seen("s1", "api")], 10)
        .expect("saw");
    // Held as a write stuck on the disk holds it; short, so a regression fails fast.
    let other = Store::open(&path).expect("open");
    other.conn().execute_batch("BEGIN IMMEDIATE").expect("lock");
    store
        .conn()
        .busy_timeout(std::time::Duration::from_millis(50))
        .expect("timeout");

    let polled = store.saw_sessions("claude", &[seen("s1", "api")], 20);
    other.conn().execute_batch("ROLLBACK").expect("unlock");
    assert!(
        polled.is_ok(),
        "a poll that saw nothing new waited to write: {polled:?}"
    );
}

#[test]
fn only_what_changed_or_went_stale_is_written_again() {
    let (_dir, store) = store();
    store
        .saw_sessions("claude", &[seen("s1", "api")], 10)
        .expect("saw");
    let kept = store.open_sessions("claude").expect("kept");
    let fresh = 10 + SEEN_REFRESH_SECS - 1;

    assert!(already_kept(&kept, &[seen("s1", "api")], fresh));
    assert!(!already_kept(&kept, &[seen("s1", "api")], fresh + 1));
    assert!(!already_kept(&kept, &[seen("s1", "renamed")], fresh));
    let idle = SessionSeen {
        status: "idle",
        ..seen("s1", "api")
    };
    assert!(!already_kept(&kept, &[idle], fresh));
    assert!(!already_kept(
        &kept,
        &[seen("s1", "api"), seen("s2", "web")],
        fresh
    ));
    assert!(!already_kept(&kept, &[], fresh));

    store
        .saw_sessions("claude", &[seen("s1", "api")], fresh + 1)
        .expect("saw");
    let seen_at = store
        .seen_session("claude", "s1")
        .expect("read")
        .map(|one| one.last_seen_at);
    assert_eq!(seen_at, Some(fresh + 1));
}
