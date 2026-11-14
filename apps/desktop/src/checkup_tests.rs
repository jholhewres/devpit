use std::path::Path;

use super::*;

fn session(state: Doing) -> IslandSession {
    IslandSession {
        session_id: "s".to_owned(),
        pane_id: None,
        project_id: None,
        project: None,
        color: None,
        card_id: None,
        card: None,
        root: None,
        state,
        steps: Vec::new(),
        said: None,
        at: 0.0,
    }
}

fn healthy() -> Facts {
    Facts {
        sessions: Some(Sessions::default()),
        agent_cli: Some("/usr/bin/claude".to_owned()),
        tmux: Some("/usr/bin/tmux".to_owned()),
        disk: Some(Disk {
            free: 80_000_000_000,
            total: 500_000_000_000,
        }),
        load: Some(2.0),
        cpus: 8,
    }
}

fn named<'a>(checks: &'a [Check], name: &str) -> &'a Check {
    checks
        .iter()
        .find(|check| check.name == name)
        .unwrap_or_else(|| panic!("no check named {name}"))
}

#[test]
fn status_counts_the_sessions_running_and_those_waiting_on_you() {
    let all = [
        session(Doing::Working),
        session(Doing::Waiting),
        session(Doing::Open),
        session(Doing::Done),
        session(Doing::Failed),
    ];
    assert_eq!(
        sessions_of(&all),
        Sessions {
            running: 3,
            waiting: 1
        }
    );
}

#[test]
fn a_healthy_machine_passes_every_check_and_is_given_no_fix() {
    let checks = checks(&healthy(), Path::new("/home/me/.devpit"));
    assert!(checks.iter().all(|check| check.ok), "{checks:#?}");
    assert!(checks.iter().all(|check| check.fix.is_none()));
}

#[test]
fn each_failing_check_says_how_to_fix_it() {
    let facts = Facts {
        sessions: None,
        agent_cli: None,
        tmux: None,
        disk: Some(Disk {
            free: 2_000_000_000,
            total: 500_000_000_000,
        }),
        load: Some(30.0),
        cpus: 8,
    };
    let checks = checks(&facts, Path::new("/home/me/.devpit"));
    let open = named(&checks, "devpit is open");
    assert!(!open.ok);
    assert_eq!(open.fix.as_deref(), Some("run `devpit open`"));
    assert!(!named(&checks, "claude is installed").ok);
    let disk = named(&checks, "the disk has room");
    assert!(!disk.ok);
    assert_eq!(disk.detail, "2.0 GB free of 500.0 GB");
    assert!(disk
        .fix
        .as_deref()
        .is_some_and(|fix| fix.contains("/home/me/.devpit")));
    let load = named(&checks, "the load is normal");
    assert!(!load.ok);
    assert_eq!(load.detail, "30.00 on 8 CPUs");
    assert!(checks
        .iter()
        .filter(|check| !check.ok)
        .all(|check| check.fix.is_some()));
}

#[cfg(unix)]
#[test]
fn tmux_missing_fails_where_sessions_need_it() {
    let facts = Facts {
        tmux: None,
        ..healthy()
    };
    let checks = checks(&facts, Path::new("/"));
    assert!(!named(&checks, "tmux is installed").ok);
}

#[test]
fn a_check_this_system_cannot_make_does_not_fail() {
    let facts = Facts {
        disk: None,
        load: None,
        ..healthy()
    };
    assert!(checks(&facts, Path::new("/")).iter().all(|check| check.ok));
}

#[cfg(unix)]
#[test]
fn this_machine_answers_its_disk_and_load() {
    let root = std::env::temp_dir();
    let disk = disk(&root).expect("a disk under the temp folder");
    assert!(disk.total >= disk.free && disk.total > 0);
    assert!(load().is_some_and(|load| load >= 0.0));
}

#[test]
fn the_disk_is_read_from_dfs_portable_output() {
    let said = "Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/mapper/root 487196716 436989588 25386240 95% /\n";
    assert_eq!(
        df_disk(said),
        Some(Disk {
            free: 25_386_240 * 1024,
            total: 487_196_716 * 1024,
        })
    );
    assert_eq!(df_disk("df: /nowhere: No such file or directory\n"), None);
}
