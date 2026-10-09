use super::{asked, calendar, escaped, Dated};

#[test]
fn a_dated_card_is_an_event_at_its_moment() {
    let said = calendar(
        &[Dated {
            card_id: "card_1".to_owned(),
            title: "Review the PR, today".to_owned(),
            project: "api".to_owned(),
            at: 1_791_498_600,
        }],
        0,
    );
    assert!(said.starts_with("BEGIN:VCALENDAR\r\n"));
    assert!(said.contains("UID:card_1@devpit\r\n"));
    assert!(said.contains("DTSTART:20261008T223000Z\r\n"));
    assert!(said.contains("SUMMARY:Review the PR\\, today\r\n"));
    assert!(said.ends_with("END:VCALENDAR\r\n"));
}

#[test]
fn text_cannot_break_out_of_its_line() {
    assert_eq!(escaped("a;b\nEND:VEVENT"), "a\\;b\\nEND:VEVENT");
}

#[test]
fn only_the_address_with_the_secret_is_the_calendar() {
    assert!(asked("/cal/abc123.ics", "abc123"));
    assert!(!asked("/cal/abc124.ics", "abc123"));
    assert!(!asked("/cal/abc12.ics", "abc123"));
    assert!(!asked("/cal/abc123", "abc123"));
    assert!(!asked("/remote.html", "abc123"));
}
