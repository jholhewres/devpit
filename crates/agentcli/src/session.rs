//! What `agents --json` returns, and how it decodes.

use serde::Deserialize;

use crate::AgentError;

/// Whether a session is working right now.
///
/// An unknown value decodes to `Unknown` rather than failing: the vendor is
/// free to add a state, and a listing that refuses to render because of one
/// new word is worse than a session shown as unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Idle,
    Busy,
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
    pub name: Option<String>,
    pub cwd: String,
    pub pid: Option<i64>,
    pub started_at: Option<i64>,
    pub kind: Kind,
    pub status: Status,
}

#[derive(Deserialize)]
struct Raw {
    #[serde(rename = "sessionId")]
    session_id: String,
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
            status: match row.status.as_str() {
                "idle" => Status::Idle,
                "busy" => Status::Busy,
                _ => Status::Unknown,
            },
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Recorded from the installed CLI, not written from the documentation.
    const REAL: &[u8] = br#"[
      {"pid":843661,"cwd":"/home/x/p","kind":"interactive",
       "startedAt":1788877447347,"sessionId":"7ebf5e9c","name":"p-17","status":"busy"}
    ]"#;

    #[test]
    fn the_recorded_listing_decodes() {
        let sessions = parse_list(REAL).expect("decode");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "7ebf5e9c");
        assert_eq!(sessions[0].status, Status::Busy);
        assert_eq!(sessions[0].kind, Kind::Interactive);
        assert_eq!(sessions[0].pid, Some(843661));
    }

    /// The vendor adding a state must not empty the screen.
    #[test]
    fn an_unknown_status_decodes_instead_of_failing() {
        let sessions =
            parse_list(br#"[{"sessionId":"a","status":"hibernating","kind":"orbital"}]"#)
                .expect("decode");
        assert_eq!(sessions[0].status, Status::Unknown);
        assert_eq!(sessions[0].kind, Kind::Unknown);
    }

    /// No sessions is a normal answer, not a failure.
    #[test]
    fn an_empty_listing_is_not_an_error() {
        assert!(parse_list(b"[]").expect("decode").is_empty());
    }

    #[test]
    fn output_that_is_not_a_listing_is_reported_not_guessed() {
        assert!(matches!(
            parse_list(b"claude: command not found"),
            Err(AgentError::Unreadable(_))
        ));
    }
}
