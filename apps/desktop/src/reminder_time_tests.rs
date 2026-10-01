use super::{instant_of, span_of};

#[test]
fn a_moment_with_its_offset_is_read_to_the_second() {
    // 2026-10-02T18:00:00Z
    let utc = 1_790_964_000;
    assert_eq!(instant_of("2026-10-02T18:00:00Z"), Ok(utc));
    assert_eq!(instant_of("2026-10-02T15:00:00-03:00"), Ok(utc));
    assert_eq!(instant_of("2026-10-02T15:00-03:00"), Ok(utc));
    assert_eq!(instant_of("2026-10-02 23:30:00+05:30"), Ok(utc));
    assert_eq!(instant_of("2026-10-02T15:00:00.250-0300"), Ok(utc));
    assert_eq!(instant_of("1970-01-01T00:00:00Z"), Ok(0));
    assert_eq!(instant_of("2028-02-29T00:00:00Z"), Ok(1_835_395_200));
}

#[test]
fn a_moment_without_an_offset_or_out_of_range_is_refused() {
    for wrong in [
        "2026-10-02T15:00:00",
        "2026-10-02",
        "tomorrow at 3",
        "2026-13-02T15:00:00Z",
        "2026-02-30T15:00:00Z",
        "2027-02-29T15:00:00Z",
        "2026-10-02T24:00:00Z",
        "2026-10-02T15:61:00Z",
        "2026-10-02T15:00:00+15:00",
        "26-10-02T15:00:00Z",
    ] {
        assert!(instant_of(wrong).is_err(), "{wrong} was read");
    }
}

#[test]
fn a_span_is_minutes_hours_or_days() {
    assert_eq!(span_of("15m"), Ok(900));
    assert_eq!(span_of("2h"), Ok(7200));
    assert_eq!(span_of("1d"), Ok(86_400));
    for wrong in ["", "m", "0m", "-5m", "15", "1w", "999d"] {
        assert!(span_of(wrong).is_err(), "{wrong} was read");
    }
}
