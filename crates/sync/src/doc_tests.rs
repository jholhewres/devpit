use super::*;

fn cell(value: Option<&str>, ms: u64, by: &str) -> Cell {
    Cell {
        value: value.map(str::to_owned),
        at: Stamp {
            ms,
            n: 0,
            by: by.to_owned(),
        },
    }
}

fn doc(writes: &[(&str, &str, Option<&str>, u64, &str)]) -> Doc {
    let mut doc = Doc::default();
    for (record, field, value, ms, by) in writes {
        doc.put(record, field, cell(*value, *ms, by));
    }
    doc
}

#[test]
fn the_later_write_to_a_field_is_kept_and_an_earlier_one_is_not() {
    let mut kept = doc(&[("app", "appearance.theme", Some("dark"), 10, "m_a")]);
    assert!(!kept.put("app", "appearance.theme", cell(Some("light"), 5, "m_b")));
    assert!(kept.put("app", "appearance.theme", cell(Some("light"), 11, "m_b")));
    assert_eq!(
        kept.get("app", "appearance.theme")
            .and_then(|c| c.value.as_deref()),
        Some("light")
    );
}

#[test]
fn merging_lands_on_the_same_doc_in_any_order_and_any_number_of_times() {
    let a = doc(&[
        ("app", "appearance.theme", Some("dark"), 10, "m_a"),
        ("origin:1", "icon", Some("rocket"), 30, "m_a"),
    ]);
    let b = doc(&[
        ("app", "appearance.theme", Some("light"), 10, "m_b"),
        ("origin:1", "icon", None, 20, "m_b"),
        ("origin:1", "color", Some("#ff0000"), 5, "m_b"),
    ]);
    let c = doc(&[("app", "terminal.confirm_stop", Some("true"), 1, "m_c")]);

    let mut abc = a.clone();
    abc.merge(&b);
    abc.merge(&c);
    let mut cba = c.clone();
    cba.merge(&b);
    cba.merge(&a);
    assert_eq!(abc, cba, "order does not matter");

    let mut twice = abc.clone();
    assert!(!twice.merge(&abc), "merging what it holds changes nothing");
    assert!(!twice.merge(&b));
    assert_eq!(twice, abc);

    // The tie at 10 ms goes to m_b everywhere.
    assert_eq!(
        abc.get("app", "appearance.theme")
            .and_then(|c| c.value.as_deref()),
        Some("light")
    );
    assert_eq!(abc.digest(), cba.digest());
}

#[test]
fn a_cleared_field_wins_over_an_earlier_value_and_reaches_the_others() {
    let mut kept = doc(&[("origin:1", "icon", Some("rocket"), 10, "m_a")]);
    kept.merge(&doc(&[("origin:1", "icon", None, 20, "m_b")]));
    let icon = kept.get("origin:1", "icon").expect("still a field");
    assert_eq!(icon.value, None);
}

#[test]
fn words_in_a_later_shape_are_refused_rather_than_written_over() {
    let kept = doc(&[("app", "appearance.theme", Some("dark"), 10, "m_a")]);
    assert_eq!(Doc::from_words(&kept.to_words()).expect("read"), kept);
    let later = format!(r#"{{"format":{},"records":{{}}}}"#, FORMAT + 1);
    assert!(Doc::from_words(later.as_bytes()).is_err());
    assert!(Doc::from_words(b"not json").is_err());
}
