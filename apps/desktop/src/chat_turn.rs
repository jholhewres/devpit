//! What a chat turn settles before anything runs.
//!
//! Apart from `chat.rs`, which had grown to its ceiling holding the command and
//! every rule a turn follows at once.

use devpit_agentcli::head::Head;

/// The head written before the turn runs.
///
/// What the conversation already settled carries over; a turn may still change
/// the model, the budget, the permission mode and the effort.
pub(crate) fn opening(
    head: Option<&Head>,
    profile_id: &str,
    model: Option<String>,
    budget_usd: Option<f64>,
    permission: Option<String>,
    effort: Option<String>,
    now: f64,
) -> Head {
    Head {
        profile: profile_id.to_owned(),
        model,
        created_at: head.map_or(now, |head| head.created_at),
        card_id: head.and_then(|head| head.card_id.clone()),
        cost_usd: head.map(|head| head.cost_usd).unwrap_or_default(),
        budget_usd: budget_usd.or(head.and_then(|head| head.budget_usd)),
        session_id: head.and_then(|head| head.session_id.clone()),
        permission: permission
            .or_else(|| head.and_then(|head| head.permission.clone()))
            // Nothing to ask with yet: a turn left waiting on a prompt this
            // screen cannot draw would hang with no way to answer it.
            .or_else(|| Some("acceptEdits".to_owned())),
        effort: effort.or_else(|| head.and_then(|head| head.effort.clone())),
        title: head.and_then(|head| head.title.clone()),
        rewind: head.map(|head| head.rewind.clone()).unwrap_or_default(),
    }
}

#[cfg(test)]
#[path = "chat_turn_tests.rs"]
mod tests;
