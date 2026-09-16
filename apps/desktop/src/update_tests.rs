//! The three rules, each by branch.

use super::*;
use devpit_rpc::UpdateStatus as S;

#[test]
fn checking_happens_at_start_then_daily() {
    let day = 24.0 * 60.0 * 60.0;
    assert!(due(None, 1000.0, 0, true), "the first start never checked");
    assert!(!due(Some(1000.0), 1000.0 + day - 1.0, 0, true));
    assert!(due(Some(1000.0), 1000.0 + day, 0, true));
}

#[test]
fn a_failure_waits_an_hour_and_doubles_to_six() {
    let hour = 60.0 * 60.0;
    assert!(!due(Some(0.0), hour - 1.0, 1, true));
    assert!(due(Some(0.0), hour, 1, true));
    assert!(due(Some(0.0), 2.0 * hour, 2, true));
    assert!(!due(Some(0.0), 3.0 * hour, 3, true), "four hours to wait");
    // However many times it has failed, six hours is the longest wait.
    assert!(due(Some(0.0), 6.0 * hour, 9, true));
}

/// The switch is not advice: off means the network is never asked, at startup
/// or ever after.
#[test]
fn automatic_updates_off_never_checks() {
    assert!(!due(None, 1000.0, 0, false));
    assert!(!due(Some(0.0), 10.0 * 24.0 * 60.0 * 60.0, 0, false));
    assert!(!due(Some(0.0), 10.0 * 24.0 * 60.0 * 60.0, 5, false));
}

fn available() -> S {
    S::Available {
        version: "0.2.0".to_owned(),
        notes: "what changed".to_owned(),
        kind: InstallKind::AppImage,
        test_feed: false,
    }
}

#[test]
fn a_check_becomes_an_offer_or_nothing() {
    assert_eq!(next(&S::Idle, Event::Check), Some(S::Checking));
    assert_eq!(
        next(
            &S::Checking,
            Event::Found {
                version: "0.2.0".to_owned(),
                notes: "what changed".to_owned(),
                kind: InstallKind::AppImage,
                test_feed: false,
            }
        ),
        Some(available())
    );
    assert_eq!(next(&S::Checking, Event::NothingNewer), Some(S::Idle));
}

/// A build the machine looks after is told so rather than offered a download
/// it must not run.
#[test]
fn an_externally_managed_build_is_never_offered_a_download() {
    assert_eq!(
        next(
            &S::Checking,
            Event::Found {
                version: "0.2.0".to_owned(),
                notes: String::new(),
                kind: InstallKind::ExternallyManaged,
                test_feed: false,
            }
        ),
        Some(S::ExternallyManaged)
    );
}

/// The three refusals this table exists for.
#[test]
fn what_is_refused_is_refused_once() {
    // Checking again while a download runs.
    assert_eq!(next(&S::Downloading { percent: 20 }, Event::Check), None);
    // Anything at all once the installer has started, except its own failure.
    assert_eq!(next(&S::Installing, Event::Check), None);
    assert_eq!(next(&S::Installing, Event::NothingNewer), None);
    assert_eq!(
        next(
            &S::Installing,
            Event::Failed {
                message: "the installer refused".to_owned()
            }
        ),
        Some(S::Failed {
            message: "the installer refused".to_owned(),
            recoverable: false
        }),
        "a failure past the commit is not something to retry"
    );
}

#[test]
fn what_this_copy_is_by_branch() {
    let apt = vec!["/usr/bin/apt".to_owned()];
    assert_eq!(
        install_kind(Some("/opt/devpit.AppImage"), Some("appimage"), &[]),
        InstallKind::AppImage
    );
    // Stamped but not running as one: there is no file to replace.
    assert_eq!(
        install_kind(None, Some("appimage"), &[]),
        InstallKind::Unmanaged
    );
    // The review's case: an AppImage devpit puts APPIMAGE into every tmux
    // pane, and a .deb build started from one of them must stay a .deb.
    assert_eq!(
        install_kind(Some("/opt/devpit.AppImage"), Some("deb"), &apt),
        InstallKind::Deb
    );
    assert_eq!(
        install_kind(Some("/opt/devpit.AppImage"), None, &apt),
        InstallKind::Unmanaged,
        "APPIMAGE without the stamp is an inherited variable, not this build"
    );
    assert_eq!(install_kind(None, Some("deb"), &apt), InstallKind::Deb);
    assert_eq!(
        install_kind(None, Some("deb"), &[]),
        InstallKind::ExternallyManaged,
        "a deb on a machine with no package tools is somebody else's to update"
    );
    assert_eq!(install_kind(None, None, &apt), InstallKind::Unmanaged);
    assert_eq!(
        install_kind(Some(""), Some("deb"), &apt),
        InstallKind::Deb,
        "an empty APPIMAGE is not an AppImage"
    );
}

/// The lookup never reads `$PATH`: the command it feeds runs as root.
#[test]
fn tools_are_looked_up_only_where_root_writes() {
    for found in tools_found() {
        assert!(
            TRUSTED.iter().any(|dir| found.starts_with(dir)),
            "{found} came from somewhere else"
        );
    }
}

/// Three refusals, three sentences.
#[test]
fn what_may_be_downloaded_and_what_may_not() {
    assert_eq!(may_download(&available()), Ok(()));

    let deb = S::Available {
        version: "0.2.0".to_owned(),
        notes: String::new(),
        kind: InstallKind::Deb,
        test_feed: false,
    };
    assert_eq!(may_download(&deb), Ok(()));

    // A build nobody installs from here: `make dev`, a `cargo run`.
    let unmanaged = S::Available {
        version: "0.2.0".to_owned(),
        notes: String::new(),
        kind: InstallKind::Unmanaged,
        test_feed: false,
    };
    assert!(may_download(&unmanaged)
        .expect_err("unmanaged was allowed")
        .contains("release page"));

    // The fixture feed offers what nobody can install.
    let fixture = S::Available {
        version: "0.2.0".to_owned(),
        notes: String::new(),
        kind: InstallKind::AppImage,
        test_feed: true,
    };
    assert!(may_download(&fixture)
        .expect_err("a test feed was allowed")
        .contains("test feed"));

    // And the two states where a second download is a second update.
    assert!(may_download(&S::Downloading { percent: 10 }).is_err());
    assert!(may_download(&S::Installing).is_err());
    assert!(may_download(&S::Idle).is_err());
}

/// From `Ready` onward nothing new starts — and `Ready` is the point, not
/// `Waiting`: a card set to advance on its own (`advancing.rs`) would
/// otherwise start a run between the moment the person chose and the moment
/// the installer commits, and that run dies with the process.
#[test]
fn a_run_started_between_the_check_and_the_commit_is_refused() {
    let ready = S::Ready {
        version: "0.2.0".to_owned(),
    };
    // Downloaded is not decided: a closed card left every run refused until
    // a restart, so Ready alone refuses nothing.
    assert!(starting_refused(&ready).is_none());
    assert!(starting_refused(&S::Waiting {
        runs: 1,
        turns: 0,
        since: 0.0
    })
    .is_some());
    assert!(starting_refused(&S::Installing).is_some());

    // And before that, work goes on as usual: an offer nobody accepted yet
    // must not stop a person from starting something.
    assert!(starting_refused(&S::Idle).is_none());
    assert!(starting_refused(&available()).is_none());
    assert!(starting_refused(&S::Downloading { percent: 40 }).is_none());
}

#[test]
fn the_plan_follows_the_choice_and_what_is_running() {
    let nothing = devpit_rpc::UpdateWork::default();
    let busy = devpit_rpc::UpdateWork {
        runs: vec![devpit_rpc::UpdateBlocking {
            id: "run_1".to_owned(),
            title: "Wire the board".to_owned(),
        }],
        turns: Vec::new(),
        keeps: Vec::new(),
    };

    // Nothing in the way: the choice does not come up at all.
    assert_eq!(
        install_plan(Choice::WhenItIsDone, &nothing),
        Plan::InstallNow
    );
    assert_eq!(install_plan(Choice::StopIt, &nothing), Plan::InstallNow);

    assert_eq!(install_plan(Choice::WhenItIsDone, &busy), Plan::WaitForIdle);
    assert_eq!(install_plan(Choice::StopIt, &busy), Plan::StopThenInstall);

    // Later is later, whatever is running.
    assert_eq!(install_plan(Choice::Later, &busy), Plan::Later);
    assert_eq!(install_plan(Choice::Later, &nothing), Plan::Later);

    assert_eq!(blockers(&busy), 1);
    assert_eq!(blockers(&nothing), 0);
}

/// The steps an install takes, and the one that is never among them.
///
/// tmux is not in this list and must never be: the terminals outlive the
/// window on purpose, which is what makes it possible to install while an
/// agent is mid-task in a pane. `cargo xtask check` refuses the words
/// `kill_server`/`kill-server` anywhere under `apps/desktop`.
#[test]
fn the_update_path_never_kills_tmux() {
    let appimage = quit_steps(InstallKind::AppImage);
    assert_eq!(
        appimage,
        vec![QuitStep::AskTheWindow, QuitStep::Install, QuitStep::Restart]
    );

    // A package is shown, never installed from here.
    assert_eq!(
        quit_steps(InstallKind::Deb),
        vec![QuitStep::ShowTheCommand],
        "a .deb would mean asking for root"
    );

    // And nothing at all for a build nobody installs over.
    assert!(quit_steps(InstallKind::Unmanaged).is_empty());
    assert!(quit_steps(InstallKind::ExternallyManaged).is_empty());
}

/// The window says it is done, and the update goes on at once.
#[tokio::test]
async fn the_window_ack_releases_the_restart() {
    let ready = std::sync::Arc::new(tokio::sync::Notify::new());
    let answering = ready.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        answering.notify_one();
    });

    let waited = std::time::Instant::now();
    assert!(window_saved(&ready, std::time::Duration::from_secs(5)).await);
    assert!(
        waited.elapsed() < std::time::Duration::from_secs(1),
        "it waited for the deadline rather than for the window"
    );
}

/// And a window that never answers does not own the update.
#[tokio::test]
async fn a_silent_window_does_not_hold_the_update() {
    let ready = tokio::sync::Notify::new();
    assert!(!window_saved(&ready, std::time::Duration::from_millis(20)).await);
}

/// The regression: a download kept the bytes and dropped the update they
/// belong to, so install always answered "the update to install is no longer
/// known" and no AppImage ever updated itself.
#[test]
fn a_downloaded_update_is_still_there_to_install() {
    let found = std::sync::Mutex::new(None::<String>);
    let downloaded = std::sync::Mutex::new(None);

    keep_for_install(
        &found,
        &downloaded,
        "0.1.1".to_owned(),
        "0.1.1".to_owned(),
        vec![7, 7],
    );

    assert_eq!(
        found.lock().expect("found").take().as_deref(),
        Some("0.1.1")
    );
    assert_eq!(
        downloaded.lock().expect("downloaded").take(),
        Some(("0.1.1".to_owned(), vec![7, 7]))
    );
}

/// Work in the way, as a list a test can watch being stopped.
#[derive(Default)]
struct Fake {
    runs: std::sync::Mutex<Vec<String>>,
    turns: std::sync::Mutex<Vec<String>>,
    /// Everything that happened, in order.
    said: std::sync::Mutex<Vec<String>>,
    /// A turn that ignores its signal.
    stubborn: bool,
}

impl Fake {
    fn with(runs: &[&str], turns: &[&str]) -> Self {
        let owned = |ids: &[&str]| ids.iter().map(|id| id.to_string()).collect();
        Fake {
            runs: std::sync::Mutex::new(owned(runs)),
            turns: std::sync::Mutex::new(owned(turns)),
            ..Fake::default()
        }
    }

    fn said(&self) -> Vec<String> {
        self.said.lock().unwrap().clone()
    }
}

impl InTheWay for Fake {
    fn work(&self) -> devpit_rpc::UpdateWork {
        let listed = |ids: &std::sync::Mutex<Vec<String>>| {
            ids.lock()
                .unwrap()
                .iter()
                .map(|id| devpit_rpc::UpdateBlocking {
                    id: id.clone(),
                    title: id.clone(),
                })
                .collect()
        };
        devpit_rpc::UpdateWork {
            runs: listed(&self.runs),
            turns: listed(&self.turns),
            keeps: Vec::new(),
        }
    }

    fn stop_run(&self, run_id: &str) {
        self.said
            .lock()
            .unwrap()
            .push(format!("{run_id} cancelled"));
        self.runs.lock().unwrap().retain(|id| id != run_id);
    }

    fn stop_turn(&self, conversation_id: &str) {
        self.said
            .lock()
            .unwrap()
            .push(format!("{conversation_id} stopped"));
        if !self.stubborn {
            self.turns
                .lock()
                .unwrap()
                .retain(|id| id != conversation_id);
        }
    }
}

/// Choosing to wait, or to stop the work, holds new work back at once: a run
/// started while the others end would be the next thing in the way.
#[test]
fn wait_refuses_new_work() {
    let ready = S::Ready {
        version: "0.2.0".to_owned(),
    };
    let busy = Fake::with(&["run_1"], &["chat_1"]).work();
    for choice in [Choice::WhenItIsDone, Choice::StopIt] {
        assert_ne!(install_plan(choice, &busy), Plan::InstallNow);
        let held = waiting_for(&ready, &busy, 10.0).expect("ready becomes a wait");
        assert_eq!(
            held,
            S::Waiting {
                runs: 1,
                turns: 1,
                since: 10.0
            }
        );
        assert!(
            starting_refused(&held).is_some(),
            "{choice:?} let new work in"
        );
        // Choosing again while waiting keeps the wait it already has.
        assert_eq!(waiting_for(&held, &busy, 99.0), Some(held.clone()));
    }
}

/// "Stop it and restart" stops every run and turn, and they are recorded as
/// stopped before anything is installed.
#[tokio::test]
async fn stop_records_cancelled_before_commit() {
    let way = Fake::with(&["run_1", "run_2"], &["chat_1"]);
    let went = stop_and_wait(
        &way,
        std::time::Duration::from_secs(5),
        std::time::Duration::from_millis(1),
    )
    .await;
    way.said.lock().unwrap().push("install".to_owned());

    assert!(went);
    assert_eq!(
        way.said(),
        vec![
            "run_1 cancelled",
            "run_2 cancelled",
            "chat_1 stopped",
            "install"
        ]
    );
    assert_eq!(blockers(&way.work()), 0);
}

/// A turn that ignores its signal holds the update for the few seconds it was
/// given, not for ever.
#[tokio::test]
async fn work_that_will_not_stop_holds_the_update_only_so_long() {
    let way = Fake {
        stubborn: true,
        ..Fake::with(&[], &["chat_1"])
    };
    let started = std::time::Instant::now();
    let went = stop_and_wait(
        &way,
        std::time::Duration::from_millis(30),
        std::time::Duration::from_millis(5),
    )
    .await;
    assert!(!went);
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
}

/// Nothing running: no question, nothing stopped, no waiting.
#[tokio::test]
async fn nothing_running_goes_straight_to_install() {
    let way = Fake::with(&[], &[]);
    for choice in [Choice::WhenItIsDone, Choice::StopIt] {
        assert_eq!(install_plan(choice, &way.work()), Plan::InstallNow);
    }
    let started = std::time::Instant::now();
    assert!(
        stop_and_wait(
            &way,
            std::time::Duration::from_secs(5),
            std::time::Duration::from_secs(1),
        )
        .await
    );
    assert!(way.said().is_empty());
    assert!(started.elapsed() < std::time::Duration::from_millis(500));
}

/// A refusal after the person decided ends the wait, so new work is not
/// refused until a restart.
#[test]
fn a_refused_install_ends_the_wait() {
    let waiting = S::Waiting {
        runs: 0,
        turns: 0,
        since: 1.0,
    };
    let failed = next(
        &waiting,
        Event::Failed {
            message: "no bytes".to_owned(),
        },
    )
    .expect("a wait can fail");
    assert!(matches!(
        failed,
        S::Failed {
            recoverable: true,
            ..
        }
    ));
    assert!(starting_refused(&failed).is_none());
}

/// An offer from a test feed never reaches the installer: it cannot be
/// downloaded, and nothing else produces bytes to install.
#[test]
fn a_fixture_feed_never_installs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let feed = dir.path().join("latest.json");
    std::fs::write(&feed, r#"{"version":"9.9.9","notes":"n"}"#).unwrap();

    let heard = next(&S::Checking, feed_says(&feed, "0.1.0")).expect("an offer");
    assert!(matches!(
        heard,
        S::Available {
            test_feed: true,
            ..
        }
    ));
    assert!(may_download(&heard).is_err());
    assert_eq!(next(&heard, Event::Install), None);
}
