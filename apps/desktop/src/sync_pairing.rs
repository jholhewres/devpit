//! The sync key, carried between two machines of the account: an offer this
//! machine makes on devpit.app's relay, or one from another machine that
//! this one takes — or the recovery code the person wrote down.
//!
//! An offer is good once, for two minutes, and only to a machine signed in
//! to the same account that proves it holds the offer's secret; devpit.app
//! carries the session and holds no key to it (`devpit_sync::pairing`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use devpit_core::{preference, Store};
use devpit_rpc::{ErrorCode, RpcError, SyncOffer, SyncStatus};
use devpit_sync::pairing::{self, Offer, GOOD_FOR};
use devpit_sync::{files, SyncKey};

use crate::sync::{round_waited, set_problem, status, take_key, tell, token, CHANGED};

struct Offering {
    shown: SyncOffer,
    stop: Arc<AtomicBool>,
}

static OFFERING: Mutex<Option<Offering>> = Mutex::new(None);

fn refused(why: impl Into<String>) -> RpcError {
    RpcError::new(ErrorCode::Invalid, why.into())
}

fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs_f64())
        .unwrap_or_default()
}

/// The offer this machine is making, while it lasts.
pub(crate) fn offering() -> Option<SyncOffer> {
    OFFERING
        .lock()
        .ok()?
        .as_ref()
        .map(|held| held.shown.clone())
        .filter(|shown| shown.expires_at > now())
}

/// Ends the offer on show, if there is one.
pub(crate) fn withdraw() {
    if let Some(held) = OFFERING.lock().ok().and_then(|mut held| held.take()) {
        held.stop.store(true, Ordering::SeqCst);
    }
}

/// `sync.offer` — an offer another machine of the account takes the key
/// with, in place of any before it.
#[tauri::command]
#[specta::specta]
pub async fn sync_offer() -> Result<SyncStatus, RpcError> {
    let token = token()?;
    let root = Store::root()?;
    let key =
        files::key(&root).ok_or_else(|| refused("this computer holds no sync key to give"))?;
    let (offer, keys) =
        Offer::new(&crate::remote_conn::machine_name()).map_err(RpcError::internal)?;
    withdraw();
    let stop = Arc::new(AtomicBool::new(false));
    let shown = SyncOffer {
        text: offer.text(),
        expires_at: now() + GOOD_FOR.as_secs_f64(),
    };
    if let Ok(mut held) = OFFERING.lock() {
        *held = Some(Offering {
            shown: shown.clone(),
            stop: stop.clone(),
        });
    }
    let origin = crate::account::origin();
    std::thread::spawn(move || {
        let relay = devpit_remote::Relay::new(&origin, &token, &offer.host);
        let ended = pairing::offer(
            &relay,
            &keys,
            &offer.secret,
            &key,
            Instant::now() + GOOD_FOR,
            &stop,
        );
        let ours = match OFFERING.lock() {
            Ok(mut held) if held.as_ref().is_some_and(|one| one.shown == shown) => {
                *held = None;
                true
            }
            _ => false,
        };
        if ours {
            // Withdrawn by the person is not a problem; anything else is.
            match ended {
                Ok(_) => set_problem(None),
                Err(why) if !stop.load(Ordering::SeqCst) => set_problem(Some(why)),
                Err(_) => {}
            }
        }
        tell(CHANGED);
    });
    crate::off_main::blocking(status).await
}

/// `sync.offer_withdraw` — the offer on show stops being good.
#[tauri::command]
#[specta::specta]
pub async fn sync_offer_withdraw() -> Result<SyncStatus, RpcError> {
    withdraw();
    crate::off_main::blocking(status).await
}

/// The key `given` holds: another machine's offer, taken through the relay,
/// or the recovery code.
async fn key_from(given: String) -> Result<SyncKey, RpcError> {
    if !given.trim().starts_with(pairing::PREFIX) {
        return SyncKey::from_recovery_code(&given).map_err(|err| refused(err.to_string()));
    }
    let offer = Offer::parse(&given).map_err(refused)?;
    let token = token()?;
    let origin = crate::account::origin();
    crate::off_main::blocking(move || {
        pairing::join(&origin, &token, &offer, &crate::remote_conn::machine_name()).map_err(refused)
    })
    .await
}

/// `sync.join` — this machine takes the account's key, from another
/// machine's offer or from the recovery code, and syncs with it.
#[tauri::command]
#[specta::specta]
pub async fn sync_join(given: String) -> Result<SyncStatus, RpcError> {
    take_key(key_from(given).await?).await?;
    Store::open_default()?.set_preference_flag(preference::SYNC, true)?;
    set_problem(None);
    round_waited().await;
    crate::off_main::blocking(status).await
}
