//! `session.cost` — what a session has spent so far, for wherever it is drawn.
//!
//! One answer for a terminal's session and a chat's: both are a transcript
//! the CLI writes, read on from where the last ask stopped.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use devpit_agentcli::session_cost::Tally;
use devpit_rpc::{RpcError, SessionCost, TokenCounts};

static TALLIES: OnceLock<Mutex<HashMap<String, (PathBuf, Tally)>>> = OnceLock::new();

/// The transcript a session writes, in any installation: `projects/<cwd>/<id>.jsonl`.
fn transcript(configs: &[PathBuf], session_id: &str) -> Option<PathBuf> {
    let file = format!("{session_id}.jsonl");
    configs.iter().find_map(|config| {
        std::fs::read_dir(config.join("projects"))
            .ok()?
            .filter_map(Result::ok)
            .map(|folder| folder.path().join(&file))
            .find(|path| path.is_file())
    })
}

pub(crate) fn cost_of(configs: &[PathBuf], session_id: &str) -> Option<SessionCost> {
    // Only what a CLI could have named a file: this becomes a path.
    if session_id.is_empty()
        || !session_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return None;
    }
    let mut all = TALLIES.get_or_init(Default::default).lock().ok()?;
    if !all.get(session_id).is_some_and(|(path, _)| path.is_file()) {
        let path = transcript(configs, session_id)?;
        all.insert(session_id.to_owned(), (path, Tally::default()));
    }
    let (path, tally) = all.get_mut(session_id)?;
    tally.advance(Path::new(path)).ok()?;
    Some(said(tally))
}

fn said(tally: &Tally) -> SessionCost {
    SessionCost {
        cost_usd: tally.cost_usd,
        last_turn_usd: tally.last_turn_usd,
        since_seen_usd: tally.cost_usd - tally.before_usd.unwrap_or(0.0),
        tokens: TokenCounts {
            input: tally.input as f64,
            output: tally.output as f64,
            cache_read: tally.cache_read as f64,
            cache_write: tally.cache_write as f64,
        },
        model: tally.model.clone(),
        unpriced_tokens: tally.unpriced as f64,
    }
}

#[tauri::command]
#[specta::specta]
pub async fn session_cost(session_id: String) -> Result<Option<SessionCost>, RpcError> {
    crate::off_main::blocking(move || {
        let configs: Vec<PathBuf> = crate::installations::found()?
            .into_iter()
            .map(|one| one.directory)
            .collect();
        Ok(cost_of(&configs, &session_id))
    })
    .await
}

#[cfg(test)]
#[path = "session_cost_tests.rs"]
mod tests;
