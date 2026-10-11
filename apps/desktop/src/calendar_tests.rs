use super::{entries, worth_sending, Dated};

fn dated(title: &str, at: i64) -> Dated {
    Dated {
        card_id: "card_01J".to_owned(),
        title: title.to_owned(),
        project: "api".to_owned(),
        at,
    }
}

#[test]
fn by_default_no_card_words_leave_this_machine() {
    let sent = entries(&[dated("Rotate the prod keys", 1_791_498_600)], false).to_string();
    assert!(!sent.contains("Rotate"));
    assert!(!sent.contains("api"));
    let event = &entries(&[dated("Rotate the prod keys", 1_791_498_600)], false)["events"][0];
    assert_eq!(event["uid"], "card_01J");
    assert_eq!(event["at"], 1_791_498_600.0);
    assert!(event["title"].is_null() && event["project"].is_null());
}

#[test]
fn with_titles_on_an_event_carries_its_card_and_project() {
    let event = &entries(&[dated("Rotate the prod keys", 1_791_498_600)], true)["events"][0];
    assert_eq!(event["title"], "Rotate the prod keys");
    assert_eq!(event["project"], "api");
}

#[test]
fn what_devpit_app_would_refuse_is_cut_or_left_out_here() {
    let long = "t".repeat(500);
    let sent = entries(&[dated(&long, 1_791_498_600), dated("1969", -60)], true);
    let events = sent["events"].as_array().expect("events");
    assert_eq!(events.len(), 1, "a date before 2000 is left out");
    assert_eq!(events[0]["title"].as_str().map(str::len), Some(200));
}

#[test]
fn an_unchanged_calendar_is_not_sent_again() {
    let body = entries(&[dated("x", 1_791_498_600)], false).to_string();
    assert!(worth_sending(&body, None));
    assert!(!worth_sending(&body, Some(&body)));
    let moved = entries(&[dated("x", 1_791_498_660)], false).to_string();
    assert!(worth_sending(&moved, Some(&body)));
}
