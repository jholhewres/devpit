use devpit_rpc::{LiveSession, PendingPrompt, PromptOption, Routine, RoutineKind};

use super::{beats, carried, due, news, notices, stuck, told, Finding};
use crate::channels::Action;
use crate::schedules::Kept;

/// Friday 2026-11-13, 12:00 UTC.
const NOON: i64 = 1_794_571_200;
const MINUTE: i64 = 60;

fn finding(key: &str) -> Finding {
    Finding {
        key: key.to_owned(),
        line: format!("{key} needs you"),
        asking: None,
    }
}

#[test]
fn with_assistant_mode_off_no_heartbeat_runs() {
    assert!(!beats(false, None, 15, NOON));
    assert!(!beats(false, Some(NOON - 60 * MINUTE), 15, NOON));
    assert!(beats(true, None, 15, NOON), "on, the first beat is now");
    assert!(!beats(true, Some(NOON - 14 * MINUTE), 15, NOON));
    assert!(beats(true, Some(NOON - 15 * MINUTE), 15, NOON));
}

#[test]
fn the_heartbeat_is_silent_when_nothing_needs_attention_and_speaks_once_when_something_does() {
    assert!(news(&[], &[]).is_empty(), "nothing found is silence");
    let ci = [finding("ci:api#12")];
    assert_eq!(news(&[], &ci), [&ci[0]], "something found is said");
    let said = vec!["ci:api#12".to_owned()];
    assert!(news(&said, &ci).is_empty(), "and said once");
    let more = [finding("ci:api#12"), finding("blocked:card_7")];
    assert_eq!(news(&said, &more), [&more[1]], "only what is new");
}

#[test]
fn a_routine_runs_at_its_time_once_and_not_while_paused() {
    let routine = Routine {
        kind: RoutineKind::Ci,
        minute: 9 * 60,
        days: Vec::new(),
        zone: "America/Sao_Paulo".to_owned(),
        offset_minutes: -180,
    };
    let mut all = vec![Kept {
        id: "routine_1".to_owned(),
        routine,
        paused: false,
        last_run: None,
        next_at: Some(NOON),
    }];
    assert!(due(&mut all, NOON - 1).is_empty(), "not yet");
    assert_eq!(due(&mut all, NOON), [(RoutineKind::Ci, None)]);
    assert_eq!(all[0].last_run, Some(NOON));
    assert_eq!(
        all[0].next_at,
        Some(NOON + 24 * 60 * MINUTE),
        "tomorrow at nine"
    );
    assert!(due(&mut all, NOON + MINUTE).is_empty(), "once");

    all[0].paused = true;
    assert!(
        due(&mut all, NOON + 25 * 60 * MINUTE).is_empty(),
        "paused, it does not run"
    );
}

fn waiting(name: &str, minutes_ago: i64) -> LiveSession {
    LiveSession {
        name: name.to_owned(),
        pid: 7,
        job: None,
        status: "idle".to_owned(),
        kind: "interactive".to_owned(),
        cwd: "/w".to_owned(),
        project_id: Some("api".to_owned()),
        project_name: None,
        card_id: None,
        since: Some(((NOON - minutes_ago * MINUTE) * 1000) as f64),
        in_devpit: true,
        waiting: Some(PendingPrompt {
            question: "Drop the staging database?".to_owned(),
            options: vec![
                PromptOption {
                    label: "Yes".to_owned(),
                    hint: None,
                },
                PromptOption {
                    label: "No".to_owned(),
                    hint: None,
                },
            ],
            cursor: 0,
        }),
        pane: None,
        session_id: None,
        step: None,
        draft: None,
        remote: Default::default(),
    }
}

#[test]
fn a_session_stuck_on_a_question_longer_than_the_setting_needs_attention() {
    let mut deploy = waiting("deploy", 11);
    deploy.pane = Some(devpit_rpc::LivePane {
        project_id: "api".to_owned(),
        pane_id: "leaf_deploy".to_owned(),
    });
    let live = [deploy, waiting("tests", 5)];
    let found = stuck(&live, 10, NOON, |pane| {
        Some(format!("tab_{}", pane.pane_id))
    });
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].line,
        "deploy has waited 11 min on: Drop the staging database?"
    );
    // Its terminal goes with it: the notice's buttons answer it there.
    assert_eq!(found[0].asking.as_deref(), Some("tab_leaf_deploy"));
}

/// The heartbeat's notice of a stuck session carries its terminal, so its
/// question goes to Telegram with the choices as the same buttons the
/// Sessions panel answers with: a press is the person's answer.
#[test]
fn a_question_answered_from_telegram_goes_through_the_existing_buttons() {
    let (_, notice) = told(
        "Needs you",
        &["deploy has waited 11 min".to_owned()],
        Some("tab_deploy".to_owned()),
    );
    assert_eq!(notice.asking.as_deref(), Some("tab_deploy"));
    let seen = waiting("deploy", 11).waiting.expect("a question");
    let read_off_the_screen = |target: Option<&str>| -> Vec<Action> {
        assert_eq!(target, Some("tab_deploy"));
        vec![Action::Answer {
            target: "tab_deploy".to_owned(),
            seen: seen.clone(),
            choice: 1,
        }]
    };
    let (text, buttons) =
        crate::channels::message(&[&notice], true, read_off_the_screen).expect("a message");
    assert!(text.contains("deploy has waited 11 min"));
    let key = crate::channel_actions::remember(&buttons[0]);
    let pressed = crate::channel_actions::take(&key).expect("the press");
    assert_eq!(
        crate::channel_actions::label(&pressed),
        crate::channel_actions::label(&buttons[0])
    );
    assert!(
        matches!(&pressed, Action::Answer { target, choice: 1, .. } if target == "tab_deploy"),
        "{pressed:?}"
    );
}

/// A source that could not be read keeps what the last beat found of it, so
/// the next good look does not say it again.
#[test]
fn a_look_that_could_not_read_ci_does_not_make_a_failure_news_again() {
    let last = vec!["ci:api#12".to_owned(), "blocked:card_7".to_owned()];
    let kept = carried(&last, &[], &["ci:"]);
    assert_eq!(
        kept,
        ["ci:api#12"],
        "gh timed out: the failure is still known"
    );
    assert!(news(&kept, &[finding("ci:api#12")]).is_empty());
    // Read and gone, it may come back as news later.
    assert!(carried(&last, &[], &[]).is_empty());
    // Found again while another project could not be read: kept once, beat after beat.
    let mut keys = last.clone();
    for _ in 0..5 {
        keys = carried(&keys, &[finding("ci:api#12")], &["ci:"]);
    }
    assert_eq!(keys, ["ci:api#12"]);
}

/// Two sessions stuck at once make two notices, each with its own question's
/// buttons; the rest go together without any.
#[test]
fn each_question_goes_alone_with_its_own_buttons() {
    let mut deploy = finding("waiting:deploy");
    deploy.asking = Some("tab_deploy".to_owned());
    let mut tests = finding("waiting:tests");
    tests.asking = Some("tab_tests".to_owned());
    let ci = finding("ci:api#12");
    let said = notices(&[&deploy, &ci, &tests]);
    assert_eq!(
        said,
        [
            (
                vec!["waiting:deploy needs you".to_owned()],
                Some("tab_deploy".to_owned())
            ),
            (
                vec!["waiting:tests needs you".to_owned()],
                Some("tab_tests".to_owned())
            ),
            (vec!["ci:api#12 needs you".to_owned()], None),
        ]
    );
}
