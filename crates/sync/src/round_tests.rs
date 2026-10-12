use std::cell::RefCell;

use super::*;
use crate::space::{Here, Seen};

/// A namespace held in memory: one record per name, fields as set.
struct Table {
    rows: RefCell<Here>,
    writes: RefCell<Vec<(String, String, Option<String>)>>,
}

impl Table {
    fn new(rows: &[(&str, &str, &str, u64)]) -> Table {
        let mut here = Here::new();
        for (record, field, value, at) in rows {
            here.entry((*record).to_owned()).or_default().insert(
                (*field).to_owned(),
                Seen::of(Some((*value).to_owned()), Some(*at)),
            );
        }
        Table {
            rows: RefCell::new(here),
            writes: RefCell::new(Vec::new()),
        }
    }

    fn set(&self, record: &str, field: &str, value: &str, at: u64) {
        self.rows
            .borrow_mut()
            .entry(record.to_owned())
            .or_default()
            .insert(field.to_owned(), Seen::of(Some(value.to_owned()), Some(at)));
    }

    fn value(&self, record: &str, field: &str) -> Option<String> {
        self.rows.borrow().get(record)?.get(field)?.value.clone()
    }
}

impl Namespace for Table {
    fn name(&self) -> &'static str {
        "table"
    }

    fn here(&self) -> Result<Here, String> {
        Ok(self.rows.borrow().clone())
    }

    fn write(&self, record: &str, field: &str, value: Option<&str>) -> Result<(), String> {
        self.writes.borrow_mut().push((
            record.to_owned(),
            field.to_owned(),
            value.map(str::to_owned),
        ));
        if let Some(fields) = self.rows.borrow_mut().get_mut(record) {
            // Written now, as a store would stamp it.
            fields.insert(
                field.to_owned(),
                Seen::of(value.map(str::to_owned), Some(99_999)),
            );
        }
        Ok(())
    }
}

fn kept_after(
    table: &Table,
    kept: &mut Kept,
    clock: &mut Clock,
    fetched: Option<&(u64, Doc)>,
) -> bool {
    settle(table, kept, clock, fetched, 50_000).expect("settle")
}

#[test]
fn a_value_changed_here_is_stamped_once_and_not_again() {
    let table = Table::new(&[("app", "appearance.theme", "dark", 1_000)]);
    let (mut kept, mut clock) = (Kept::default(), Clock::new("m_a"));
    kept_after(&table, &mut kept, &mut clock, None);
    let first = kept
        .mirror
        .get("app", "appearance.theme")
        .cloned()
        .expect("stamped");
    assert_eq!(first.at.ms, 1_000, "placed by when it was set here");

    kept_after(&table, &mut kept, &mut clock, None);
    assert_eq!(
        kept.mirror.get("app", "appearance.theme"),
        Some(&first),
        "unchanged is not new"
    );

    table.set("app", "appearance.theme", "light", 2_000);
    kept_after(&table, &mut kept, &mut clock, None);
    let second = kept.mirror.get("app", "appearance.theme").expect("stamped");
    assert_eq!(second.value.as_deref(), Some("light"));
    assert!(second.at > first.at);
}

#[test]
fn a_machine_joining_takes_what_the_others_settled_before_adding_its_own() {
    let mut theirs = Doc::default();
    theirs.put(
        "app",
        "appearance.theme",
        Cell {
            value: Some("light".to_owned()),
            at: Stamp {
                ms: 10,
                n: 0,
                by: "m_a".to_owned(),
            },
        },
    );
    // Set long after the other machine's choice, and still not taken over it.
    let table = Table::new(&[
        ("app", "appearance.theme", "dark", 90_000),
        ("app", "terminal.confirm_stop", "false", 90_000),
    ]);
    let (mut kept, mut clock) = (Kept::default(), Clock::new("m_b"));
    let written = kept_after(&table, &mut kept, &mut clock, Some(&(1, theirs)));
    assert!(written);
    assert_eq!(
        table.value("app", "appearance.theme").as_deref(),
        Some("light")
    );
    assert_eq!(
        kept.mirror
            .get("app", "terminal.confirm_stop")
            .and_then(|c| c.value.as_deref()),
        Some("false"),
        "what nobody had is added"
    );
    assert_eq!(kept.version, 1);
}

#[test]
fn a_change_made_offline_loses_to_a_later_one_from_another_machine() {
    let table = Table::new(&[("app", "appearance.theme", "dark", 1_000)]);
    let (mut kept, mut clock) = (Kept::default(), Clock::new("m_b"));
    kept_after(&table, &mut kept, &mut clock, None);

    // Changed here at 5 s, while another machine changed it at 7 s.
    table.set("app", "appearance.theme", "system", 5_000);
    let mut theirs = kept.mirror.clone();
    theirs.put(
        "app",
        "appearance.theme",
        Cell {
            value: Some("light".to_owned()),
            at: Stamp {
                ms: 7_000,
                n: 0,
                by: "m_a".to_owned(),
            },
        },
    );
    kept_after(&table, &mut kept, &mut clock, Some(&(2, theirs)));
    assert_eq!(
        table.value("app", "appearance.theme").as_deref(),
        Some("light")
    );

    // And a change made after that was seen wins, whatever the wall says.
    table.set("app", "appearance.theme", "dark", 3_000);
    kept_after(&table, &mut kept, &mut clock, None);
    let cell = kept.mirror.get("app", "appearance.theme").expect("cell");
    assert_eq!(cell.value.as_deref(), Some("dark"));
    assert!(cell.at.ms >= 7_000);
}

#[test]
fn a_record_this_machine_does_not_have_is_kept_but_never_written_here() {
    let table = Table::new(&[("origin:mine", "icon", "rocket", 1_000)]);
    let mut theirs = Doc::default();
    theirs.put(
        "origin:elsewhere",
        "icon",
        Cell {
            value: Some("star".to_owned()),
            at: Stamp {
                ms: 5,
                n: 0,
                by: "m_a".to_owned(),
            },
        },
    );
    let (mut kept, mut clock) = (Kept::default(), Clock::new("m_b"));
    kept_after(&table, &mut kept, &mut clock, Some(&(1, theirs)));
    assert!(
        kept.mirror.get("origin:elsewhere", "icon").is_some(),
        "kept for the others"
    );
    assert!(
        table.writes.borrow().is_empty(),
        "nothing here to write it to"
    );
}

#[test]
fn what_was_written_here_is_not_taken_for_a_change_made_here() {
    let table = Table::new(&[("app", "appearance.theme", "dark", 1_000)]);
    let (mut kept, mut clock) = (Kept::default(), Clock::new("m_b"));
    kept_after(&table, &mut kept, &mut clock, None);
    let mut theirs = kept.mirror.clone();
    theirs.put(
        "app",
        "appearance.theme",
        Cell {
            value: Some("light".to_owned()),
            at: Stamp {
                ms: 9_000,
                n: 0,
                by: "m_a".to_owned(),
            },
        },
    );
    kept_after(&table, &mut kept, &mut clock, Some(&(2, theirs)));
    let after = kept.mirror.clone();
    kept_after(&table, &mut kept, &mut clock, None);
    assert_eq!(
        kept.mirror, after,
        "the written value keeps the other machine's stamp"
    );
    assert_eq!(
        kept.mirror
            .get("app", "appearance.theme")
            .map(|c| c.at.by.as_str()),
        Some("m_a")
    );
}
