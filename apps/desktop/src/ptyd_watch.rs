//! What each ptyd pane runs and where, told by the daemon as it changes
//! (`Watch`) to [`crate::ptyd_tap`], so its panes cost no polling at all.

use std::collections::HashMap;
use std::sync::Mutex;

use devpit_rpc::PaneRunning;

/// What each pane, by the daemon's number, has in front.
#[derive(Default)]
pub(crate) struct Fronts {
    programs: HashMap<u32, String>,
    /// Where each pane's shell last said it stands.
    cwds: HashMap<u32, String>,
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
            // Kept, not news: the window hears a folder from the shell itself.
            Kind::Cwd => {
                let cwd = String::from_utf8_lossy(&frame.bytes).into_owned();
                self.cwds.insert(frame.pane, cwd);
                None
            }
            Kind::Exited => {
                self.cwds.remove(&frame.pane);
                self.programs.remove(&frame.pane).map(|_| frame.pane)
            }
            _ => None,
        }
    }

    pub(crate) fn program_of(&self, pane: u32) -> Option<&str> {
        self.programs.get(&pane).map(String::as_str)
    }

    pub(crate) fn cwd_of(&self, pane: u32) -> Option<&str> {
        self.cwds.get(&pane).map(String::as_str)
    }

    /// Adds to what is known, never takes away: a pane opened a moment ago
    /// is placed before its layout is stored.
    pub(crate) fn place(&mut self, places: HashMap<u32, (String, String)>) {
        self.places.extend(places);
    }

    pub(crate) fn placed(&self, pane: u32) -> bool {
        self.places.contains_key(&pane)
    }

    /// Which project and leaf pane `pane` is.
    pub(crate) fn place_of(&self, pane: u32) -> Option<&(String, String)> {
        self.places.get(&pane)
    }

    /// Whether `leaf` is a placed pane of `project`'s.
    pub(crate) fn holds(&self, project: &str, leaf: &str) -> bool {
        self.place_of(crate::ptyd_link::pane_number(leaf))
            .is_some_and(|(placed_in, placed)| placed_in == project && placed == leaf)
    }

    /// Every pane placed, by number.
    pub(crate) fn panes(&self) -> Vec<u32> {
        self.places.keys().copied().collect()
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

    pub(crate) fn forget_all(&mut self) -> Vec<String> {
        let projects = self.projects();
        self.programs.clear();
        self.cwds.clear();
        projects
    }
}

fn fronts() -> &'static Mutex<Fronts> {
    &crate::ptyd_tap::tap().fronts
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

/// Whether `leaf` is a pane the daemon holds for `project`, as the layouts
/// read so far and the panes opened since say.
pub(crate) fn holds(project_id: &str, leaf_id: &str) -> bool {
    fronts()
        .lock()
        .is_ok_and(|fronts| fronts.holds(project_id, leaf_id))
}

/// What pane `pane` runs in front, as the daemon last said.
pub(crate) fn program(pane: u32) -> Option<String> {
    let fronts = fronts().lock().ok()?;
    fronts.program_of(pane).map(ToOwned::to_owned)
}

/// Where pane `pane`'s shell stands, as the daemon last said.
pub(crate) fn cwd(pane: u32) -> Option<String> {
    let fronts = fronts().lock().ok()?;
    fronts.cwd_of(pane).map(ToOwned::to_owned)
}

/// Every ptyd leaf of every project's layouts, by the daemon's number.
pub(crate) fn places(store: &devpit_core::Store) -> HashMap<u32, (String, String)> {
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

#[cfg(test)]
#[path = "ptyd_watch_tests.rs"]
mod tests;
