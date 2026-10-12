//! Sync between the person's machines, through devpit.app, which cannot read
//! what it keeps — `devpit-sync`, and `docs/sync.md` for the design.
//!
//! Off unless turned on, per machine. One thread runs the rounds, one at a
//! time: soon after something here changed, every few minutes to hear the
//! other machines, and when a command asks. The key is kept in devpit's
//! home at mode 600; it leaves this machine only inside an offer's Noise
//! session (`sync_pairing`) or as the recovery code the person writes down.

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use devpit_core::{preference, Store};
use devpit_rpc::{ErrorCode, RpcError, SyncMachine, SyncRecovery, SyncStatus};
use devpit_sync::machines::{self, Machines};
use devpit_sync::prefs::Prefs;
use devpit_sync::projects::Projects;
use devpit_sync::{files, Blobs, DevpitApp, Namespace, SyncError, SyncKey};
use tauri::{AppHandle, Emitter};

/// How often the other machines are listened for when nothing changed here.
const EVERY: Duration = Duration::from_secs(3 * 60);
/// A burst of changes here is one round.
const SETTLE: Duration = Duration::from_secs(3);

/// What the settings show, every round: read again.
pub(crate) const CHANGED: &str = "sync:changed";
/// Values another machine changed were written here: the window reads its
/// settings and projects again.
const WRITTEN: &str = "sync:written";

/// A round asked of the worker; `done` hears when it is over.
struct Ask {
    done: Option<tokio::sync::oneshot::Sender<()>>,
}

static WORKER: OnceLock<Sender<Ask>> = OnceLock::new();
static APP: OnceLock<AppHandle> = OnceLock::new();
/// Why the last round stopped, until one goes through.
static PROBLEM: Mutex<Option<String>> = Mutex::new(None);
/// Held by a round and by whatever replaces the key, so a round never keeps
/// a state made under one key beside another.
static FILES: Mutex<()> = Mutex::new(());

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis() as u64)
        .unwrap_or_default()
}

fn root() -> Result<PathBuf, RpcError> {
    Ok(Store::root()?)
}

fn refused(why: impl Into<String>) -> RpcError {
    RpcError::new(ErrorCode::Invalid, why.into())
}

/// Whether sync is on on this machine.
pub(crate) fn on(store: &Store) -> bool {
    store
        .preference_flag(preference::SYNC)
        .ok()
        .flatten()
        .unwrap_or(false)
}

pub(crate) fn token() -> Result<String, RpcError> {
    crate::account::token::read()
        .ok_or_else(|| refused("sign in to devpit.app on this computer first, above"))
}

pub(crate) fn tell(event: &str) {
    if let Some(app) = APP.get() {
        let _ = app.emit(event, ());
    }
}

pub(crate) fn set_problem(problem: Option<String>) {
    if let Ok(mut held) = PROBLEM.lock() {
        *held = problem;
    }
}

/// Something that syncs changed here: a round goes once it settles.
pub(crate) fn nudge() {
    if let Some(worker) = WORKER.get() {
        let _ = worker.send(Ask { done: None });
    }
}

/// A round now, waited for.
pub(crate) async fn round_waited() {
    let Some(worker) = WORKER.get() else {
        return;
    };
    let (done, heard) = tokio::sync::oneshot::channel();
    if worker.send(Ask { done: Some(done) }).is_ok() {
        let _ = heard.await;
    }
}

/// Starts the thread that runs the rounds while the app runs.
pub(crate) fn keep_synced(app: &AppHandle) {
    let _ = APP.set(app.clone());
    let (asks, asked) = channel();
    if WORKER.set(asks).is_ok() {
        std::thread::spawn(move || work(&asked));
        nudge();
    }
}

fn work(asked: &Receiver<Ask>) {
    loop {
        let mut waiting = match asked.recv_timeout(EVERY) {
            Ok(ask) => vec![ask],
            Err(RecvTimeoutError::Timeout) => Vec::new(),
            Err(RecvTimeoutError::Disconnected) => return,
        };
        // Asked by a command, it goes now; nudged by a change, it settles.
        if waiting.iter().all(|ask| ask.done.is_none()) {
            std::thread::sleep(SETTLE);
        }
        waiting.extend(asked.try_iter());
        let wrote = match round_now() {
            Ok(wrote) => wrote,
            Err(err) => {
                devpit_core::reports::background("sync", &err);
                false
            }
        };
        if wrote {
            tell(WRITTEN);
        }
        tell(CHANGED);
        for ask in waiting {
            if let Some(done) = ask.done {
                let _ = done.send(());
            }
        }
    }
}

/// One round on this thread, when sync is on and this machine holds the key:
/// whether it wrote anything here. Its problem, if any, is kept for the
/// settings; a key the account no longer syncs with is forgotten.
fn round_now() -> Result<bool, String> {
    let _held = FILES.lock().map_err(|_| "sync is stuck".to_owned())?;
    let root = Store::root().map_err(|err| err.to_string())?;
    let store = Store::open_default().map_err(|err| err.to_string())?;
    let Some(key) = files::key(&root).filter(|_| on(&store)) else {
        return Ok(false);
    };
    let ended = match crate::account::token::read() {
        None => Err(SyncError::SignedOut),
        Some(token) => run(&root, &store, &key, &token),
    };
    match ended {
        Ok(outcome) => {
            set_problem(outcome.problems.first().cloned());
            Ok(!outcome.written.is_empty())
        }
        Err(err) => {
            if matches!(err, SyncError::OtherKey | SyncError::Reset) {
                files::forget(&root);
            }
            set_problem(Some(err.to_string()));
            Ok(false)
        }
    }
}

/// What syncs from this machine: its preferences, its projects' settings,
/// and itself among the account's machines.
pub(crate) fn namespaces<'a>(
    store: &'a Store,
    machine: &'a str,
    name: &'a str,
    now_ms: u64,
) -> (Prefs<'a>, Projects<'a>, Machines<'a>) {
    (
        Prefs { store },
        Projects { store },
        Machines {
            id: machine,
            name,
            now_ms,
        },
    )
}

fn run(
    root: &std::path::Path,
    store: &Store,
    key: &SyncKey,
    token: &str,
) -> Result<devpit_sync::Outcome, SyncError> {
    let blobs = DevpitApp::new(&crate::account::origin(), token)?;
    let mut state = files::state(root);
    let machine = state.machine.clone();
    let name = crate::remote_conn::machine_name();
    let now = now_ms();
    let (prefs, projects, machines) = namespaces(store, &machine, &name, now);
    let spaces: [&dyn Namespace; 3] = [&prefs, &projects, &machines];
    // A runtime of the worker's own: the round holds the store across its
    // waits, which no task of the app's runtime may.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| SyncError::Local(err.to_string()))?;
    let outcome = runtime.block_on(devpit_sync::round(&blobs, key, &mut state, &spaces, now));
    files::keep_state(root, &state).map_err(SyncError::Local)?;
    outcome
}

/// Puts `key` in place of whatever this machine held, as a machine that
/// never synced: what it knew under another key is no use under this one.
pub(crate) async fn take_key(key: SyncKey) -> Result<(), RpcError> {
    crate::off_main::blocking(move || {
        let _held = FILES
            .lock()
            .map_err(|_| RpcError::internal("sync is stuck"))?;
        let root = Store::root()?;
        files::forget(&root);
        files::keep_key(&root, &key).map_err(RpcError::internal)
    })
    .await
}

/// Where sync stands on this machine.
pub(crate) fn status() -> Result<SyncStatus, RpcError> {
    let root = root()?;
    let store = Store::open_default()?;
    let state = files::state(&root);
    let mut listed = machines::listed(state.mirror("machines"));
    listed.sort_by_key(|one| one.id != state.machine);
    Ok(SyncStatus {
        signed_in: crate::account::token::read().is_some(),
        on: on(&store),
        has_key: files::key(&root).is_some(),
        synced_at: state.synced_at.map(|ms| ms as f64 / 1000.0),
        problem: PROBLEM.lock().ok().and_then(|held| held.clone()),
        machines: listed
            .into_iter()
            .map(|one| SyncMachine {
                this_one: one.id == state.machine,
                id: one.id,
                name: one.name,
                seen_at: one.seen.map(|seconds| seconds as f64),
            })
            .collect(),
        offering: crate::sync_pairing::offering(),
    })
}

/// `sync.read`
#[tauri::command]
#[specta::specta]
pub async fn sync_read() -> Result<SyncStatus, RpcError> {
    crate::off_main::blocking(status).await
}

/// `sync.set` — sync on or off on this machine. On, a machine without the
/// key makes one only for an account that syncs nothing yet; one that does
/// is joined from another machine, never written over.
#[tauri::command]
#[specta::specta]
pub async fn sync_set(on: bool) -> Result<SyncStatus, RpcError> {
    let root = root()?;
    if on && files::key(&root).is_none() {
        let blobs = DevpitApp::new(&crate::account::origin(), &token()?)
            .map_err(|err| RpcError::internal(err.to_string()))?;
        let listed = blobs.list().await.map_err(|err| refused(err.to_string()))?;
        if listed.is_empty() {
            take_key(SyncKey::generate().map_err(|err| RpcError::internal(err.to_string()))?)
                .await?;
        }
    }
    Store::open_default()?.set_preference_flag(preference::SYNC, on)?;
    set_problem(None);
    if on {
        round_waited().await;
    } else {
        crate::sync_pairing::withdraw();
    }
    crate::off_main::blocking(status).await
}

/// `sync.now` — a round now, rather than at the next one.
#[tauri::command]
#[specta::specta]
pub async fn sync_now() -> Result<SyncStatus, RpcError> {
    round_waited().await;
    crate::off_main::blocking(status).await
}

/// `sync.recovery` — the key as the person writes it down, shown when asked.
#[tauri::command]
#[specta::specta]
pub async fn sync_recovery() -> Result<SyncRecovery, RpcError> {
    let key = files::key(&root()?).ok_or_else(|| refused("this computer holds no sync key yet"))?;
    Ok(SyncRecovery {
        code: key.recovery_code(),
    })
}

/// `sync.start_over` — deletes what the account synced and starts again with
/// a new key, from what this machine has. Every other machine stops until it
/// is joined again.
#[tauri::command]
#[specta::specta]
pub async fn sync_start_over() -> Result<SyncStatus, RpcError> {
    let blobs = DevpitApp::new(&crate::account::origin(), &token()?)
        .map_err(|err| RpcError::internal(err.to_string()))?;
    blobs
        .clear()
        .await
        .map_err(|err| refused(err.to_string()))?;
    take_key(SyncKey::generate().map_err(|err| RpcError::internal(err.to_string()))?).await?;
    crate::sync_pairing::withdraw();
    Store::open_default()?.set_preference_flag(preference::SYNC, true)?;
    set_problem(None);
    round_waited().await;
    crate::off_main::blocking(status).await
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
