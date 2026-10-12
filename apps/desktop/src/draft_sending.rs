//! A draft sent at the orchestrator's call, on the person's own word.
//!
//! The orchestrator asks (`devpit_send_draft`); devpit reads the person's last
//! message in the orchestrator's log itself and sends only when that message
//! lets this draft go (`devpit_agentcli::person`). So "envie" counts from
//! wherever the person spoke to the orchestrator — the composer or the Claude
//! app — and never from another session, a tool or the agent's own account.
//! One message lets one draft go: a second ask on the same words is refused.
//! From a channel's chat a word lets nothing go: the draft waits for the
//! person's press on its button there.

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use devpit_agentcli::person::{authorises, bare, last_said, From, Said};
use serde_json::{json, Value};

/// A message that let a draft go: (profile, message id, session). A bare
/// "envie" names no session, so it is spent whole.
type Spent = (String, String, String);

fn spent() -> &'static Mutex<Vec<Spent>> {
    static SPENT: OnceLock<Mutex<Vec<Spent>>> = OnceLock::new();
    SPENT.get_or_init(Mutex::default)
}

/// Records `key` as spent; false when it already was.
pub(crate) fn spend(held: &mut Vec<Spent>, key: Spent) -> bool {
    if held.contains(&key) {
        return false;
    }
    held.push(key);
    true
}

/// What sending to `name` on `said` spends.
pub(crate) fn key_of(profile: &str, said: &Said, name: &str) -> Spent {
    let session = if bare(&said.text) { "" } else { name };
    (profile.to_owned(), said.id.clone(), session.to_owned())
}

/// Whether what the person said lets the draft for `name` go, with
/// `waiting` drafts in all. Never a word from a channel: there the yes is a
/// press on devpit's own button.
pub(crate) fn lets_go(said: &Said, name: &str, waiting: usize) -> bool {
    !matches!(said.from, From::Channel(_)) && authorises(&said.text, name, waiting)
}

/// Sends the draft for `name` when the person's last word in the log of the
/// orchestrator of `project`, standing in `root`, asks for it.
pub(crate) fn send(project: &str, profile: &str, root: &Path, name: &str) -> Result<Value, String> {
    let drafted = crate::reply_drafts::drafted(profile, name).ok_or_else(|| {
        format!("no draft waits for {name}: write it with devpit_draft_reply first")
    })?;
    let press = crate::channels::Action::SendDraft {
        profile: profile.to_owned(),
        session: name.to_owned(),
    };
    if crate::channel_talk::offer(project, press) {
        return Ok(crate::channel_talk::pending(
            project,
            &format!("send the draft to {name}"),
        ));
    }
    let config = crate::live_sessions::config_of(profile).map_err(|err| err.message)?;
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let log = devpit_agentcli::peers::newest(&devpit_agentcli::peers::logs_of(&config, &root))
        .ok_or(
            "devpit could not find this conversation's log, so it cannot read what the person said",
        )?;
    let said = last_said(&devpit_agentcli::peers::tail(&log))
        .ok_or("devpit found nothing the person said in this conversation")?;
    let waiting = crate::reply_drafts::all()
        .iter()
        .filter(|(whose, _, _)| whose == profile)
        .count();
    if !lets_go(&said, name, waiting) {
        return Err(match said.from {
            From::Channel(_) => format!(
                "not sent: the person last wrote from {}, where a draft goes only by its button. It waits beside the session for their click.",
                said.from.label()
            ),
            _ => format!(
                "not sent: the person's last message («{}») does not ask for the draft to {name} to go. It waits beside the session for their click, or for them to say \"envie\" or name the session.",
                short(&said.text)
            ),
        });
    }
    let mut held = spent()
        .lock()
        .map_err(|_| "the record of sends could not be read")?;
    if held.contains(&key_of(profile, &said, name)) {
        return Err(
            "not sent: that message of the person's already sent this; one message sends once"
                .to_owned(),
        );
    }
    let from = format!("{} («{}»)", said.from.label(), short(&said.text));
    crate::live_sessions::reply_now(profile, name, &drafted, Some(&from))
        .map_err(|err| err.message)?;
    let _ = spend(&mut held, key_of(profile, &said, name));
    Ok(json!({ "sent": name, "from": said.from.label(), "words": drafted }))
}

/// A message cut to what a record needs.
fn short(text: &str) -> String {
    let mut cut: String = text.chars().take(80).collect();
    if text.chars().count() > 80 {
        cut.push('…');
    }
    cut
}

#[cfg(test)]
#[path = "draft_sending_tests.rs"]
mod tests;
