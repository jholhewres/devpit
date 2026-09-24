//! `start` — an orchestrator hands a card's work to a session of its own
//! account.
//!
//! The session is a background one tied to the card, in the card's own
//! checkout: the board and the card's Sessions say it is there, it can be
//! attached to a terminal, and it is named so the orchestrator can message it
//! and ask to hear when it goes idle. Nothing starts anywhere else, and nothing
//! starts out of sight — which is what an orchestrator is allowed on the
//! condition of.

use devpit_rpc::Board;
use serde_json::{json, Value};

/// The longest first message handed to a session. A brief, not a document:
/// the card holds the rest.
const LONGEST_PROMPT: usize = 8000;

/// Starts the session and links it to the card. Answers with the name to
/// message it by.
pub(crate) fn hand(
    board: &Board,
    profile_id: &str,
    card_id: &str,
    prompt: &str,
    named: Option<&str>,
) -> Result<Value, String> {
    let prompt = prompt.trim();
    if prompt.is_empty() || prompt.chars().count() > LONGEST_PROMPT {
        return Err(format!(
            "a session is handed between 1 and {LONGEST_PROMPT} characters of work"
        ));
    }
    let card = board
        .cards
        .iter()
        .find(|card| card.id == card_id)
        .ok_or("that card is not on this project's board")?;
    let name = session_name(named.unwrap_or(&card.title), card_id);

    let store = crate::projects::store().map_err(|err| err.message)?;
    // The card's own checkout: two sessions at work in one project do not
    // write over each other, and the card's Changes show what this one did.
    let cwd = crate::checkout::checkout_of(&store, card_id, |_| {})?;
    let runner = crate::agent_profiles::runner_for(&store, profile_id)?;
    let session_id = crate::steps::fresh_session_id();
    let short = devpit_agentcli::start_background(
        &cwd,
        Some(&runner),
        Some(&session_id),
        None,
        None,
        crate::steps::hook_settings().as_deref(),
        Some(&devpit_agentcli::Handed {
            name: &name,
            prompt,
        }),
    )
    .map_err(|err| untrusted(&err.to_string(), &cwd).unwrap_or_else(|| err.to_string()))?;

    let transcript = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .map(|home| devpit_agentcli::transcript_path(&home, &cwd, &session_id));
    store
        .link_session(
            card_id,
            &short,
            &session_id,
            transcript.as_ref().and_then(|path| path.to_str()),
            cwd.to_str(),
            Some(profile_id),
        )
        .map_err(|err| err.to_string())?;

    Ok(json!({
        "name": name,
        "sessionId": session_id,
        "cardId": card_id,
        "cwd": cwd.display().to_string(),
        "next": "Message it by this name with SendMessage, and pass notify_when_idle to hear when it is done.",
    }))
}

/// What to do when Claude Code will not start in the card's checkout because
/// nobody has told it to trust that folder yet. Trust is inherited, so the
/// folder all checkouts sit in, trusted once, covers every card after it.
/// devpit does not write the CLI's own settings to do it: running sessions
/// rewrite that file, and a second writer is how it gets corrupted.
pub(crate) fn untrusted(said: &str, cwd: &std::path::Path) -> Option<String> {
    said.contains("not trusted").then(|| {
        let base = cwd.parent().and_then(|p| p.parent()).unwrap_or(cwd);
        format!(
            "Claude Code does not trust {} yet, so it will not start a session there. \
             Ask the person to run `claude` once in {} and accept the trust prompt — that \
             covers every card's checkout — then hand the card again.",
            cwd.display(),
            base.display()
        )
    })
}

/// A name another session can address: the card's words, short, plain, and
/// the end of its id so two cards of one title are two names.
pub(crate) fn session_name(title: &str, card_id: &str) -> String {
    let mut out = String::new();
    for ch in title.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let words: String = out.trim_matches('-').chars().take(28).collect();
    let words = words.trim_end_matches('-');
    let tail: String = card_id
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let tail = tail.to_ascii_lowercase();
    if words.is_empty() {
        format!("card-{tail}")
    } else {
        format!("{words}-{tail}")
    }
}

#[cfg(test)]
mod tests {
    use super::{session_name, untrusted};

    #[test]
    fn an_untrusted_checkout_says_which_folder_to_trust_once() {
        let cwd = std::path::Path::new("/home/me/.devpit/worktrees/prj_1/card_9");
        let said =
            untrusted("Workspace not trusted. Run `claude` in … once", cwd).expect("explained");
        assert!(said.contains("/home/me/.devpit/worktrees "), "{said}");
        assert_eq!(untrusted("some other failure", cwd), None);
    }

    #[test]
    fn a_session_is_named_after_its_card_and_told_apart_by_its_id() {
        assert_eq!(
            session_name("Fix the login timeout!", "card_01ABCD9XYZ"),
            "fix-the-login-timeout-9xyz"
        );
        assert_eq!(session_name("   ", "card_01ABCD9XYZ"), "card-9xyz");
        assert_ne!(
            session_name("Same", "card_1111"),
            session_name("Same", "card_2222")
        );
    }
}
