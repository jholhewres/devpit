//! The runs happening right now, and stopping one.

use std::collections::HashMap;
use std::sync::Mutex;

use devpit_core::Store;
use devpit_rpc::{ErrorCode, RpcError};
use std::sync::Arc;
use tauri::State;

/// The runs in flight, by id, and the process behind each.
///
/// A step you cannot stop is a step you learn not to start: a turn that hangs
/// would otherwise leave its run at `running` for the life of the app, and the
/// card behind it asking for confirmation on every move.
#[derive(Default)]
pub struct InFlight {
    processes: Mutex<HashMap<String, u32>>,
}

impl InFlight {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn watch(&self, run_id: &str, pid: u32) {
        if let Ok(mut processes) = self.processes.lock() {
            processes.insert(run_id.to_owned(), pid);
        }
    }

    pub fn forget(&self, run_id: &str) {
        if let Ok(mut processes) = self.processes.lock() {
            processes.remove(run_id);
        }
    }

    fn pid_of(&self, run_id: &str) -> Option<u32> {
        self.processes.lock().ok()?.get(run_id).copied()
    }
}

/// `run.cancel` — stops a run, and says so on the card.
///
/// The row goes to `cancelled` rather than `failed`: a person stopping work is
/// not the work going wrong, and a board that cannot tell them apart teaches
/// you to distrust every red row on it.
#[tauri::command]
#[specta::specta]
pub fn run_cancel(
    state: State<'_, Arc<InFlight>>,
    card_id: String,
    run_id: String,
) -> Result<(), RpcError> {
    let Some(pid) = state.pid_of(&run_id) else {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "that run is not in flight here — it may have finished already",
        ));
    };

    // SIGTERM, not SIGKILL: the CLI writes its transcript on the way out, and
    // what a cancelled turn already spent is worth keeping.
    //
    // Through the system's own `kill` rather than a crate: it is one command
    // that is already on the machine, and a dependency for one signal is a
    // dependency to keep in step forever.
    #[cfg(unix)]
    let stopped = std::process::Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);
    #[cfg(not(unix))]
    let stopped = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);

    if !stopped {
        return Err(RpcError::internal(format!("could not stop process {pid}")));
    }

    let store = Store::open_default()?;
    store.finish_run(
        &run_id,
        "cancelled",
        Some("stopped by you"),
        None,
        None,
        None,
    )?;
    state.forget(&run_id);
    let _ = card_id;
    Ok(())
}
