//! The buttons under a notice, on any channel: kept here by an opaque key, so
//! a channel carries only the key, and done here when one is pressed.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::channels::Action;

/// Buttons kept answerable; older ones say they have expired.
const KEPT: usize = 200;

fn held() -> &'static Mutex<HashMap<String, Action>> {
    static HELD: OnceLock<Mutex<HashMap<String, Action>>> = OnceLock::new();
    HELD.get_or_init(Mutex::default)
}

/// Keeps `action` and answers the key a channel carries for it.
pub(crate) fn remember(action: &Action) -> String {
    let key = ulid::Ulid::generate().to_string();
    if let Ok(mut held) = held().lock() {
        if held.len() > KEPT {
            held.clear();
        }
        held.insert(key.clone(), action.clone());
    }
    key
}

/// The action a key stood for, once.
pub(crate) fn take(key: &str) -> Option<Action> {
    held().lock().ok()?.remove(key)
}

pub(crate) fn label(action: &Action) -> String {
    match action {
        Action::SendDraft { .. } => "Send the draft".to_owned(),
        Action::ReminderDone { .. } => "Done".to_owned(),
        Action::Snooze { .. } => "In 15 minutes".to_owned(),
        Action::Answer { seen, choice, .. } => {
            let said = seen
                .options
                .get(*choice as usize)
                .map_or("", |one| one.label.as_str());
            // A button's label is short: cut at a word, and say it was cut.
            let short = if said.chars().count() <= 26 {
                said.to_owned()
            } else {
                let cut: String = said.chars().take(26).collect();
                format!(
                    "{}…",
                    cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head)
                )
            };
            format!("{}. {short}", choice + 1)
        }
    }
}

/// Does what a button asks, here, as the person on `channel`; answers what happened.
pub(crate) fn act(action: &Action, channel: &str) -> String {
    let now = devpit_core::reports::now() as i64;
    let done = match action {
        Action::SendDraft { profile, session } => {
            match crate::reply_drafts::drafted(profile, session) {
                Some(draft) => {
                    crate::live_sessions::reply_now(profile, session, &draft, Some(channel))
                        .map(|_| format!("Sent to {session}."))
                        .map_err(|err| err.message)
                }
                None => Err("That draft is no longer waiting.".to_owned()),
            }
        }
        Action::ReminderDone { card_id } => crate::projects::store()
            .map_err(|err| err.message)
            .and_then(|store| {
                store
                    .handle_reminder(card_id, now)
                    .map_err(|err| err.to_string())
            })
            .map(|_| "Marked done.".to_owned()),
        Action::Snooze { card_id } => crate::projects::store()
            .map_err(|err| err.message)
            .and_then(|store| {
                crate::reminders::snooze(&store, card_id, (now + 15 * 60) as f64, now)
                    .map_err(|err| err.message)
            })
            .map(|_| "Again in 15 minutes.".to_owned()),
        Action::Answer {
            target,
            seen,
            choice,
        } => crate::live_answer::answer_in(target, seen, Some(*choice))
            .map(|_| format!("Answered: {}.", label(action)))
            .map_err(|err| err.message),
    };
    if matches!(action, Action::ReminderDone { .. } | Action::Snooze { .. }) {
        if let Some(app) = crate::telegram::app().get() {
            crate::reminders::changed(app);
        }
    }
    done.unwrap_or_else(|why| why)
}

#[cfg(test)]
mod tests {
    use super::{remember, take};
    use crate::channels::Action;

    #[test]
    fn an_answer_button_says_its_number_and_the_choice() {
        let seen = devpit_rpc::PendingPrompt {
            question: "Run the migration?".to_owned(),
            options: vec![devpit_rpc::PromptOption {
                label: "Yes, and don't ask again for this command".to_owned(),
                hint: None,
            }],
            cursor: 0,
        };
        let said = super::label(&Action::Answer {
            target: "t".to_owned(),
            seen,
            choice: 0,
        });
        assert_eq!(said, "1. Yes, and don't ask again…");
    }

    #[test]
    fn a_key_stands_for_its_action_once() {
        let action = Action::Snooze {
            card_id: "card_1".to_owned(),
        };
        let key = remember(&action);
        assert_eq!(take(&key), Some(action));
        assert_eq!(take(&key), None);
        assert_eq!(take("made-up"), None);
    }
}
