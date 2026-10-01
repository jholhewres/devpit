//! A moment as an agent writes it, read without a clock library.
//!
//! ISO 8601 with its offset — `2026-10-02T15:00:00-03:00`, `…T18:00Z` — and
//! nothing looser. A moment without an offset is refused: read here, it
//! would mean whatever this machine's zone says, which is not necessarily
//! the zone of the person who asked to be reminded.

/// Seconds since the epoch of `text`, or why it is not a moment.
pub(crate) fn instant_of(text: &str) -> Result<i64, String> {
    let bad = || {
        format!("`{text}` is not a moment: write it as 2026-10-02T15:00:00-03:00, with its offset")
    };
    let text = text.trim();
    let (date, rest) = text.split_once(['T', ' ']).ok_or_else(bad)?;
    let mut day = date.split('-');
    let (Some(year), Some(month), Some(dom), None) =
        (day.next(), day.next(), day.next(), day.next())
    else {
        return Err(bad());
    };
    let number = |part: &str, len: usize| {
        (part.len() == len && part.bytes().all(|b| b.is_ascii_digit()))
            .then(|| part.parse::<i64>().ok())
            .flatten()
    };
    let (year, month, dom) = (
        number(year, 4).ok_or_else(bad)?,
        number(month, 2).ok_or_else(bad)?,
        number(dom, 2).ok_or_else(bad)?,
    );

    // The offset is what follows the time: `Z`, or a sign and hours[:minutes].
    let (clock, offset) = if let Some(clock) = rest.strip_suffix(['Z', 'z']) {
        (clock, 0)
    } else {
        let at = rest.rfind(['+', '-']).ok_or_else(bad)?;
        let (clock, zone) = rest.split_at(at);
        let sign = if zone.starts_with('-') { -1 } else { 1 };
        let zone = &zone[1..];
        let (hours, minutes) = match zone.split_once(':') {
            Some((hours, minutes)) => (hours, minutes),
            None if zone.len() == 4 => zone.split_at(2),
            None => (zone, "00"),
        };
        let (hours, minutes) = (
            number(hours, 2).ok_or_else(bad)?,
            number(minutes, 2).ok_or_else(bad)?,
        );
        if hours > 14 || minutes > 59 {
            return Err(bad());
        }
        (clock, sign * (hours * 3600 + minutes * 60))
    };
    // Fractions of a second are dropped: a reminder is not that precise.
    let clock = clock.split('.').next().unwrap_or_default();
    let mut parts = clock.split(':');
    let hour = number(parts.next().unwrap_or_default(), 2).ok_or_else(bad)?;
    let minute = number(parts.next().ok_or_else(bad)?, 2).ok_or_else(bad)?;
    let second = match parts.next() {
        Some(second) => number(second, 2).ok_or_else(bad)?,
        None => 0,
    };
    if parts.next().is_some() || hour > 23 || minute > 59 || second > 60 {
        return Err(bad());
    }
    if !(1..=12).contains(&month) || dom < 1 || dom > days_in(year, month) {
        return Err(bad());
    }
    Ok(days_from_civil(year, month, dom) * 86_400 + hour * 3600 + minute * 60 + second - offset)
}

/// How long `15m`, `2h` or `1d` is, in seconds.
pub(crate) fn span_of(text: &str) -> Result<i64, String> {
    let text = text.trim();
    let bad = || format!("`{text}` is not a span: write 15m, 2h or 1d");
    let (count, unit) = text.split_at(text.len().saturating_sub(1));
    let count: i64 = count.parse().map_err(|_| bad())?;
    let unit = match unit {
        "m" => 60,
        "h" => 3600,
        "d" => 86_400,
        _ => return Err(bad()),
    };
    if !(1..=366 * 86_400 / unit).contains(&count) {
        return Err(bad());
    }
    Ok(count * unit)
}

fn days_in(year: i64, month: i64) -> i64 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Howard Hinnant's).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * 146_097 + of_cycle - 719_468
}

#[cfg(test)]
#[path = "reminder_time_tests.rs"]
mod tests;
