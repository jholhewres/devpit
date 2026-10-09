//! Card dates and reminders as a calendar, served by the Remote at an address
//! with a secret in it — so only from this machine, over the owner's tailnet:
//! a calendar app on one of their devices subscribes to it, and no server of
//! anyone else's ever holds it.

use devpit_core::preference;
use devpit_rpc::{CalendarLink, RpcError};

/// One dated card, as the calendar shows it.
pub(crate) struct Dated {
    pub card_id: String,
    pub title: String,
    pub project: String,
    /// Seconds since the epoch.
    pub at: i64,
}

/// Text as iCalendar keeps it: backslash, comma, semicolon and newline escaped.
pub(crate) fn escaped(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\r', "")
        .replace('\n', "\\n")
}

fn stamp(epoch: i64) -> String {
    let day = devpit_agentcli::spend_scan::day_of(epoch).replace('-', "");
    let of_day = epoch.rem_euclid(86_400);
    format!(
        "{day}T{:02}{:02}{:02}Z",
        of_day / 3600,
        of_day % 3600 / 60,
        of_day % 60
    )
}

/// The calendar for `dated`, made at `now`.
pub(crate) fn calendar(dated: &[Dated], now: i64) -> String {
    let mut out = String::from(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//devpit//cards//EN\r\nX-WR-CALNAME:devpit\r\n",
    );
    for one in dated {
        out.push_str(&format!(
            "BEGIN:VEVENT\r\nUID:{}@devpit\r\nDTSTAMP:{}\r\nDTSTART:{}\r\nDURATION:PT15M\r\nSUMMARY:{}\r\nDESCRIPTION:{}\r\nEND:VEVENT\r\n",
            one.card_id,
            stamp(now),
            stamp(one.at),
            escaped(&one.title),
            escaped(&one.project)
        ));
    }
    out.push_str("END:VCALENDAR\r\n");
    out
}

fn token(create: bool) -> Result<Option<String>, RpcError> {
    let store = crate::projects::store()?;
    if let Some(kept) = store
        .preference(preference::CALENDAR_TOKEN)?
        .filter(|kept| !kept.is_empty())
    {
        return Ok(Some(kept));
    }
    if !create {
        return Ok(None);
    }
    let fresh = format!("{}{}", ulid::Ulid::generate(), ulid::Ulid::generate()).to_lowercase();
    store.set_preference(preference::CALENDAR_TOKEN, &fresh)?;
    Ok(Some(fresh))
}

/// Whether `path` is the calendar's address, `/cal/<token>.ics`.
pub(crate) fn asked(path: &str, kept: &str) -> bool {
    let Some(given) = path
        .strip_prefix("/cal/")
        .and_then(|rest| rest.strip_suffix(".ics"))
    else {
        return false;
    };
    // Compared whole, so its time says nothing about how much matched.
    given.len() == kept.len()
        && given
            .bytes()
            .zip(kept.bytes())
            .fold(0u8, |diff, (a, b)| diff | (a ^ b))
            == 0
}

/// The calendar when `path` is its address, else nothing.
pub(crate) fn served(path: &str) -> Option<String> {
    let kept = token(false).ok().flatten()?;
    if !asked(path, &kept) {
        return None;
    }
    let store = crate::projects::store().ok()?;
    let names = crate::projects::project_list_unread_now()
        .map(|listed| listed.projects)
        .unwrap_or_default();
    let dated: Vec<Dated> = store
        .reminders_open(None)
        .ok()?
        .into_iter()
        .map(|row| Dated {
            project: names
                .iter()
                .find(|one| one.id == row.project_id)
                .map(|one| one.name.clone())
                .unwrap_or_default(),
            card_id: row.card_id,
            title: row.title,
            at: row.due_at,
        })
        .collect();
    Some(calendar(&dated, devpit_core::reports::now() as i64))
}

/// `calendar.link` — the calendar's path on the Remote, made the first time.
#[tauri::command]
#[specta::specta]
pub async fn calendar_link() -> Result<CalendarLink, RpcError> {
    crate::off_main::blocking(|| {
        Ok(CalendarLink {
            path: format!("/cal/{}.ics", token(true)?.unwrap_or_default()),
        })
    })
    .await
}

/// `calendar.renew` — a new address; the old one stops answering.
#[tauri::command]
#[specta::specta]
pub async fn calendar_renew() -> Result<CalendarLink, RpcError> {
    crate::off_main::blocking(|| {
        crate::projects::store()?.set_preference(preference::CALENDAR_TOKEN, "")?;
        Ok(CalendarLink {
            path: format!("/cal/{}.ics", token(true)?.unwrap_or_default()),
        })
    })
    .await
}

#[cfg(test)]
#[path = "calendar_tests.rs"]
mod tests;
