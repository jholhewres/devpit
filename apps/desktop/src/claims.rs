//! Who owns a pane right now.
//!
//! A pane has one live client at a time, and the interesting case is the
//! handover: a React remount attaches again before the old attach has noticed
//! it is over, so the two overlap. The rule is that the **newer** one wins.
//!
//! Newer is decided *here*, by the order the attaches arrive, and not by an
//! id the caller mints. That was the first cut and it was wrong twice over: a
//! webview reload restarts whatever counter the client keeps, so half of all
//! reloads produced an id below the high-water mark and were refused; and the
//! screen reused one id for the whole window, so a remount compared equal to
//! itself and every second mount was told a newer client owned the pane. In
//! development that is every mount, because React mounts twice on purpose.
//!
//! Its own type because that rule is the only subtle thing in the session
//! layer, and because a rule with an `AppHandle` next to it is a rule that
//! cannot be tested without opening a window.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use devpit_rpc::{ErrorCode, RpcError};

pub(crate) use crate::live::Live;

/// Who holds a pane, and which attach that was.
struct Holder {
    client_id: String,
    generation: u64,
}

/// What claiming a pane got you.
pub(crate) struct Claimed {
    /// Hand this back to [`Claims::install`]. It is how a slow attach finds
    /// out that a faster one replaced it while its pty was starting.
    pub(crate) generation: u64,
    /// The client this one displaced, for the caller to stop.
    pub(crate) replaced: Option<Arc<Live>>,
}

#[derive(Default)]
pub struct Claims {
    /// The attached client of each pane.
    lives: Mutex<HashMap<String, Arc<Live>>>,
    /// Who has claimed each pane and has not released it.
    held: Mutex<HashMap<String, Holder>>,
    /// Ever-increasing, across every pane, for as long as the app runs.
    minted: AtomicU64,
}

impl Claims {
    pub fn new() -> Self {
        Self::default()
    }

    /// Claims the pane for a new client, and hands back the one it replaced.
    ///
    /// Never refuses. Arriving later *is* being newer, and the process that
    /// owns the panes is the only place that can say which arrived later —
    /// which is the whole reason the generation is minted here.
    ///
    /// The caller kills what comes back. Returning it rather than killing it
    /// here keeps this type about ownership and nothing else.
    pub(crate) fn take_for(&self, pane_id: &str, client_id: &str) -> Result<Claimed, RpcError> {
        let generation = self.minted.fetch_add(1, Ordering::SeqCst) + 1;
        self.held
            .lock()
            .map_err(|_| RpcError::internal("session claim lock"))?
            .insert(
                pane_id.to_owned(),
                Holder {
                    client_id: client_id.to_owned(),
                    generation,
                },
            );

        Ok(Claimed {
            generation,
            replaced: self
                .lives
                .lock()
                .map_err(|_| RpcError::internal("session lock"))?
                .remove(pane_id),
        })
    }

    /// Installs the pty ends, unless a newer client claimed the pane while
    /// this one was still starting up.
    pub(crate) fn install(
        &self,
        pane_id: &str,
        generation: u64,
        live: Arc<Live>,
    ) -> Result<bool, RpcError> {
        let held = self
            .held
            .lock()
            .map_err(|_| RpcError::internal("session claim lock"))?;
        if held
            .get(pane_id)
            .is_none_or(|holder| holder.generation != generation)
        {
            return Ok(false);
        }
        self.lives
            .lock()
            .map_err(|_| RpcError::internal("session lock"))?
            .insert(pane_id.to_owned(), live);
        Ok(true)
    }

    /// Gives up the claim, if it is still this client's to give up.
    ///
    /// A detach arriving late from a client that has already been replaced
    /// must not free the pane for the one that replaced it — which is why the
    /// client id has to be unique per attach and not per window.
    pub fn release(&self, pane_id: &str, client_id: &str) -> Result<bool, RpcError> {
        let mut held = self
            .held
            .lock()
            .map_err(|_| RpcError::internal("session claim lock"))?;
        if held
            .get(pane_id)
            .is_some_and(|holder| holder.client_id == client_id)
        {
            held.remove(pane_id);
            return Ok(true);
        }
        Ok(false)
    }

    /// The client attached to a pane, or the error the screen shows instead.
    pub(crate) fn live(&self, pane_id: &str) -> Result<Arc<Live>, RpcError> {
        self.lives
            .lock()
            .map_err(|_| RpcError::internal("session lock"))?
            .get(pane_id)
            .cloned()
            .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "that pane is not attached"))
    }

    /// Removes a pane's client when it is the one named, and hands it over.
    pub(crate) fn take_if_theirs(
        &self,
        pane_id: &str,
        client_id: &str,
    ) -> Result<Option<Arc<Live>>, RpcError> {
        let mut lives = self
            .lives
            .lock()
            .map_err(|_| RpcError::internal("session lock"))?;
        Ok(match lives.get(pane_id) {
            Some(live) if live.client_id == client_id => lives.remove(pane_id),
            _ => None,
        })
    }

    /// Forgets a pane's client, if it is still this exact one.
    ///
    /// By identity and not by id: a drain ending has to clear its own entry
    /// and must not clear the entry of the client that replaced it.
    pub(crate) fn forget_exactly(&self, pane_id: &str, live: &Arc<Live>) {
        if let Ok(mut lives) = self.lives.lock() {
            if lives
                .get(pane_id)
                .is_some_and(|current| Arc::ptr_eq(current, live))
            {
                lives.remove(pane_id);
            }
        }
    }
}

#[cfg(test)]
#[path = "claims_tests.rs"]
mod tests;
