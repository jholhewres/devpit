//! The runs happening right now, and stopping one.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use devpit_core::Store;
use devpit_rpc::{ErrorCode, RpcError};
use std::sync::Arc;
use tauri::{AppHandle, State};

use crate::card_activity::{run_heard, run_reference, state_of_run};

/// The runs in flight, by id, and the process behind each.
///
/// A step you cannot stop is a step you learn not to start: a turn that hangs
/// would otherwise leave its run at `running` for the life of the app, and the
/// card behind it asking for confirmation on every move.
#[derive(Default)]
pub struct InFlight {
    processes: Mutex<HashMap<String, u32>>,
    /// Runs a person has asked to stop. Marked before the signal is sent, so
    /// the run's own thread, waking to a killed process, records the stop and
    /// not a failure.
    cancelled: Mutex<HashSet<String>>,
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
        self.uncancel(run_id);
    }

    /// Marks a run as stopped by a person.
    pub fn cancel(&self, run_id: &str) {
        if let Ok(mut cancelled) = self.cancelled.lock() {
            cancelled.insert(run_id.to_owned());
        }
    }

    /// Takes the mark back, for a stop that never reached the process.
    pub fn uncancel(&self, run_id: &str) {
        if let Ok(mut cancelled) = self.cancelled.lock() {
            cancelled.remove(run_id);
        }
    }

    pub fn was_cancelled(&self, run_id: &str) -> bool {
        self.cancelled
            .lock()
            .map(|cancelled| cancelled.contains(run_id))
            .unwrap_or(false)
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
    app: AppHandle,
    state: State<'_, Arc<InFlight>>,
    card_id: String,
    run_id: String,
) -> Result<(), RpcError> {
    let store = Store::open_default()?;
    // The stop is told on the card, so the run has to be that card's.
    if !store.runs(&card_id)?.iter().any(|run| run.id == run_id) {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "that run is not on this card",
        ));
    }
    stop_run(&app, &state, &store, &card_id, &run_id)
}

/// Stops one run by id, records it cancelled, and tells its card.
///
/// Apart from the command so an update that was told to stop the work can do
/// exactly what the card's stop button does, and nothing less.
pub(crate) fn stop_run(
    app: &AppHandle,
    state: &InFlight,
    store: &Store,
    card_id: &str,
    run_id: &str,
) -> Result<(), RpcError> {
    let Some(pid) = state.pid_of(run_id) else {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "that run is not in flight here — it may have finished already",
        ));
    };

    // Before the signal: the run's thread wakes the moment its process dies,
    // and has to find the stop already said.
    state.cancel(run_id);

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
        state.uncancel(run_id);
        return Err(RpcError::internal(format!("could not stop process {pid}")));
    }

    let stopped_here = store.finish_run(
        run_id,
        "cancelled",
        Some("stopped by you"),
        None,
        None,
        None,
    )?;
    state.forget(run_id);
    // Unless the run reached its own end first, and told the card itself.
    if stopped_here {
        run_heard(
            app,
            card_id,
            &run_reference(store, run_id),
            state_of_run("cancelled"),
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "in_flight_tests.rs"]
mod tests;
