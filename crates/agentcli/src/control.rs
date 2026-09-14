//! Talking to a turn while it runs.
//!
//! In stream-json input mode the CLI reads control requests on stdin for as
//! long as stdin is open — measured on Claude Code 2.1.270: a `stop_task` was
//! answered within a tenth of a second by the task ending as `killed`. So a
//! turn keeps its stdin, and this is the handle to it.
//!
//! When to close it is the decision here. Closed at the first `result` the
//! process exits a second later; closed while a background task still runs,
//! that task may be cut off before the wake-up pass it would have caused. So
//! it closes once both are true: a result has arrived and nothing is running.

use std::collections::BTreeSet;
use std::io::Write;
use std::process::ChildStdin;
use std::sync::{Arc, Mutex};

use devpit_rpc::Part;

/// A running turn's stdin, shared with whoever may need to steer it.
#[derive(Clone, Default)]
pub struct Control {
    stdin: Arc<Mutex<Option<ChildStdin>>>,
}

impl Control {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn attach(&self, stdin: ChildStdin) {
        if let Ok(mut held) = self.stdin.lock() {
            *held = Some(stdin);
        }
    }

    /// Writes one line; false when the turn has already let go of stdin.
    pub(crate) fn write(&self, line: &str) -> bool {
        let Ok(mut held) = self.stdin.lock() else {
            return false;
        };
        let Some(stdin) = held.as_mut() else {
            return false;
        };
        writeln!(stdin, "{line}")
            .and_then(|_| stdin.flush())
            .is_ok()
    }

    /// Asks the CLI to stop one background task, by the id it gave it.
    pub fn stop_task(&self, task_id: &str) -> bool {
        self.write(&stop_task_request(task_id))
    }

    /// Lets go of stdin, which is what lets the CLI exit.
    pub(crate) fn close(&self) {
        if let Ok(mut held) = self.stdin.lock() {
            held.take();
        }
    }
}

/// The control request that stops one task, as the CLI's own schema names it:
/// `{subtype: "stop_task", task_id}`.
pub fn stop_task_request(task_id: &str) -> String {
    serde_json::json!({
        "type": "control_request",
        "request_id": format!("stop-{task_id}"),
        "request": { "subtype": "stop_task", "task_id": task_id },
    })
    .to_string()
}

/// The tasks still running, kept current as parts arrive.
#[derive(Debug, Default)]
pub struct Running {
    ids: BTreeSet<String>,
}

impl Running {
    pub fn saw(&mut self, part: &Part) {
        if let Part::Task {
            task_id, status, ..
        } = part
        {
            if status == "started" || status == "running" {
                self.ids.insert(task_id.clone());
            } else {
                self.ids.remove(task_id);
            }
        }
    }

    pub fn idle(&self) -> bool {
        self.ids.is_empty()
    }
}

/// Whether the turn may let go of stdin now.
pub fn may_close(result_seen: bool, running: &Running) -> bool {
    result_seen && running.idle()
}

#[cfg(test)]
#[path = "control_tests.rs"]
mod tests;
