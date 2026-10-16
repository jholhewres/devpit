//! A plan changed after its cards exist: a card merged into another, whose
//! evidence it then finishes on, or cut from the plan. Either one stops
//! holding the cards that wait on it.
//!
//! A card merged into another never closed, because the gate asked for its
//! own evidence, and the cards waiting on it never started. Like a plan's
//! apply, the change waits for the person's yes: without `apply` it only says
//! what would change.

use devpit_core::store::card_numbers::card_number_of;
use devpit_core::{CardRow, Store};
use devpit_devflow::plan::PREFIX;
use serde_json::{json, Value};

/// Whether the call changes cards that exist rather than an epic's plan.
pub(crate) fn asks(params: &Value) -> bool {
    ["merge", "cut"]
        .iter()
        .any(|key| params.get(*key).is_some_and(|given| !given.is_null()))
}

/// What the person is asked to say yes to.
enum Change {
    Merge { from: CardRow, into: CardRow },
    Cut(CardRow),
}

impl Change {
    fn card(&self) -> &CardRow {
        match self {
            Change::Merge { from, .. } => from,
            Change::Cut(card) => card,
        }
    }

    fn target(&self) -> Option<&CardRow> {
        match self {
            Change::Merge { into, .. } => Some(into),
            Change::Cut(_) => None,
        }
    }

    fn said(&self) -> String {
        match self {
            Change::Merge { from, into } => format!(
                "\"{}\" ({}) merged into \"{}\" ({}): it finishes on that card's evidence",
                from.title, from.id, into.title, into.id
            ),
            Change::Cut(card) => format!(
                "\"{}\" ({}) cut from the plan: it closes without evidence",
                card.title, card.id
            ),
        }
    }
}

/// `devflow_plan` with `merge: { from, into }` or `cut: <card>`: a preview,
/// or with `apply` after the person's yes, the change. `relayed` is the yes
/// an orchestrator passed on, in the person's words.
pub(crate) fn respond(
    store: &Store,
    project_id: &str,
    params: &Value,
    relayed: Option<&str>,
) -> Result<Value, String> {
    let change = read(store, project_id, params)?;
    let card = change.card();
    let waiting: Vec<Value> = store
        .card_edges(project_id)
        .map_err(|err| err.to_string())?
        .blockers
        .iter()
        .filter(|(_, by)| *by == card.id)
        .filter_map(|(id, _)| store.card(id).ok().flatten())
        .map(|row| json!({ "id": row.id, "title": row.title }))
        .collect();
    let apply = params.get("apply").and_then(Value::as_bool) == Some(true);
    if !apply {
        return Ok(json!({
            "applied": false,
            "change": change.said(),
            "releases": waiting,
            "note": "Nothing was changed. Show the person this change and the cards it lets \
                     start; after their explicit yes, call again with the same change and \
                     apply: true.",
        }));
    }
    let into = change.target().map(|into| into.id.as_str());
    if !store
        .close_card(&card.id, into)
        .map_err(|err| err.to_string())?
    {
        return Err(format!(
            "nothing was changed: {} cannot take another card's work",
            into.unwrap_or_default()
        ));
    }
    let whose = match relayed {
        Some(quote) => format!("after the person's yes, passed on by the orchestrator: «{quote}»"),
        None => "after the person's yes in the conversation".to_owned(),
    };
    let note = format!("{PREFIX}{} {whose}.", capitalized(&change.said()));
    let _ = store.add_comment(&card.id, "agent", &note);
    Ok(json!({
        "applied": true,
        "change": change.said(),
        "releases": waiting,
        "note": "Changed. The cards in releases no longer wait on it; finish it with \
                 devpit_finish_card when the person wants it off the board.",
    }))
}

fn capitalized(said: &str) -> String {
    let mut chars = said.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// The change asked, each card on this project's board and the merge one a
/// card can take: never into itself, nor into a card already merged or cut.
fn read(store: &Store, project_id: &str, params: &Value) -> Result<Change, String> {
    let named = |value: Option<&Value>, what: &str| -> Result<CardRow, String> {
        let text = value
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .ok_or_else(|| format!("which card is {what}? pass its id or #number"))?;
        own_card(store, project_id, text)
    };
    let merge = params.get("merge").filter(|given| !given.is_null());
    let cut = params.get("cut").filter(|given| !given.is_null());
    match (merge, cut) {
        (Some(_), Some(_)) => Err("one change at a time: `merge` or `cut`, not both".to_owned()),
        (Some(merge), None) => {
            let from = named(merge.get("from"), "merged (`merge.from`)")?;
            let into = named(merge.get("into"), "taking its work (`merge.into`)")?;
            if from.id == into.id {
                return Err("a card cannot be merged into itself".to_owned());
            }
            if store
                .card_closed(&into.id)
                .map_err(|err| err.to_string())?
                .is_some()
            {
                return Err(format!(
                    "\"{}\" was itself merged or cut: merge into the card that holds the work",
                    into.title
                ));
            }
            Ok(Change::Merge { from, into })
        }
        (None, Some(cut)) => Ok(Change::Cut(named(Some(cut), "cut (`cut`)")?)),
        (None, None) => Err("pass `merge: { from, into }` or `cut: <card>`".to_owned()),
    }
}

/// A card of this project's board, by id or `#number`.
fn own_card(store: &Store, project_id: &str, text: &str) -> Result<CardRow, String> {
    let id = match card_number_of(text) {
        Some((_, number)) => store
            .card_numbered(project_id, number)
            .map_err(|err| err.to_string())?
            .ok_or_else(|| format!("no card #{number} on this project's board"))?,
        None => text.to_owned(),
    };
    let theirs = store
        .project_id_of_card(&id)
        .map_err(|err| err.to_string())?;
    store
        .card(&id)
        .map_err(|err| err.to_string())?
        .filter(|_| theirs.as_deref() == Some(project_id))
        .ok_or_else(|| format!("no card `{text}` on this project's board"))
}

#[cfg(test)]
#[path = "devflow_plan_edit_tests.rs"]
mod tests;
