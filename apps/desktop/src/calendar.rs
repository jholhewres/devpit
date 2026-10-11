//! Card dates and reminders as a calendar on devpit.app, for any calendar app
//! — Google's included — to subscribe to. This machine sends the events when
//! they change; devpit.app serves them at an address with a secret in it,
//! which the person can renew or end. By default an event says only that
//! something is due: a card's title and project leave this machine only when
//! titles are on.

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use devpit_core::preference;
use devpit_rpc::{CalendarFeed, ErrorCode, RpcError};
use serde::Deserialize;
use serde_json::{json, Value};

/// A change is sent once it settles this long, so a burst of edits is one send.
const SETTLE: Duration = Duration::from_secs(5);
/// How often the calendar is looked at anyway: a card renamed tells no one.
const EVERY: Duration = Duration::from_secs(10 * 60);
/// What devpit.app takes: at most this many events, these long, in this span.
const MOST_EVENTS: usize = 1000;
const LONGEST_TITLE: usize = 200;
const LONGEST_PROJECT: usize = 100;
const EARLIEST: i64 = 946_684_800;
const LATEST: i64 = 7_258_118_400;

/// One dated card, as the calendar shows it.
pub(crate) struct Dated {
    pub card_id: String,
    pub title: String,
    pub project: String,
    /// Seconds since the epoch.
    pub at: i64,
}

fn cut(text: &str, most: usize) -> String {
    text.chars().take(most).collect()
}

/// What leaves this machine for `dated`: a uid and a moment for each, and
/// the card's title and project only when `titles` is on.
pub(crate) fn entries(dated: &[Dated], titles: bool) -> Value {
    let events: Vec<Value> = dated
        .iter()
        .filter(|one| (EARLIEST..=LATEST).contains(&one.at))
        .take(MOST_EVENTS)
        .map(|one| {
            json!({
                "uid": one.card_id,
                "at": one.at as f64,
                "title": titles.then(|| cut(&one.title, LONGEST_TITLE)),
                "project": titles
                    .then(|| cut(&one.project, LONGEST_PROJECT))
                    .filter(|project| !project.is_empty()),
            })
        })
        .collect();
    json!({ "events": events })
}

/// Whether `body` goes to devpit.app: only when it is not what it last took.
pub(crate) fn worth_sending(body: &str, last: Option<&str>) -> bool {
    last != Some(body)
}

/// The body devpit.app last took, so an unchanged calendar is not sent again.
static SENT: Mutex<Option<String>> = Mutex::new(None);

fn store() -> Result<devpit_core::Store, RpcError> {
    crate::projects::store()
}

fn link() -> Option<String> {
    store()
        .ok()?
        .preference(preference::CALENDAR_LINK)
        .ok()
        .flatten()
        .filter(|kept| !kept.is_empty())
}

fn titles() -> bool {
    store()
        .ok()
        .and_then(|store| {
            store
                .preference_flag(preference::CALENDAR_TITLES)
                .ok()
                .flatten()
        })
        .unwrap_or(false)
}

fn feed() -> CalendarFeed {
    CalendarFeed {
        link: link(),
        titles: titles(),
        signed_in: crate::account::token::read().is_some(),
    }
}

/// Every date not yet dealt with, gone off or still to come, soonest first.
fn dated() -> Result<Vec<Dated>, RpcError> {
    let names = crate::projects::project_list_unread_now()
        .map(|listed| listed.projects)
        .unwrap_or_default();
    Ok(store()?
        .reminders_open(None)?
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
        .collect())
}

fn token() -> Result<String, RpcError> {
    crate::account::token::read().ok_or_else(|| {
        RpcError::new(
            ErrorCode::Invalid,
            "sign in to devpit.app on this computer first",
        )
    })
}

fn unreachable(_: reqwest::Error) -> RpcError {
    RpcError::internal("devpit.app did not answer")
}

/// Sends the calendar when it changed, or `always`. Nothing without an
/// address or a sign-in.
async fn send(always: bool) -> Result<(), RpcError> {
    let (Some(_), Some(token)) = (link(), crate::account::token::read()) else {
        return Ok(());
    };
    let titles = titles();
    let body =
        crate::off_main::blocking(move || Ok(entries(&dated()?, titles).to_string())).await?;
    let last = SENT.lock().ok().and_then(|held| held.clone());
    if !always && !worth_sending(&body, last.as_deref()) {
        return Ok(());
    }
    let answer = crate::account::client()?
        .post(format!("{}/v1/calendar/events", crate::account::origin()))
        .bearer_auth(token)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body.clone())
        .send()
        .await
        .map_err(unreachable)?;
    match answer.status() {
        status if status.is_success() => {
            if let Ok(mut held) = SENT.lock() {
                *held = Some(body);
            }
            Ok(())
        }
        // Ended elsewhere: this machine stops holding an address that is gone.
        reqwest::StatusCode::NOT_FOUND => {
            store()?.set_preference(preference::CALENDAR_LINK, "")?;
            Ok(())
        }
        status => Err(RpcError::internal(format!("devpit.app answered {status}"))),
    }
}

fn nudged() -> &'static tokio::sync::Notify {
    static NUDGED: OnceLock<tokio::sync::Notify> = OnceLock::new();
    NUDGED.get_or_init(tokio::sync::Notify::new)
}

/// A card's date changed: the calendar goes again once it settles.
pub(crate) fn nudge() {
    nudged().notify_one();
}

/// Keeps devpit.app's copy of the calendar current while the app runs.
pub(crate) fn keep_sent() {
    nudge();
    tauri::async_runtime::spawn(async {
        loop {
            let _ = tokio::time::timeout(EVERY, nudged().notified()).await;
            tokio::time::sleep(SETTLE).await;
            if let Err(why) = send(false).await {
                devpit_core::reports::background("calendar", &why.message);
            }
        }
    });
}

#[derive(Deserialize)]
struct Made {
    url: String,
}

/// A new address from devpit.app, kept here; the one before stops answering.
async fn renew() -> Result<(), RpcError> {
    let made: Made = crate::account::client()?
        .post(format!("{}/v1/calendar/link", crate::account::origin()))
        .bearer_auth(token()?)
        .send()
        .await
        .map_err(unreachable)?
        .error_for_status()
        .map_err(|err| {
            RpcError::internal(format!(
                "devpit.app refused a calendar address: {}",
                err.without_url()
            ))
        })?
        .json()
        .await
        .map_err(unreachable)?;
    store()?.set_preference(preference::CALENDAR_LINK, &made.url)?;
    Ok(())
}

/// `calendar.read`
#[tauri::command]
#[specta::specta]
pub async fn calendar_read() -> Result<CalendarFeed, RpcError> {
    crate::off_main::blocking(|| Ok(feed())).await
}

/// `calendar.set` — the calendar on devpit.app on or off, and whether its
/// events name the card. Off ends the address and the events there.
#[tauri::command]
#[specta::specta]
pub async fn calendar_set(on: bool, titles: bool) -> Result<CalendarFeed, RpcError> {
    store()?.set_preference_flag(preference::CALENDAR_TITLES, titles)?;
    match (on, link().is_some()) {
        (true, false) => {
            renew().await?;
            send(true).await?;
        }
        (false, true) => {
            crate::account::client()?
                .delete(format!("{}/v1/calendar", crate::account::origin()))
                .bearer_auth(token()?)
                .send()
                .await
                .map_err(unreachable)?
                .error_for_status()
                .map_err(|err| {
                    RpcError::internal(format!(
                        "devpit.app did not end the calendar: {}",
                        err.without_url()
                    ))
                })?;
            store()?.set_preference(preference::CALENDAR_LINK, "")?;
            if let Ok(mut held) = SENT.lock() {
                *held = None;
            }
        }
        (true, true) => send(false).await?,
        (false, false) => {}
    }
    Ok(feed())
}

/// `calendar.reset` — a new address; the old one stops answering at once.
#[tauri::command]
#[specta::specta]
pub async fn calendar_reset() -> Result<CalendarFeed, RpcError> {
    renew().await?;
    Ok(feed())
}

#[cfg(test)]
#[path = "calendar_tests.rs"]
mod tests;
