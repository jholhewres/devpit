use devpit_rpc::{EndedSession, LiveSession, PendingPrompt};

use super::{rendered, Reminder};

fn session(name: &str) -> LiveSession {
    LiveSession {
        name: name.to_owned(),
        pid: 1,
        job: None,
        status: "busy".to_owned(),
        kind: "interactive".to_owned(),
        cwd: "/w".to_owned(),
        project_id: Some("p".to_owned()),
        project_name: Some("api".to_owned()),
        card_id: None,
        since: None,
        in_devpit: true,
        waiting: None,
        pane: None,
        session_id: None,
        step: None,
        draft: None,
    }
}

#[test]
fn what_waits_on_the_person_comes_first_and_says_what() {
    let mut asking = session("api-fix");
    asking.waiting = Some(PendingPrompt {
        question: "Run   the\nmigration?".to_owned(),
        options: Vec::new(),
        cursor: 0,
    });
    let mut drafted = session("web-ui");
    drafted.draft = Some("/remote-control".to_owned());
    let due = Reminder {
        project: "api".to_owned(),
        title: "review the PR".to_owned(),
        at: 0,
        went_off: true,
    };
    let text = rendered(0, &[asking, drafted], &[], &[due]);
    let waiting = text.find("## Waiting on the person").expect("section");
    let running = text.find("## Sessions running (2)").expect("section");
    assert!(waiting < running);
    assert!(text.contains("- api-fix asks: Run the migration?"));
    assert!(text.contains("- a draft for web-ui waits to be sent: «/remote-control»"));
    assert!(text.contains("- reminder went off (api): review the PR"));
}

#[test]
fn an_empty_round_says_so_rather_than_leaving_blanks() {
    let text = rendered(86_400 + 3_660, &[], &[], &[]);
    assert!(text.contains("Written by devpit at 1970-01-02 01:01 UTC"));
    assert_eq!(
        text.matches("Nothing.").count() + text.matches("None.").count(),
        4
    );
}

#[test]
fn an_ended_session_says_who_stopped_it_and_what_it_left() {
    let ended = EndedSession {
        session_id: "s".to_owned(),
        name: "old".to_owned(),
        cwd: "/w".to_owned(),
        project_id: Some("p".to_owned()),
        project_name: Some("api".to_owned()),
        card_id: None,
        started_at: 0.0,
        ended_at: 60_000.0,
        ended_by: Some("person".to_owned()),
        last_status: None,
        dirty: Some(3),
    };
    let text = rendered(120, &[], &[ended], &[]);
    assert!(text.contains(
        "- old — api, ended 1970-01-01 00:01 UTC, stopped by the person, 3 files not committed"
    ));
}
