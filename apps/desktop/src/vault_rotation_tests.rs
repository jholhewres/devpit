use std::path::PathBuf;

use devpit_core::Store;

use super::{after_rotation, due, kept_again, plan, rotated};
use crate::reminders::{fire, offered, KIND};
use crate::vault::{keep, listed};

const DAY: i64 = 24 * 60 * 60;
const VALUE: &str = "hf_value_that_is_never_said";

#[test]
fn a_rotation_is_due_on_its_date_or_an_interval_after_the_key_was_set() {
    assert_eq!(due(1_000, None, None), None, "no rotation asked for");
    assert_eq!(due(1_000, Some(90), None), Some(1_000 + 90 * DAY));
    assert_eq!(
        due(1_000, Some(90), Some(5_000)),
        Some(5_000),
        "a date asked for comes before the interval"
    );
    assert_eq!(after_rotation(7_000, Some(30)), Some(7_000 + 30 * DAY));
    assert_eq!(
        after_rotation(7_000, None),
        None,
        "a date alone is done with"
    );
}

struct Project {
    _dir: tempfile::TempDir,
    store: Store,
    id: String,
    root: PathBuf,
    kept: PathBuf,
}

impl Project {
    fn set_at(&self) -> i64 {
        listed(&self.kept)[0].set_at as i64
    }
}

/// A project with a git checkout and one secret kept in its `.env`.
fn project() -> Project {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("store");
    let root = dir.path().join("api");
    std::fs::create_dir_all(&root).expect("root");
    assert!(std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&root)
        .status()
        .expect("git")
        .success());
    let id = store.add_project(&root, None).expect("project");
    store.ensure_board(&id).expect("board");
    let kept = dir.path().join("home").join("secrets.json");
    keep(&root, &kept, "HF_TOKEN", VALUE).expect("kept");
    Project {
        _dir: dir,
        store,
        id,
        root,
        kept,
    }
}

#[test]
fn a_secret_due_for_rotation_goes_off_as_a_reminder_that_names_it_and_not_its_value() {
    let p = project();
    let due_at = p.set_at() + 90 * DAY;
    let list = plan(
        &p.store,
        &p.id,
        &p.kept,
        "HF_TOKEN",
        Some(90),
        None,
        p.set_at(),
    )
    .expect("set");
    assert_eq!(list[0].rotate_at, Some(due_at as f64));
    let card = list[0]
        .reminder_card
        .clone()
        .expect("a card carries the date");

    assert!(fire(&p.store, due_at - 1).is_empty(), "not before its date");
    let fired = fire(&p.store, due_at);
    assert_eq!(fired.len(), 1);
    assert_eq!(
        (fired[0].card_id.as_str(), fired[0].project_id.as_str()),
        (card.as_str(), p.id.as_str())
    );
    assert_eq!(fired[0].title, "Rotate HF_TOKEN");
    let bell = &p.store.notices(10).expect("notices")[0];
    assert_eq!(bell.kind, KIND);
    assert_eq!(bell.project_id.as_deref(), Some(p.id.as_str()));
    assert!(bell.title.contains("HF_TOKEN"));

    let row = p.store.card(&card).expect("card").expect("there");
    for said in [row.title, row.body, bell.title.clone()] {
        assert!(!said.contains(VALUE), "{said}");
    }
    let kept = std::fs::read_to_string(&p.kept).expect("kept");
    assert!(!kept.contains(VALUE) && kept.contains("rotateDays"));
    assert!(
        !std::fs::read_to_string(p.root.join(".env"))
            .expect("env")
            .contains("rotate"),
        "the rotation is never kept in the .env"
    );
}

#[test]
fn rotated_moves_the_date_on_by_its_interval() {
    let p = project();
    let list = plan(
        &p.store,
        &p.id,
        &p.kept,
        "HF_TOKEN",
        Some(30),
        None,
        p.set_at(),
    )
    .expect("set");
    let card = list[0].reminder_card.clone();
    let due_at = p.set_at() + 30 * DAY;
    fire(&p.store, due_at);
    assert_eq!(p.store.reminders_pending().expect("pending").len(), 1);

    let later = due_at + 3_600;
    let list = rotated(&p.store, &p.id, &p.kept, "HF_TOKEN", later).expect("rotated");
    assert_eq!(list[0].rotate_at, Some((later + 30 * DAY) as f64));
    assert_eq!(
        list[0].reminder_card, card,
        "the same card carries the next date"
    );
    assert!(p.store.reminders_pending().expect("pending").is_empty());
    assert_eq!(
        p.store.next_reminder().expect("next"),
        Some(later + 30 * DAY)
    );
    assert_eq!(
        p.store.unread_notices().expect("unread"),
        0,
        "the bell is read too"
    );
    assert_eq!(listed(&p.kept)[0].rotate_at, list[0].rotate_at, "and kept");
}

#[test]
fn a_date_alone_ends_once_rotated_and_forget_puts_the_card_away() {
    let p = project();
    let now = p.set_at();
    let list = plan(
        &p.store,
        &p.id,
        &p.kept,
        "HF_TOKEN",
        None,
        Some(now + 10 * DAY),
        now,
    )
    .expect("set");
    let card = list[0].reminder_card.clone().expect("card");
    let list = rotated(&p.store, &p.id, &p.kept, "HF_TOKEN", now + DAY).expect("rotated");
    assert_eq!(
        (list[0].rotate_at, list[0].reminder_card.as_deref()),
        (None, None)
    );
    assert!(
        p.store.reminder(&card).expect("read").is_none(),
        "its card is put away"
    );

    plan(&p.store, &p.id, &p.kept, "HF_TOKEN", Some(30), None, now).expect("again");
    let list = plan(&p.store, &p.id, &p.kept, "HF_TOKEN", None, None, now).expect("forgotten");
    assert_eq!((list[0].rotate_days, list[0].rotate_at), (None, None));
    assert_eq!(p.store.next_reminder().expect("next"), None);

    for (days, at) in [(Some(0), None), (Some(4_000), None), (None, Some(now - 1))] {
        assert!(plan(&p.store, &p.id, &p.kept, "HF_TOKEN", days, at, now).is_err());
    }
    assert!(plan(&p.store, &p.id, &p.kept, "NOT_KEPT", Some(30), None, now).is_err());
}

#[test]
fn a_new_value_counts_as_rotated_and_the_rotation_stays() {
    let p = project();
    let now = p.set_at();
    plan(&p.store, &p.id, &p.kept, "HF_TOKEN", Some(30), None, now).expect("set");
    keep(&p.root, &p.kept, "HF_TOKEN", "hf_the_new_one").expect("kept again");
    assert_eq!(
        listed(&p.kept)[0].rotate_days,
        Some(30),
        "a new value keeps it"
    );

    let later = now + 5 * DAY;
    assert!(kept_again(&p.store, &p.id, &p.kept, "HF_TOKEN", later).expect("rotated"));
    assert_eq!(
        listed(&p.kept)[0].rotate_at,
        Some((later + 30 * DAY) as f64)
    );

    keep(&p.root, &p.kept, "OTHER_KEY", "x").expect("kept");
    assert!(!kept_again(&p.store, &p.id, &p.kept, "OTHER_KEY", later).expect("none"));
}

#[test]
fn a_rotation_reminder_offers_rotated_snooze_and_forget() {
    let said = |rotation| -> Vec<String> {
        offered("card_1", rotation)
            .iter()
            .map(crate::channel_actions::label)
            .collect()
    };
    assert_eq!(said(true), ["Rotated", "In 15 minutes", "Forget"]);
    assert_eq!(said(false), ["Done", "In 15 minutes"]);
}
