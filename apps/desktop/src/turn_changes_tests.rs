use super::*;

fn git(dir: &Path, args: &[&str]) {
    let ok = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git")
        .status
        .success();
    assert!(ok, "git {args:?} failed");
}

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "t@example.invalid"]);
    git(dir.path(), &["config", "user.name", "T"]);
    git(dir.path(), &["config", "commit.gpgsign", "false"]);
    std::fs::write(dir.path().join("a.txt"), "one\n").expect("write");
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-q", "-m", "start"]);
    dir
}

#[test]
fn a_turn_that_edited_a_file_reports_it() {
    let dir = repo();
    let before = before(dir.path());
    std::fs::write(dir.path().join("a.txt"), "one\ntwo\n").expect("edit");
    std::fs::write(dir.path().join("b.txt"), "new\n").expect("create");

    let Some(Part::Changes {
        files,
        before: kept,
    }) = since(dir.path(), before.as_deref())
    else {
        panic!("expected a rollup");
    };
    assert_eq!(
        files,
        [
            ChangedFile {
                path: "a.txt".to_owned(),
                added: 1,
                removed: 0
            },
            ChangedFile {
                path: "b.txt".to_owned(),
                added: 1,
                removed: 0
            },
        ]
    );
    assert_eq!(kept, before, "the tree the turn found, to put back");
}

#[test]
fn a_turn_that_changed_nothing_says_nothing() {
    let dir = repo();
    let before = before(dir.path());
    assert!(since(dir.path(), before.as_deref()).is_none());
}

#[test]
fn a_folder_that_is_not_a_repository_has_no_rollup() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(before(dir.path()).is_none());
    assert!(since(dir.path(), None).is_none());
}

#[test]
fn a_turns_start_does_not_wait_for_its_snapshot() {
    let (release, held) = std::sync::mpsc::channel::<()>();
    let asked = std::time::Instant::now();
    let before = Before::reading(move || {
        // git still reading, until the test lets it go or two seconds pass.
        let _ = held.recv_timeout(std::time::Duration::from_secs(2));
        Some("tree".to_owned())
    });
    let waited = asked.elapsed();
    release.send(()).ok();

    assert!(
        waited < std::time::Duration::from_secs(1),
        "the start waited {waited:?}"
    );
    assert_eq!(before.taken().as_deref(), Some("tree"));
}

#[test]
fn a_turns_end_is_sent_before_what_it_changed_is_read() {
    use std::sync::Arc;
    use tauri::ipc::InvokeResponseBody;

    let heard = Arc::new(Mutex::new(Vec::<&str>::new()));
    let into = heard.clone();
    let sink = Channel::new(move |body: InvokeResponseBody| {
        let said = match body.deserialize::<Frame>().expect("a frame") {
            Frame::Ended { .. } => "ended",
            Frame::Part {
                part: Part::Changes { .. },
                ..
            } => "changes",
            _ => "other",
        };
        into.lock().unwrap().push(said);
        Ok(())
    });
    let end = devpit_rpc::TurnEnd {
        turn_id: "turn_1".to_owned(),
        cost_usd: None,
        duration_ms: None,
        stop_reason: Some("end_turn".to_owned()),
        is_error: false,
        context: None,
        ..Default::default()
    };
    let collected = Mutex::new(Vec::new());
    let mut seen_by_git = Vec::new();

    ended(
        &sink,
        end,
        || {
            seen_by_git = heard.lock().unwrap().clone();
            Some(Part::Changes {
                files: vec![ChangedFile {
                    path: "a.txt".to_owned(),
                    added: 1,
                    removed: 0,
                }],
                before: None,
            })
        },
        &collected,
        "m1",
    );

    assert_eq!(seen_by_git, ["ended"]);
    assert_eq!(*heard.lock().unwrap(), ["ended", "changes"]);
    assert!(matches!(
        collected.lock().unwrap()[..],
        [Part::Changes { .. }]
    ));
}
