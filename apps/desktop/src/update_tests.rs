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
        install_kind(Some("/opt/devpit.AppImage"), None, &[]),
        InstallKind::AppImage
    );
    assert_eq!(
        install_kind(None, Some("appimage"), &[]),
        InstallKind::AppImage
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
    assert!(starting_refused(&ready).is_some());
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
