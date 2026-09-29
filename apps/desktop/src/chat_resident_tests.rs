use super::evicted;

#[test]
fn a_reachable_chat_is_not_closed_by_another_conversation() {
    let listening = [
        ("orch_a", "orch"),
        ("remote_b", "orch"),
        ("elsewhere", "api"),
    ];
    let closed = evicted(listening.into_iter(), "orch", |key| key == "remote_b");
    assert_eq!(closed, ["orch_a"]);
}

#[test]
fn two_remote_control_chats_in_one_project_both_stay() {
    let listening = [("remote_a", "api")];
    assert!(evicted(listening.into_iter(), "api", |_| true).is_empty());
}
