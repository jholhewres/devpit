//! What each pane's agent last said through its hooks, so the live sessions
//! read a pane's screen only when its agent says it waits on a person.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use crate::card_activity::Doing;

#[derive(Default)]
pub(crate) struct PaneStates {
    heard: HashMap<String, Doing>,
    /// Panes read once without a word from their hooks: alive from before the
    /// app started, or an agent with none.
    looked: HashSet<String>,
}

impl PaneStates {
    pub(crate) fn heard(&mut self, leaf: &str, doing: Doing) {
        self.heard.insert(leaf.to_owned(), doing);
    }

    /// Whether the screen is worth reading: the agent said it waits, or it has
    /// said nothing yet and was never read.
    pub(crate) fn should_look(&mut self, leaf: &str) -> bool {
        match self.heard.get(leaf) {
            Some(Doing::Waiting) => true,
            Some(_) => false,
            None => self.looked.insert(leaf.to_owned()),
        }
    }
}

fn states() -> &'static Mutex<PaneStates> {
    static STATES: std::sync::OnceLock<Mutex<PaneStates>> = std::sync::OnceLock::new();
    STATES.get_or_init(Mutex::default)
}

pub(crate) fn heard(leaf: &str, doing: Doing) {
    if let Ok(mut states) = states().lock() {
        states.heard(leaf, doing);
    }
}

pub(crate) fn should_look(leaf: &str) -> bool {
    states()
        .lock()
        .is_ok_and(|mut states| states.should_look(leaf))
}

#[cfg(test)]
#[path = "pane_said_tests.rs"]
mod tests;
