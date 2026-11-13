//! Settings → Worktrees → Cleanup: the switch, the last run, and running it
//! now. It also runs on its own, a few minutes after start and every hour.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use devpit_core::{preference, Store};
use devpit_rpc::RpcError;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::worktree_sweep::{self, CleanupRun, Places, Rules};

/// The cleanup as Settings shows it.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeCleanup {
    pub delete_merged: bool,
    pub last: Option<CleanupRun>,
}

const FIRST_AFTER: Duration = Duration::from_secs(5 * 60);
const EVERY: Duration = Duration::from_secs(60 * 60);

/// One cleanup at a time: the hour and the button never race.
static RUNNING: Mutex<()> = Mutex::new(());

fn store() -> Result<Store, RpcError> {
    Ok(Store::open_default()?)
}

fn read(store: &Store) -> Result<WorktreeCleanup, RpcError> {
    Ok(WorktreeCleanup {
        delete_merged: worktree_sweep::rules_of(store).delete_landed,
        last: store
            .preference(preference::WORKTREES_CLEANUP_LAST)?
            .and_then(|raw| serde_json::from_str(&raw).ok()),
    })
}

/// `worktree.cleanup_read`
#[tauri::command]
#[specta::specta]
pub async fn worktree_cleanup_read() -> Result<WorktreeCleanup, RpcError> {
    crate::off_main::blocking(|| read(&store()?)).await
}

/// `worktree.cleanup_write` — turns deleting merged worktrees on or off.
#[tauri::command]
#[specta::specta]
pub async fn worktree_cleanup_write(delete_merged: bool) -> Result<WorktreeCleanup, RpcError> {
    crate::off_main::blocking(move || {
        let store = store()?;
        store.set_preference_flag(preference::WORKTREES_DELETE_MERGED, delete_merged)?;
        read(&store)
    })
    .await
}

/// `worktree.cleanup_now` — the Delete now button: merged worktrees go now,
/// whatever the switch says, since the switch is about going on their own.
#[tauri::command]
#[specta::specta]
pub async fn worktree_cleanup_now() -> Result<WorktreeCleanup, RpcError> {
    crate::off_main::blocking(|| {
        let store = store()?;
        run_now(
            &store,
            &Rules {
                delete_landed: true,
            },
        )?;
        read(&store)
    })
    .await
}

fn run_now(store: &Store, rules: &Rules) -> Result<(), RpcError> {
    let _one = RUNNING.lock().unwrap_or_else(|held| held.into_inner());
    let places = Places {
        cache: worktree_sweep::build_caches(),
    };
    let cwds = running_in();
    let live = |card_id: &str, checkout: &Path| held(&cwds, checkout) || heard(card_id);
    let run = worktree_sweep::sweep(
        store,
        &places,
        rules,
        devpit_core::reports::now() as i64,
        &live,
    );
    if !run.removed.is_empty() || !run.kept.is_empty() {
        eprintln!("devpit {}", worktree_sweep::said(&run));
    }
    let json = serde_json::to_string(&run).map_err(|err| RpcError::internal(err.to_string()))?;
    store.set_preference(preference::WORKTREES_CLEANUP_LAST, &json)?;
    Ok(())
}

/// Runs the cleanup a few minutes after start, then every hour.
pub(crate) fn watch() {
    let _ = std::thread::Builder::new()
        .name("worktree-cleanup".to_owned())
        .spawn(|| {
            std::thread::sleep(FIRST_AFTER);
            loop {
                let ran =
                    store().and_then(|store| run_now(&store, &worktree_sweep::rules_of(&store)));
                if let Err(err) = ran {
                    devpit_core::reports::background("worktree cleanup", &err.message);
                }
                std::thread::sleep(EVERY);
            }
        });
}

/// Whether a session devpit heard from on this card is still alive.
fn heard(card_id: &str) -> bool {
    crate::card_activity::registry()
        .lock()
        .map(|activities| {
            activities
                .surviving()
                .iter()
                .any(|key| key.card_id == card_id)
        })
        .unwrap_or(false)
}

/// Whether any of these folders is the checkout or inside it.
pub(crate) fn held(cwds: &[PathBuf], checkout: &Path) -> bool {
    let checkout = std::fs::canonicalize(checkout).unwrap_or_else(|_| checkout.to_path_buf());
    cwds.iter().any(|cwd| cwd.starts_with(&checkout))
}

/// The folder every process of this user runs in now.
#[cfg(target_os = "linux")]
pub(crate) fn running_in() -> Vec<PathBuf> {
    std::fs::read_dir("/proc")
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .bytes()
                .all(|b| b.is_ascii_digit())
        })
        .filter_map(|entry| std::fs::read_link(entry.path().join("cwd")).ok())
        .collect()
}

#[cfg(target_os = "macos")]
pub(crate) fn running_in() -> Vec<PathBuf> {
    devpit_pty::host_env::command("lsof")
        .args(["-a", "-d", "cwd", "-F", "n", "-u"])
        .arg(std::env::var("USER").unwrap_or_default())
        .output()
        .map(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|line| line.strip_prefix('n'))
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default()
}

/// Without a cheap way to ask, only the sessions devpit heard from count.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub(crate) fn running_in() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(test)]
#[path = "worktree_cleanup_tests.rs"]
mod tests;
