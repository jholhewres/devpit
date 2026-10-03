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

/// The agent's process ending or missing is the agent's, so a turn it cut
/// short answers with a code the error report does not keep.
#[test]
fn a_turn_the_agent_cut_short_is_not_an_internal_error() {
    use devpit_agentcli::AgentError;
    use devpit_rpc::ErrorCode;

    let missing = super::turn_refused(AgentError::NotInstalled);
    let ended = super::turn_refused(AgentError::Unreadable(
        "the conversation's process ended".to_owned(),
    ));

    assert_eq!(missing.code, ErrorCode::NotFound);
    assert_eq!(ended.code, ErrorCode::Busy);
    assert!(ended.message.contains("process ended"), "{}", ended.message);
}
