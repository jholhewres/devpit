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

/// A listing, when it was made, and the hook sequence read just before tmux
/// was asked: a hook heard after it is newer than anything the listing says.
pub(crate) struct Listing {
    at: Instant,
    pub seq: u64,
    pub rows: Arc<Vec<Snapped>>,
}

/// The listing every caller shares, one made at a time.
pub(crate) struct Shared {
    kept: Mutex<Option<Arc<Listing>>>,
    listing: Mutex<()>,
}

impl Shared {
    pub(crate) const fn new() -> Shared {
        Shared {
            kept: Mutex::new(None),
            listing: Mutex::new(()),
        }
    }

    /// The kept listing while it is at most [`FRESH`] old, else `list`'s.
    pub(crate) fn fresh(&self, list: impl FnOnce() -> Vec<Snapped>) -> Arc<Listing> {
        let kept = || {
            self.kept
                .lock()
                .ok()?
                .clone()
                .filter(|kept| kept.at.elapsed() < FRESH)
        };
        if let Some(kept) = kept() {
            return kept;
        }
        let _listing = self.listing.lock();
        if let Some(kept) = kept() {
            return kept;
        }
        self.keep(list)
    }

    /// A new listing, kept for everyone.
    pub(crate) fn refresh(&self, list: impl FnOnce() -> Vec<Snapped>) -> Arc<Listing> {
        let _listing = self.listing.lock();
        self.keep(list)
    }

    fn keep(&self, list: impl FnOnce() -> Vec<Snapped>) -> Arc<Listing> {
        let seq = crate::card_activity::next_seq();
        let listing = Arc::new(Listing {
            seq,
            rows: Arc::new(list()),
            at: Instant::now(),
        });
        if let Ok(mut held) = self.kept.lock() {
            *held = Some(listing.clone());
        }
        listing
    }
}

static SHARED: Shared = Shared::new();

/// The panes of every session, from a listing at most [`FRESH`] old.
pub(crate) fn rows() -> Arc<Vec<Snapped>> {
    listing().rows.clone()
}

/// [`rows`], with the sequence it is newer than.
pub(crate) fn listing() -> Arc<Listing> {
    SHARED.fresh(listed)
}

fn listed() -> Vec<Snapped> {
    let socket = crate::sessions::socket_path().ok();
    listed_at(socket.as_deref(), || {
        crate::sessions::tmux_server()
            .ok()
            .and_then(|server| server.snapshot().ok())
            .unwrap_or_default()
    })
}

/// The listing `list` makes, only where tmux's socket is. No socket, no
/// server: nothing to ask, and not even the tmux helper, whose wrapped shell
/// a ptyd-only machine would build every two seconds for nothing.
pub(crate) fn listed_at(
    socket: Option<&std::path::Path>,
    list: impl FnOnce() -> Vec<Snapped>,
) -> Vec<Snapped> {
    match socket {
        Some(socket) if socket.exists() => list(),
        _ => Vec::new(),
    }
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

/// Every project's panes, named by one `name` for all of them: naming reads
/// the process table, once a tick rather than once a project.
pub(crate) fn named_by_project(
    rows: &[Snapped],
    name: impl FnOnce(Vec<Running>) -> Vec<devpit_rpc::PaneRunning>,
) -> HashMap<String, Vec<devpit_rpc::PaneRunning>> {
    let mut now: HashMap<String, Vec<devpit_rpc::PaneRunning>> = HashMap::new();
    let mut owners = Vec::new();
    let mut panes = Vec::new();
    for project in projects(rows) {
        for pane in in_project(rows, &project) {
            owners.push(project.clone());
            panes.push(pane);
        }
        now.insert(project, Vec::new());
    }
    for (project, pane) in owners.into_iter().zip(name(panes)) {
        now.entry(project).or_default().push(pane);
    }
    now
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
            let Listing { seq, rows, .. } = &*SHARED.refresh(listed);
            let (seq, rows) = (*seq, rows.clone());
            let mut now = named_by_project(&rows, crate::shell_launch::named_panes);
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
