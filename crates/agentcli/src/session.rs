//! What `agents --json` returns, and how it decodes.

use serde::Deserialize;

use crate::AgentError;

/// What a session is doing.
///
/// Read off a real listing rather than guessed. The two kinds of session speak
/// differently: an interactive one reports `status: idle|busy`, a background
/// one reports `state: working|blocked|done` — and it reports `status: idle`
/// alongside, so reading `status` first would call a working session idle.
///
/// `Blocked` earns its own variant instead of folding into busy: it is the
/// agent waiting on a person, and it is the only state where nothing happens
/// until someone comes back. A board that cannot tell those apart shows five
/// cards working when one of them has been waiting on you for an hour.
///
/// An unknown word decodes to `Unknown` rather than failing: a listing that
/// refuses to render because of one new state is worse than one unknown row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Working right now.
    Busy,
    /// Waiting on a person.
    Blocked,
    /// Finished, and still listed.
    Done,
    /// Idle: attached and waiting for input, or between turns.
    Idle,
    Unknown,
}

/// Whether the session is one a person is sitting in front of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Interactive,
    Background,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct AgentSession {
    pub session_id: String,
    /// The short handle `attach`, `logs` and `stop` take. Background only.
    pub short_id: Option<String>,
    pub name: Option<String>,
    pub cwd: String,
    pub pid: Option<i64>,
    pub started_at: Option<i64>,
    pub kind: Kind,
    pub status: Status,
}

/// The two shapes the CLI answers with.
///
/// An interactive session reports `status: idle|busy`; a background one
/// reports `state: working` and carries a short `id` as well. Reading only the
/// first left every background session decoding as unknown — which is exactly
/// the session a card most needs to be honest about.
#[derive(Deserialize)]
struct Raw {
    #[serde(rename = "sessionId")]
    session_id: String,
    /// The short handle, on background sessions only.
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    state: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    cwd: String,
    #[serde(default)]
    pid: Option<i64>,
    #[serde(default, rename = "startedAt")]
    started_at: Option<i64>,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    status: String,
}

pub(crate) fn parse_list(bytes: &[u8]) -> Result<Vec<AgentSession>, AgentError> {
    let raw: Vec<Raw> =
        serde_json::from_slice(bytes).map_err(|err| AgentError::Unreadable(err.to_string()))?;
    Ok(raw
        .into_iter()
        .map(|row| AgentSession {
            session_id: row.session_id,
            name: row.name,
            cwd: row.cwd,
            pid: row.pid,
            started_at: row.started_at,
            kind: match row.kind.as_str() {
                "interactive" => Kind::Interactive,
                "background" => Kind::Background,
                _ => Kind::Unknown,
            },
            short_id: row.id,
            // `state` first: a background session carries both, and its
            // `status` is `idle` even while it works.
            status: match (row.state.as_str(), row.status.as_str()) {
                ("working", _) => Status::Busy,
                ("blocked", _) => Status::Blocked,
                ("done", _) => Status::Done,
                (_, "busy") => Status::Busy,
                (_, "idle") => Status::Idle,
                _ => Status::Unknown,
            },
        })
        .collect())
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
