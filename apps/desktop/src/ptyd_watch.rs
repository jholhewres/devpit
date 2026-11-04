//! What each ptyd pane runs, told by the daemon as it changes (`Watch`), so
//! its panes cost no polling at all.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use devpit_rpc::PaneRunning;

/// How long to wait before asking a daemon that was not there again.
const AGAIN: Duration = Duration::from_secs(5);

/// What each pane, by the daemon's number, has in front.
#[derive(Default)]
pub(crate) struct Fronts {
    programs: HashMap<u32, String>,
    /// Which project and leaf a pane number is, read off the layouts.
    places: HashMap<u32, (String, String)>,
}

impl Fronts {
    /// A frame from the daemon. Answers the pane's number when what it runs
    /// changed.
    pub(crate) fn heard(&mut self, frame: &devpit_ptyd::frame::Frame) -> Option<u32> {
        use devpit_ptyd::frame::Kind;
        match frame.kind()? {
            Kind::Program => {
                let program = String::from_utf8_lossy(&frame.bytes).into_owned();
                let was = self.programs.insert(frame.pane, program.clone());
                (was.as_deref() != Some(program.as_str())).then_some(frame.pane)
            }
            Kind::Exited => self.programs.remove(&frame.pane).map(|_| frame.pane),
            _ => None,
        }
    }

    pub(crate) fn place(&mut self, places: HashMap<u32, (String, String)>) {
        self.places = places;
    }

    pub(crate) fn placed(&self, pane: u32) -> bool {
        self.places.contains_key(&pane)
    }

    pub(crate) fn project_of(&self, pane: u32) -> Option<&str> {
        self.places.get(&pane).map(|(project, _)| project.as_str())
    }

    pub(crate) fn projects(&self) -> Vec<String> {
        let mut found: Vec<String> = self
            .programs
            .keys()
            .filter_map(|pane| self.project_of(*pane).map(ToOwned::to_owned))
            .collect();
        found.sort();
        found.dedup();
        found
    }

    pub(crate) fn running_in(&self, project_id: &str) -> Vec<PaneRunning> {
        let mut found: Vec<PaneRunning> = self
            .programs
            .iter()
            .filter_map(|(pane, program)| {
                let (project, leaf) = self.places.get(pane)?;
                (project == project_id).then(|| crate::shell_launch::named_program(leaf, program))
            })
            .collect();
        found.sort_by(|a, b| a.pane_id.cmp(&b.pane_id));
        found
    }

    fn forget_all(&mut self) -> Vec<String> {
        let projects = self.projects();
        self.programs.clear();
        projects
    }
}

fn fronts() -> &'static Mutex<Fronts> {
    static FRONTS: std::sync::OnceLock<Mutex<Fronts>> = std::sync::OnceLock::new();
    FRONTS.get_or_init(Mutex::default)
}

pub(crate) fn projects() -> Vec<String> {
    fronts()
        .lock()
        .map(|fronts| fronts.projects())
        .unwrap_or_default()
}

pub(crate) fn running_in(project_id: &str) -> Vec<PaneRunning> {
    fronts()
        .lock()
        .map(|fronts| fronts.running_in(project_id))
        .unwrap_or_default()
}

/// Every ptyd leaf of every project's layouts, by the daemon's number.
fn places(store: &devpit_core::Store) -> HashMap<u32, (String, String)> {
    let mut found = HashMap::new();
    for project in store.projects().unwrap_or_default() {
        for tab in store.pane_layout_tabs(&project.id).unwrap_or_default() {
            let Ok(Some((tree, focused))) = store.pane_layout(&project.id, &tab) else {
                continue;
            };
            let Ok(layout) = crate::sessions::decode(&project.id, &tree, &focused) else {
                continue;
            };
            for leaf in crate::ptyd_link::ptyd_leaves(&layout) {
                let number = crate::ptyd_link::pane_number(&leaf);
                found.insert(number, (project.id.clone(), leaf));
            }
        }
    }
    found
}

/// Listens to the daemon while ptyd terminals are on, never starting one,
/// and tells the window `panes:changed` as a pane's program changes.
pub(crate) fn watch(app: tauri::AppHandle) {
    use tauri::Emitter;
    std::thread::spawn(move || loop {
        std::thread::sleep(AGAIN);
        let Ok(store) = crate::projects::store() else {
            continue;
        };
        if !crate::ptyd_link::wanted(&store) {
            continue;
        }
        let Ok(home) = devpit_core::Store::root() else {
            continue;
        };
        let socket = crate::ptyd_link::socket_path(&home);
        // Connected only, never started: a daemon nobody asked for stays off.
        let Ok(mut stream) = devpit_ptyd::socket::connect(&socket) else {
            continue;
        };
        let watch = devpit_ptyd::frame::Frame::new(0, devpit_ptyd::frame::Kind::Watch, Vec::new());
        if devpit_ptyd::frame::write(&mut stream, &watch).is_err() {
            continue;
        }
        if let Ok(mut fronts) = fronts().lock() {
            fronts.place(places(&store));
        }
        while let Ok(Some(frame)) = devpit_ptyd::frame::read(&mut stream) {
            let Ok(mut fronts) = fronts().lock() else {
                break;
            };
            let Some(pane) = fronts.heard(&frame) else {
                continue;
            };
            if !fronts.placed(pane) {
                if let Ok(store) = crate::projects::store() {
                    fronts.place(places(&store));
                }
            }
            if let Some(project) = fronts.project_of(pane).map(ToOwned::to_owned) {
                drop(fronts);
                let _ = app.emit("panes:changed", project);
            }
        }
        let gone = fronts()
            .lock()
            .map(|mut fronts| fronts.forget_all())
            .unwrap_or_default();
        for project in gone {
            let _ = app.emit("panes:changed", project);
        }
    });
}

#[cfg(test)]
#[path = "ptyd_watch_tests.rs"]
mod tests;
