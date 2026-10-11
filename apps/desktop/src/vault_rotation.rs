//! When a project's secret is due to be rotated, and the reminder that says so.
//!
//! A reminder is a card with a date (`reminders.rs`), so a rotation is one
//! too: a card on the project's board, "Rotate NAME", whose date goes off like
//! any other's — no clock of its own. What a rotation adds — the interval, the
//! date it is due, its card — is kept with the names in `secrets.json`, never
//! in the `.env`.

use std::path::Path;

use devpit_core::Store;
use devpit_rpc::{ErrorCode, RpcError, Secret, SecretList};
use tauri::{AppHandle, Emitter};

use crate::vault::{listed, now, write_list};

const DAY: i64 = 24 * 60 * 60;
/// Ten years: past that, nobody is keeping a rotation.
const LONGEST_DAYS: u32 = 3650;

const BODY: &str = "devpit reminds you to rotate this secret. Make a new key where it was \
issued and keep it in Edit project → Secrets: the date moves on by itself. The value stays in \
the project's .env; this card carries only the date.";

/// When a secret set at `set_at` is next due: the date asked for, else one
/// interval after it was set — already past for a key that old.
pub(crate) fn due(set_at: i64, every_days: Option<u32>, at: Option<i64>) -> Option<i64> {
    at.or_else(|| every_days.map(|days| set_at + i64::from(days) * DAY))
}

/// The next date for a secret rotated at `now`: another interval, or none
/// when it had only a date.
pub(crate) fn after_rotation(now: i64, every_days: Option<u32>) -> Option<i64> {
    every_days.map(|days| now + i64::from(days) * DAY)
}

/// The reminder's card names the secret, never its value.
pub(crate) fn title(name: &str) -> String {
    format!("Rotate {name}")
}

fn refused(why: &str) -> RpcError {
    RpcError::new(ErrorCode::Invalid, why.to_owned())
}

/// Sets a secret's rotation — an interval, a date, or both; with neither, it
/// stops, and its card is put away.
pub(crate) fn plan(
    store: &Store,
    project_id: &str,
    kept: &Path,
    name: &str,
    every_days: Option<u32>,
    at: Option<i64>,
    now: i64,
) -> Result<Vec<Secret>, RpcError> {
    if every_days.is_some_and(|days| days == 0 || days > LONGEST_DAYS) {
        return Err(refused("a rotation is every 1 to 3650 days"));
    }
    if at.is_some_and(|at| at <= now) {
        return Err(refused("a rotation date is one still to come"));
    }
    change(store, project_id, kept, name, now, |secret| {
        secret.rotate_days = every_days;
        secret.rotate_at = due(secret.set_at as i64, every_days, at).map(|at| at as f64);
    })
}

/// "Rotated": due again an interval from now, or done with when it had only
/// a date.
pub(crate) fn rotated(
    store: &Store,
    project_id: &str,
    kept: &Path,
    name: &str,
    now: i64,
) -> Result<Vec<Secret>, RpcError> {
    change(store, project_id, kept, name, now, |secret| {
        secret.rotate_at = after_rotation(now, secret.rotate_days).map(|at| at as f64);
    })
}

/// A new value for a secret with a rotation is that rotation done. Says
/// whether it had one.
pub(crate) fn kept_again(
    store: &Store,
    project_id: &str,
    kept: &Path,
    name: &str,
    now: i64,
) -> Result<bool, RpcError> {
    let had = listed(kept)
        .iter()
        .any(|one| one.name == name && (one.rotate_at.is_some() || one.reminder_card.is_some()));
    if had {
        rotated(store, project_id, kept, name, now)?;
    }
    Ok(had)
}

fn change(
    store: &Store,
    project_id: &str,
    kept: &Path,
    name: &str,
    now: i64,
    how: impl FnOnce(&mut Secret),
) -> Result<Vec<Secret>, RpcError> {
    let mut list = listed(kept);
    let secret = list
        .iter_mut()
        .find(|one| one.name == name)
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "this project keeps no such secret"))?;
    how(secret);
    remind(store, project_id, secret, now)?;
    write_list(kept, &list)?;
    Ok(list)
}

/// The card follows the date: moved, made, or put away when there is none.
fn remind(store: &Store, project_id: &str, secret: &mut Secret, now: i64) -> Result<(), RpcError> {
    if let Some(card) = &secret.reminder_card {
        // What the bell said about the old date is not news any more.
        store.read_reminder_notices(card, crate::reminders::KIND, now)?;
    }
    let Some(at) = secret.rotate_at else {
        if let Some(card) = secret.reminder_card.take() {
            store.archive_card(&card)?;
        }
        return Ok(());
    };
    if let Some(card) = &secret.reminder_card {
        // False once the person archived it: then a new one is made.
        if store.set_card_due(card, Some(at as i64), false)? {
            return Ok(());
        }
    }
    let column = store
        .columns(project_id)?
        .into_iter()
        .next()
        .ok_or_else(|| RpcError::new(ErrorCode::Conflict, "this project's board has no columns"))?;
    let card = store.create_card(project_id, &column.id, &title(&secret.name), BODY)?;
    store.set_card_due(&card, Some(at as i64), false)?;
    secret.reminder_card = Some(card);
    Ok(())
}

/// The secret a reminder's card is about, by name, when it is one.
pub(crate) fn secret_on(store: &Store, project_id: &str, card_id: &str) -> Option<String> {
    let root = Store::root().ok()?;
    let kept = crate::projects::home_of(store, &root, project_id)
        .ok()?
        .secrets();
    listed(&kept)
        .into_iter()
        .find(|one| one.reminder_card.as_deref() == Some(card_id))
        .map(|one| one.name)
}

/// A channel's button, by the card it came with: rotated, or forgotten.
pub(crate) fn by_card(card_id: &str, rotate: bool) -> Result<String, String> {
    let store = crate::projects::store().map_err(|err| err.message)?;
    let project_id = store
        .reminder(card_id)
        .map_err(|err| err.to_string())?
        .map(|row| row.project_id)
        .ok_or("That reminder is no longer waiting.")?;
    let name = secret_on(&store, &project_id, card_id).ok_or("That secret has no rotation.")?;
    let kept = crate::projects::project_home(&project_id)
        .map_err(|err| err.message)?
        .secrets();
    let said = if rotate {
        let secrets =
            rotated(&store, &project_id, &kept, &name, now()).map_err(|err| err.message)?;
        match secrets
            .iter()
            .find(|one| one.name == name)
            .and_then(|one| one.rotate_at)
        {
            Some(_) => format!("{name}: the next rotation is set."),
            None => format!("{name}: rotated, and no rotation is left."),
        }
    } else {
        plan(&store, &project_id, &kept, &name, None, None, now()).map_err(|err| err.message)?;
        format!("{name}: no more reminders to rotate it.")
    };
    if let Some(app) = crate::telegram::app().get() {
        told(app, &project_id);
    }
    Ok(said)
}

/// The banner, the tray and the board read again.
pub(crate) fn told(app: &AppHandle, project_id: &str) {
    crate::reminders::changed(app);
    let _ = app.emit("board:changed", project_id);
}

fn listing(project_id: &str, secrets: Vec<Secret>) -> Result<SecretList, RpcError> {
    let root = crate::roots::root_of(project_id, None)?;
    Ok(SecretList {
        secrets,
        file: root.join(crate::vault::FILE).display().to_string(),
    })
}

/// `project.secret_rotation` — when a secret is to be rotated: every so many
/// days, by a date, or, with neither, no longer.
#[tauri::command]
#[specta::specta]
pub async fn project_secret_rotation(
    app: AppHandle,
    project_id: String,
    name: String,
    every_days: Option<u32>,
    at: Option<f64>,
) -> Result<SecretList, RpcError> {
    crate::off_main::blocking(move || {
        if at.is_some_and(|at| !at.is_finite()) {
            return Err(refused("that is not a date"));
        }
        let store = crate::projects::store()?;
        let kept = crate::projects::project_home(&project_id)?.secrets();
        let at = at.map(|at| at as i64);
        let secrets = plan(&store, &project_id, &kept, &name, every_days, at, now())?;
        told(&app, &project_id);
        listing(&project_id, secrets)
    })
    .await
}

/// `project.secret_rotated` — the person rotated it: the date moves on.
#[tauri::command]
#[specta::specta]
pub async fn project_secret_rotated(
    app: AppHandle,
    project_id: String,
    name: String,
) -> Result<SecretList, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let kept = crate::projects::project_home(&project_id)?.secrets();
        let secrets = rotated(&store, &project_id, &kept, &name, now())?;
        told(&app, &project_id);
        listing(&project_id, secrets)
    })
    .await
}

#[cfg(test)]
#[path = "vault_rotation_tests.rs"]
mod tests;
