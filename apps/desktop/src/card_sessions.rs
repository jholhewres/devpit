//! Every session of a card, as the card and its tile read them.
//!
//! Joined when read, from where each link already lives — the card's tab, its
//! runs, its background session, its conversations — with the state the app
//! has heard. Nothing here is stored a second time.

use devpit_agentcli::AgentSession;
use devpit_core::Store;
use devpit_rpc::{CardHappening, CardSession, SessionKind};

use crate::card_activity::background_state;
use crate::sessions::{decode, tab_for_card};

pub(crate) fn card_sessions(
    store: &Store,
    project_id: &str,
    card_id: &str,
    live: &[AgentSession],
    heard: &CardHappening,
) -> Vec<CardSession> {
    let said = |kind: SessionKind, reference: &str| {
        heard
            .sessions
            .iter()
            .find(|one| one.kind == kind && one.reference == reference)
            .and_then(|one| one.state)
    };
    let unplaced = |kind: SessionKind, reference: &str| CardSession {
        kind,
        reference: reference.to_owned(),
        state: said(kind, reference),
        tab_id: None,
        leaf_id: None,
    };
    let mut sessions = Vec::new();

    // Every pane of the card's tab, whether or not its agent has said anything:
    // the card can always go to its terminal.
    let tab = tab_for_card(card_id);
    if let Ok(Some((tree, focused))) = store.pane_layout(project_id, &tab) {
        if let Ok(layout) = decode(project_id, &tree, &focused) {
            for (leaf, _) in layout.tree.leaves() {
                sessions.push(CardSession {
                    tab_id: Some(tab.clone()),
                    leaf_id: Some(leaf.to_owned()),
                    ..unplaced(SessionKind::Pane, leaf)
                });
            }
        }
    }

    if let Ok(links) = store.card_links(card_id) {
        for run in &links.runs {
            sessions.push(unplaced(SessionKind::Run, &run.session_id));
        }
        if let Some(background) = &links.background {
            let listed = live
                .iter()
                .find(|one| one.session_id == background.session_id)
                .map(|one| &one.status);
            sessions.push(CardSession {
                state: Some(background_state(
                    listed,
                    said(SessionKind::Background, &background.session_id),
                )),
                ..unplaced(SessionKind::Background, &background.session_id)
            });
        }
        for chat in &links.chats {
            sessions.push(unplaced(SessionKind::Chat, &chat.conversation_id));
        }
    }

    // What was heard with no link to show for it: a run with no agent is
    // heard under its own id, and has no session on its row.
    for one in &heard.sessions {
        if !sessions
            .iter()
            .any(|listed| listed.kind == one.kind && listed.reference == one.reference)
        {
            sessions.push(one.clone());
        }
    }
    sessions
}

#[cfg(test)]
#[path = "card_sessions_tests.rs"]
mod tests;
