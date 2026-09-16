//! The three rules, each by branch.

use super::*;
use devpit_rpc::UpdateStatus as S;

fn available() -> S {
    S::Available {
        version: "0.2.0".to_owned(),
        notes: "what changed".to_owned(),
        kind: InstallKind::AppImage,
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
