//! Every question about devpit's tmux panes, answered from one
//! `tmux list-panes -a` kept for a moment.
//!
//! The sidebar, the status strip, the live sessions and the island each asked
//! tmux on their own clock; an idle window forked about two processes a
//! second. Now one spawn serves them all, and none happens while there is no
//! tmux server.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use devpit_tmux::{Running, Snapped};

/// How long one listing answers every caller: past [`EVERY`], so the
/// watcher's own listing serves everyone else while the window is shown.
const FRESH: Duration = Duration::from_millis(2500);

/// How often the window is told what changed while it is on screen.
const EVERY: Duration = Duration::from_secs(2);

type Kept = Option<(Instant, Arc<Vec<Snapped>>)>;

static KEPT: Mutex<Kept> = Mutex::new(None);
/// One listing at a time: callers that missed together take the same one.
static LISTING: Mutex<()> = Mutex::new(());

/// The panes of every session, from a listing at most [`FRESH`] old.
pub(crate) fn rows() -> Arc<Vec<Snapped>> {
    let kept = || {
        KEPT.lock()
            .ok()?
            .as_ref()
            .filter(|(at, _)| at.elapsed() < FRESH)
            .map(|(_, rows)| rows.clone())
    };
    if let Some(rows) = kept() {
        return rows;
    }
    let _listing = LISTING.lock();
    if let Some(rows) = kept() {
        return rows;
    }
    keep()
}

/// A new listing, kept for everyone.
fn refresh() -> Arc<Vec<Snapped>> {
    let _listing = LISTING.lock();
    keep()
}

fn keep() -> Arc<Vec<Snapped>> {
    let rows = Arc::new(listed());
    if let Ok(mut held) = KEPT.lock() {
        *held = Some((Instant::now(), rows.clone()));
    }
    rows
}

fn listed() -> Vec<Snapped> {
    let Ok(server) = crate::sessions::tmux_server() else {
        return Vec::new();
    };
    // No socket, no server: nothing to ask, and no spawn to ask it.
    if !server.socket_exists() {
        return Vec::new();
    }
    server.snapshot().unwrap_or_default()
}

/// A project's panes, once each: its own session and its clients all list
/// every window of the group.
pub(crate) fn in_project(rows: &[Snapped], project_id: &str) -> Vec<Running> {
    let session = devpit_tmux::Server::session_name(project_id);
    let client = format!("{session}__");
    let mut seen = HashSet::new();
    rows.iter()
        .filter(|row| row.session == session || row.session.starts_with(&client))
        .filter(|row| seen.insert(row.running.leaf_id.clone()))
        .map(|row| row.running.clone())
        .collect()
}

/// Every pane on the server, once each.
pub(crate) fn everywhere(rows: &[Snapped]) -> Vec<Running> {
    let mut seen = HashSet::new();
    rows.iter()
        .filter(|row| seen.insert(row.running.leaf_id.clone()))
        .map(|row| row.running.clone())
        .collect()
}

/// Which window each pane id is in now.
pub(crate) fn windows(rows: &[Snapped]) -> HashMap<String, String> {
    rows.iter()
        .map(|row| (row.pane_id.clone(), row.running.leaf_id.clone()))
        .collect()
}

/// The projects with a tmux pane, by id.
pub(crate) fn projects(rows: &[Snapped]) -> HashSet<String> {
    rows.iter()
        .filter_map(|row| {
            let rest = row.session.strip_prefix("devpit_")?;
            Some(
                rest.split_once("__")
                    .map_or(rest, |(project, _)| project)
                    .to_owned(),
            )
        })
        .collect()
}

/// Whether a project's panes changed since they were last told.
#[derive(Default)]
pub(crate) struct Told {
    said: HashMap<String, Vec<devpit_rpc::PaneRunning>>,
}

impl Told {
    /// The projects whose panes differ from what was last told, remembering
    /// the new answer. A project gone quiet is told once, empty.
    pub(crate) fn changed(
        &mut self,
        now: HashMap<String, Vec<devpit_rpc::PaneRunning>>,
    ) -> Vec<String> {
        let mut changed: Vec<String> = self
            .said
            .keys()
            .filter(|project| !now.contains_key(*project))
            .cloned()
            .collect();
        for project in &changed {
            self.said.remove(project);
        }
        for (project, panes) in now {
            if self.said.get(&project) != Some(&panes) {
                changed.push(project.clone());
                self.said.insert(project, panes);
            }
        }
        changed.sort();
        changed
    }
}

/// Tells the window `panes:changed` with a project's id when what its panes
/// run changes, instead of the window asking every two seconds. Only while
/// the window is on screen, and only while a tmux server is up.
pub(crate) fn watch(app: tauri::AppHandle) {
    use tauri::{Emitter, Manager};
    std::thread::spawn(move || {
        let mut told = Told::default();
        loop {
            std::thread::sleep(EVERY);
            let shown = app.get_webview_window("main").is_some_and(|main| {
                main.is_visible().unwrap_or(true) && !main.is_minimized().unwrap_or(false)
            });
            if !shown {
                continue;
            }
            let seq = crate::card_activity::next_seq();
            let rows = refresh();
            let mut now: HashMap<String, Vec<devpit_rpc::PaneRunning>> = HashMap::new();
            for project in projects(&rows) {
                let panes = crate::shell_launch::named_panes(in_project(&rows, &project));
                now.insert(project, panes);
            }
            #[cfg(unix)]
            for project in crate::ptyd_watch::projects() {
                let panes = crate::ptyd_watch::running_in(&project);
                now.entry(project).or_default().extend(panes);
            }
            for project in told.changed(now.clone()) {
                let panes = now.remove(&project).unwrap_or_default();
                crate::card_reconcile::reconcile(&app, &project, &panes, seq);
                let _ = app.emit("panes:changed", project);
            }
        }
    });
}

#[cfg(test)]
#[path = "pane_snapshot_tests.rs"]
mod tests;
