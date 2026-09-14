//! What a conversation is, apart from what was said in it.
//!
//! Beside the JSONL rather than inside it: the messages are append-only, and
//! the profile is a fact about the whole thread.

use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Head {
    pub profile: String,
    pub model: Option<String>,
    pub card_id: Option<String>,
    /// Unix seconds.
    pub created_at: f64,
    /// What every turn so far actually cost, from the CLI's own report.
    #[serde(default)]
    pub cost_usd: f64,
    /// A ceiling for the whole conversation, when one was declared.
    #[serde(default)]
    pub budget_usd: Option<f64>,
    /// The CLI's id for this thread, so the next turn resumes it.
    #[serde(default)]
    pub session_id: Option<String>,
    /// What the agent may do without asking.
    #[serde(default)]
    pub permission: Option<String>,
    /// How hard it is asked to think.
    #[serde(default)]
    pub effort: Option<String>,
    /// A name to fall back on when the transcript holds none of the person's
    /// words — a session adopted from a terminal, titled by the CLI.
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub rewind: Rewind,
}

/// Where each turn sits in the CLI's own transcript, and a fork still to run.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rewind {
    /// The message the next turn forks at. Cleared once a turn has forked.
    #[serde(default)]
    pub fork_at: Option<String>,
    /// In turn order, one per turn that wrote an assistant message.
    #[serde(default)]
    pub anchors: Vec<Anchor>,
}

/// A turn's last assistant message, as `--resume-session-at` names it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Anchor {
    pub turn_id: String,
    /// The session that message is in; `/clear` mid-turn starts another.
    pub session_id: String,
    pub uuid: String,
}

/// The head once a turn has run.
///
/// A pending fork is only settled by an assistant message: a turn that died
/// before writing one keeps the original session, so the next turn forks again
/// rather than resuming a session the fork may never have written.
pub fn after_turn(
    opening: Head,
    turn_id: &str,
    cost_usd: Option<f64>,
    session_id: Option<String>,
    anchor: Option<String>,
) -> Head {
    let mut rewind = opening.rewind.clone();
    let session = match (session_id, anchor) {
        (Some(session), Some(uuid)) => {
            rewind.anchors.push(Anchor {
                turn_id: turn_id.to_owned(),
                session_id: session.clone(),
                uuid,
            });
            rewind.fork_at = None;
            Some(session)
        }
        (seen, None) if rewind.fork_at.is_none() => seen.or(opening.session_id.clone()),
        _ => opening.session_id.clone(),
    };
    Head {
        cost_usd: opening.cost_usd + cost_usd.unwrap_or_default(),
        session_id: session,
        rewind,
        ..opening
    }
}

pub fn head_path(home: &Path, project_id: &str, conversation_id: &str) -> PathBuf {
    home.join("projects")
        .join(project_id)
        .join("sessions")
        .join(format!("{conversation_id}.json"))
}

pub fn read_head(path: &Path) -> Option<Head> {
    let file = File::open(path).ok()?;
    serde_json::from_reader(file).ok()
}

pub fn write_head(path: &Path, head: &Head) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }
    let mut file = File::create(path)?;
    file.write_all(serde_json::to_string(head)?.as_bytes())
}

/// Whether this turn may run under the profile it names.
///
/// A conversation keeps the profile it opened with. Switching account
/// mid-thread would bill one transcript to two accounts and read as one.
/// `Err` names the profile it is already tied to.
pub fn settled<'a>(head: Option<&'a Head>, asked: &str) -> Result<(), &'a str> {
    match head {
        Some(head) if head.profile != asked => Err(&head.profile),
        _ => Ok(()),
    }
}

/// What is left of a conversation's cap, or nothing when it has none.
///
/// Checked before the turn starts: the CLI's own cap stops one part way
/// through, which is a turn too late to be a ceiling.
pub fn remaining(head: Option<&Head>) -> Option<f64> {
    let head = head?;
    head.budget_usd.map(|cap| cap - head.cost_usd)
}
