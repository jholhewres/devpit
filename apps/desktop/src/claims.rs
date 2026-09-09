//! Who owns a pane right now.
//!
//! A pane has one live client at a time, and the interesting case is the
//! handover: a React remount attaches again before the old attach has noticed
//! it is over, so the two overlap. The rule is that the **newer** one wins,
//! decided by a client id that only ever goes up.
//!
//! Its own type because that rule is the only subtle thing in the session
//! layer, and because a rule with an `AppHandle` next to it is a rule that
//! cannot be tested without opening a window.

use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};

use devpit_rpc::{ErrorCode, RpcError};
use portable_pty::{ChildKiller, MasterPty};

/// One attached client: the ends of its pty, and what it has printed.
pub(crate) struct Live {
    pub(crate) client_id: String,
    pub(crate) writer: Mutex<Box<dyn Write + Send>>,
    pub(crate) master: Mutex<Box<dyn MasterPty + Send>>,
    pub(crate) killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    /// The scrollback this pane has produced, shared with the reader thread.
    ///
    /// Held here and not only inside the pty session so `session.scrollback`
    /// can answer while the frames are still draining — the drain owns the
    /// receiver, and a history nobody can ask for is a history nobody has.
    pub(crate) ring: Arc<Mutex<devpit_pty::RingBuffer>>,
    /// What to insist to, when asking politely does not work.
    pub(crate) pid: Option<u32>,
}

impl Live {
    /// Ends this client, without waiting forever to be obeyed.
    ///
    /// `kill` on a pty child is `SIGHUP`, which is a request. A closed tab
    /// must not be able to hang the window because whatever was inside it
    /// decided to take its time.
    pub(crate) fn stop(&self) -> Result<(), RpcError> {
        let mut killer = self
            .killer
            .lock()
            .map_err(|_| RpcError::internal("killer lock"))?;
        devpit_pty::stop(self.pid, devpit_pty::GRACE, || {
            let _ = killer.kill();
        });
        Ok(())
    }
}

#[derive(Default)]
pub struct Claims {
    /// The attached client of each pane.
    lives: Mutex<HashMap<String, Arc<Live>>>,
    /// Who has claimed each pane and has not released it.
    held: Mutex<HashMap<String, String>>,
    /// The highest client id ever seen for a pane.
    ///
    /// Separate from `held` because a claim is released and this is not: an
    /// `invoke` that was in flight during a detach arrives afterwards, and
    /// without a high-water mark it would find the pane free and take it back.
    highest: Mutex<HashMap<String, String>>,
}

impl Claims {
    pub fn new() -> Self {
        Self::default()
    }

    /// Claims the pane for a newer client, and hands back the one it replaced.
    ///
    /// The caller kills what comes back. Returning it rather than killing it
    /// here keeps this type about ownership and nothing else.
    pub(crate) fn take_for(
        &self,
        pane_id: &str,
        client_id: &str,
    ) -> Result<Option<Arc<Live>>, RpcError> {
        let mut highest = self
            .highest
            .lock()
            .map_err(|_| RpcError::internal("client generation lock"))?;
        if highest
            .get(pane_id)
            .is_some_and(|current| current.as_str() >= client_id)
        {
            return Err(RpcError::new(
                ErrorCode::Conflict,
                "a newer client already owns that pane",
            ));
        }
        highest.insert(pane_id.to_owned(), client_id.to_owned());
        self.held
            .lock()
            .map_err(|_| RpcError::internal("session claim lock"))?
            .insert(pane_id.to_owned(), client_id.to_owned());
        drop(highest);

        Ok(self
            .lives
            .lock()
            .map_err(|_| RpcError::internal("session lock"))?
            .remove(pane_id))
    }

    /// Installs the pty ends, unless a newer client claimed the pane while
    /// this one was still starting up.
    pub(crate) fn install(
        &self,
        pane_id: &str,
        client_id: &str,
        live: Arc<Live>,
    ) -> Result<bool, RpcError> {
        let held = self
            .held
            .lock()
            .map_err(|_| RpcError::internal("session claim lock"))?;
        if held.get(pane_id).is_none_or(|current| current != client_id) {
            return Ok(false);
        }
        self.lives
            .lock()
            .map_err(|_| RpcError::internal("session lock"))?
            .insert(pane_id.to_owned(), live);
        Ok(true)
    }

    /// Gives up the claim, if it is still this client's to give up.
    pub fn release(&self, pane_id: &str, client_id: &str) -> Result<bool, RpcError> {
        let mut held = self
            .held
            .lock()
            .map_err(|_| RpcError::internal("session claim lock"))?;
        if held
            .get(pane_id)
            .is_some_and(|current| current == client_id)
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
