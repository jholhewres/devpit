//! Replies an orchestrator drafts for the person to send as theirs.
//!
//! A message from the orchestrator approves nothing in another session — the
//! CLI treats it as coming from the orchestrator, and rightly. What the person
//! says in the orchestrator's chat used to stop there: to let a session go on,
//! they had to find its tab and type it again. So the orchestrator writes the
//! words and the window shows them beside the session; the person sends them,
//! edits them or drops them, and only their click types anything.
//!
//! Held in memory, one per session: a draft is for now, and one that outlived
//! the app would offer words about a moment that has passed.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// The most a draft may be, in characters: what a reply may be.
pub(crate) const LONGEST: usize = 4000;

type Drafts = HashMap<(String, String), String>;

fn drafts() -> &'static Mutex<Drafts> {
    static DRAFTS: OnceLock<Mutex<Drafts>> = OnceLock::new();
    DRAFTS.get_or_init(Mutex::default)
}

/// Keeps `text` as the reply drafted for the session `name` of `profile`,
/// in place of any before it.
pub(crate) fn draft(profile: &str, name: &str, text: &str) -> Result<(), String> {
    let text = text.trim();
    if text.is_empty() || text.chars().count() > LONGEST {
        return Err(format!("a draft is between 1 and {LONGEST} characters"));
    }
    drafts()
        .lock()
        .map_err(|_| "the drafts could not be read".to_owned())?
        .insert((profile.to_owned(), name.to_owned()), text.to_owned());
    Ok(())
}

/// The reply drafted for a session, if one is waiting to be sent.
pub(crate) fn drafted(profile: &str, name: &str) -> Option<String> {
    drafts()
        .lock()
        .ok()?
        .get(&(profile.to_owned(), name.to_owned()))
        .cloned()
}

/// Every draft waiting, as (profile, session, text), in a steady order.
pub(crate) fn all() -> Vec<(String, String, String)> {
    let Ok(held) = drafts().lock() else {
        return Vec::new();
    };
    let mut every: Vec<_> = held
        .iter()
        .map(|((profile, name), text)| (profile.clone(), name.clone(), text.clone()))
        .collect();
    every.sort();
    every
}

/// Lets go of a session's draft: sent, or dropped by the person.
pub(crate) fn forget(profile: &str, name: &str) {
    if let Ok(mut held) = drafts().lock() {
        held.remove(&(profile.to_owned(), name.to_owned()));
    }
}

/// `orchestrator.draft_drop` — the person drops a draft without sending it.
#[tauri::command]
#[specta::specta]
pub fn orchestrator_draft_drop(profile_id: String, name: String) {
    forget(&profile_id, &name);
}

#[cfg(test)]
#[path = "reply_drafts_tests.rs"]
mod tests;
