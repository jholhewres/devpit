//! What the error file keeps, and what it never does.

use super::*;

const DAY: u64 = 24 * 60 * 60;

fn report(message: &str) -> Report<'_> {
    Report {
        kind: "internal",
        location: Some("apps/desktop/src/cards.rs:12:5"),
        message,
        stack: None,
    }
}

fn log() -> (tempfile::TempDir, ErrorLog) {
    let dir = tempfile::tempdir().expect("tempdir");
    let log = ErrorLog::at(dir.path());
    (dir, log)
}

#[test]
fn a_repeat_counts_instead_of_adding_a_line() {
    let (_dir, log) = log();
    log.add(report("could not read run 41"), 100, None)
        .expect("add");
    log.add(report("could not read run 97"), 250, None)
        .expect("add");

    let entries = log.read();
    assert_eq!(
        entries.len(),
        1,
        "numbers are not what tells two bugs apart"
    );
    assert_eq!(entries[0].count, 2);
    assert_eq!(entries[0].first_seen, 100);
    assert_eq!(entries[0].last_seen, 250);
}

#[test]
fn a_different_place_is_a_different_error() {
    let (_dir, log) = log();
    log.add(report("broken"), 1, None).expect("add");
    log.add(
        Report {
            location: Some("apps/desktop/src/runs.rs:3:1"),
            ..report("broken")
        },
        2,
        None,
    )
    .expect("add");
    assert_eq!(log.read().len(), 2);
}

#[test]
fn an_error_older_than_fifteen_days_goes() {
    let (_dir, log) = log();
    log.add(report("old"), 0, None).expect("add");
    log.add(report("new one"), MAX_AGE_SECS + DAY, None)
        .expect("add");

    let kept: Vec<_> = log.read().into_iter().map(|entry| entry.message).collect();
    assert_eq!(kept, ["new one"]);
}

#[test]
fn past_the_entry_ceiling_the_least_recent_goes() {
    let mut entries: Vec<Entry> = (0..MAX_ENTRIES as u64 + 5)
        .map(|seen| Entry {
            fingerprint: format!("f{seen}"),
            kind: "internal".into(),
            location: None,
            message: "m".into(),
            stack: None,
            count: 1,
            first_seen: seen,
            last_seen: seen,
        })
        .collect();
    let now = MAX_ENTRIES as u64 + 5;
    keep(&mut entries, now);

    assert_eq!(entries.len(), MAX_ENTRIES);
    assert!(
        entries.iter().all(|entry| entry.last_seen >= 5),
        "the five oldest went"
    );
}

#[test]
fn past_the_byte_ceiling_the_least_recent_goes() {
    let (_dir, log) = log();
    let big = "x".repeat(STACK_CAP);
    // Sixty full stacks are about twice the ceiling.
    for seen in 0..60 {
        log.add(
            Report {
                location: Some(&format!("loc{seen}")),
                stack: Some(&big),
                ..report("same words")
            },
            seen as u64,
            None,
        )
        .expect("add");
    }
    let size = std::fs::metadata(log.path()).expect("file").len() as usize;
    assert!(size <= MAX_BYTES, "{size} bytes");
    let newest = log.read().iter().map(|entry| entry.last_seen).max();
    assert_eq!(newest, Some(59), "what goes is the oldest, not the newest");
}

#[test]
fn paths_do_not_survive_the_write() {
    let home = Path::new("/home/someone");
    let text = "could not open /home/someone/work/acme-secret/.git/HEAD \
                or /var/lib/acme/x.db (see C:\\Users\\someone\\repo) in ~/Projects/acme, home is /home/someone";
    let clean = sanitize(text, Some(home));

    for leaked in ["someone", "acme", "work", "Projects", "var/lib", "Users"] {
        assert!(!clean.contains(leaked), "{leaked} in {clean}");
    }
    assert!(clean.ends_with("home is ~"), "{clean}");
}

/// F1: a path is a path wherever it starts, and the home only where it is
/// the home — not the start of somebody else's folder.
#[test]
fn a_path_after_any_punctuation_goes_too() {
    let home = Some(Path::new("/home/jhol"));
    for text in [
        "cwd:/home/jhol/Workspace/acme",
        "open file:///home/jhol/acme/x.txt failed",
        "open file:///srv/acme/x.txt failed",
        "failed;/home/jhol/acme",
        "x|/home/jhol/acme",
        "{/home/jhol/acme}",
        "a\u{a0}/home/jhol/acme",
        "at:C:\\Users\\jhol\\acme",
        "~jhol/Workspace/acme",
        "/home/jholder/acme",
        "failed: \"/home/jhol/Work Space/acme dir/x\"",
        "failed: '/home/jhol/Work Space/acme dir/x'",
    ] {
        let clean = sanitize(text, home);
        assert!(!clean.contains("acme"), "{text:?} -> {clean:?}");
        assert!(!clean.contains("jhol"), "{text:?} -> {clean:?}");
    }
}

/// F2: credentials and long ids do not survive either.
#[test]
fn a_secret_does_not_survive_the_write() {
    for (text, secret) in [
        ("https://jhol:hunter2@github.com/acme/x", "hunter2"),
        ("GITHUB_TOKEN=ghp_abcdefghijklmnop1234", "ghp_"),
        (
            "Authorization: Bearer sk-ant-api03-abcDEF123456789xyz",
            "sk-ant",
        ),
        ("card card_01M38MRZKJ852TFM49KJEXBCAY not found", "01M38"),
    ] {
        let clean = sanitize(text, None);
        assert!(!clean.contains(secret), "{text:?} -> {clean:?}");
    }
    let url = sanitize("https://jhol:hunter2@github.com/acme/x", None);
    assert!(!url.contains("jhol"), "{url}");
}

#[test]
fn what_is_not_a_path_is_left_alone() {
    for text in [
        "expected 1/2 of https://example.com/a at apps/desktop/src/x.rs:4:2 / done",
        "at tauri://localhost/assets/index-Bx1.js:12:7 in core::ops::function::FnOnce",
        "a.b/c and x-y/z are relative",
    ] {
        assert_eq!(sanitize(text, None), text);
    }
}

#[test]
fn a_long_message_is_cut_on_a_character() {
    let (_dir, log) = log();
    let long = "é".repeat(MESSAGE_CAP);
    log.add(report(&long), 1, None).expect("add");
    let kept = &log.read()[0].message;
    assert!(kept.len() <= MESSAGE_CAP + '…'.len_utf8());
}

#[test]
fn wiping_removes_the_file_and_is_fine_when_there_is_none() {
    let (_dir, log) = log();
    log.wipe().expect("nothing to wipe");
    log.add(report("x"), 1, None).expect("add");
    log.wipe().expect("wipe");
    assert!(!log.path().exists());
}

/// F4: a write that stopped before its rename left its temporary behind, and
/// off means that goes too.
#[test]
fn wiping_takes_a_half_finished_write_with_it() {
    let (_dir, log) = log();
    log.add(report("x"), 1, None).expect("add");
    let temporary = log.path().with_extension("jsonl.tmp");
    std::fs::write(&temporary, "{}\n").expect("leftover");
    log.wipe().expect("wipe");
    assert!(!temporary.exists());
}

/// F3: fifteen days hold with no new error arriving to trigger a prune.
#[test]
fn pruning_drops_the_old_without_a_new_error() {
    let (_dir, log) = log();
    log.add(report("old"), 0, None).expect("add");
    log.prune(MAX_AGE_SECS + DAY).expect("prune");
    assert!(log.read().is_empty());
}

/// The switch is process-wide, so this is the only test that touches it.
#[test]
fn off_nothing_is_written_and_turning_off_wipes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = ErrorLog::at(dir.path()).path().to_path_buf();

    record(report("before anything was asked"));
    assert!(!path.exists(), "off is the default");

    // Turning on prunes what an earlier session left past its age.
    ErrorLog::at(dir.path())
        .add(report("left from long ago"), 0, None)
        .expect("old");
    switch(dir.path(), true).expect("on");
    assert!(
        ErrorLog::at(dir.path()).read().is_empty(),
        "pruned on start"
    );
    record(report("kept"));
    background("the updater", &"it broke");
    assert_eq!(ErrorLog::at(dir.path()).read().len(), 2);

    switch(dir.path(), false).expect("off");
    assert!(!path.exists(), "off means nothing kept");
    record(report("after"));
    assert!(!path.exists());
}

/// F5: std quotes the value a panic was about, and that value can be a card
/// body or a prompt. What is kept is the shape of the panic, not the value.
#[test]
fn a_panic_keeps_its_shape_and_not_the_value_it_quoted() {
    for (payload, secret, kept) in [
        (
            "byte index 34 is not a char boundary; it is inside 'é' (bytes 33..35) of `Plano secreto: adquirir a ACME`",
            "ACME",
            "byte index 34 is not a char boundary",
        ),
        (
            "called `Result::unwrap()` on an `Err` value: Custom { kind: Other, error: \"token sk-123 for acme\" }",
            "acme",
            "called `Result::unwrap()` on an `Err` value: …",
        ),
        (
            "assertion `left == right` failed\n  left: \"acme card\"\n right: \"other\"",
            "acme",
            "assertion `left == right` failed",
        ),
    ] {
        let message = panic_message(payload);
        assert!(!message.contains(secret), "{payload:?} -> {message:?}");
        assert!(message.starts_with(kept), "{payload:?} -> {message:?}");
    }
    assert_eq!(
        panic_message("attempt to subtract with overflow"),
        "attempt to subtract with overflow"
    );
}

/// What git and the shell name — a branch made from a card's title, a
/// command somebody wrote — is theirs; what failed is ours to keep.
#[test]
fn a_failed_command_keeps_what_failed_and_not_what_it_named() {
    for (text, secret, kept) in [
        (
            "git worktree add -b devpit/plano-da-acme-kjexbcay <path> HEAD failed: fatal: a branch named 'devpit/plano-da-acme-kjexbcay' already exists",
            "acme",
            "git worktree … failed: fatal: a branch named '…' already exists",
        ),
        (
            "preparing the worktree failed: `pnpm install --registry acme.internal` exited 1",
            "acme",
            "preparing the worktree failed: `…` exited 1",
        ),
    ] {
        let clean = sanitize(text, None);
        assert!(!clean.contains(secret), "{text:?} -> {clean:?}");
        assert_eq!(clean, kept);
    }
    // An apostrophe is not a quote.
    let prose = "the store doesn't answer and can't be read";
    assert_eq!(sanitize(prose, None), prose);
}

/// A burst is counted, not written a line at a time — and the count is not
/// lost: it goes in with the next write once the burst has gone quiet.
#[test]
fn a_burst_is_counted_and_written_once() {
    let (_dir, log) = log();
    let mut recorder = Recorder::new(log.clone());
    for _ in 0..60 {
        recorder
            .take(report("redraw failed"), 100, None)
            .expect("take");
    }
    let written = std::fs::metadata(log.path())
        .expect("file")
        .modified()
        .expect("mtime");
    assert_eq!(
        log.read()[0].count,
        1,
        "only the first of the burst was written"
    );

    recorder
        .take(report("redraw failed"), 100 + QUIET_SECS, None)
        .expect("after the quiet");
    assert_eq!(log.read()[0].count, 61, "the held ones went in with it");
    assert!(
        std::fs::metadata(log.path())
            .expect("file")
            .modified()
            .expect("mtime")
            >= written
    );
}

/// F7: a huge text from the window is cut before it is cleaned.
#[test]
fn a_huge_message_is_cut_before_it_costs_anything() {
    let (_dir, log) = log();
    let huge = "x".repeat(50 * 1024 * 1024);
    let started = std::time::Instant::now();
    log.add(report(&huge), 1, None).expect("add");
    assert!(log.read()[0].message.len() <= MESSAGE_CAP + '…'.len_utf8());
    assert!(
        started.elapsed() < std::time::Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
}

/// F6: the tests the mutants got past.
#[test]
fn a_long_id_does_not_make_one_bug_two() {
    let (_dir, log) = log();
    log.add(
        report("card card_01M38MRZKJ852TFM49KJEXBCAY not found"),
        1,
        None,
    )
    .expect("add");
    log.add(
        report("card card_01M2S1D4YD9KYTAZCGECKP22DP not found"),
        2,
        None,
    )
    .expect("add");
    assert_eq!(log.read().len(), 1);
    assert_eq!(log.read()[0].count, 2);
}

#[test]
fn the_same_words_from_another_kind_are_another_error() {
    let (_dir, log) = log();
    log.add(
        Report {
            kind: "frontend",
            ..report("broken")
        },
        1,
        None,
    )
    .expect("add");
    log.add(
        Report {
            kind: "background",
            ..report("broken")
        },
        2,
        None,
    )
    .expect("add");
    assert_eq!(log.read().len(), 2);
}

#[test]
fn age_counts_from_the_last_time_it_was_seen() {
    let (_dir, log) = log();
    log.add(report("recurring"), 0, None).expect("add");
    log.add(report("recurring"), MAX_AGE_SECS, None)
        .expect("seen again");
    log.add(report("something else"), MAX_AGE_SECS + DAY, None)
        .expect("add");

    let recurring = log
        .read()
        .into_iter()
        .find(|entry| entry.message == "recurring");
    assert_eq!(
        recurring.map(|entry| entry.count),
        Some(2),
        "a bug still happening stays"
    );
    log.prune(2 * MAX_AGE_SECS + 1).expect("prune");
    assert!(
        log.read().iter().all(|entry| entry.message != "recurring"),
        "one day past fifteen"
    );
}

#[test]
fn a_huge_stack_is_cut_and_does_not_empty_the_file() {
    let (_dir, log) = log();
    log.add(report("small"), 1, None).expect("add");
    let huge = "x".repeat(MAX_BYTES);
    log.add(
        Report {
            stack: Some(&huge),
            ..report("big")
        },
        2,
        None,
    )
    .expect("add");
    let kept = log.read();
    assert_eq!(kept.len(), 2, "one big stack does not push everything out");
    assert!(kept.iter().all(|entry| entry
        .stack
        .as_ref()
        .is_none_or(|s| s.len() <= STACK_CAP + '…'.len_utf8())));
}

#[test]
fn a_panic_while_the_lock_is_held_does_not_wait_for_it() {
    let held = ON.lock().unwrap_or_else(PoisonError::into_inner);
    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        record_now_or_never(report("while held"));
        let _ = done.send(());
    });
    let answered = finished.recv_timeout(std::time::Duration::from_secs(2));
    drop(held);
    assert!(answered.is_ok(), "the panic path waited on the lock");
}
