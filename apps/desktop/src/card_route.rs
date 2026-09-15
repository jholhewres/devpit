//! Which card a pane belongs to, from the leaf a hook named.
//!
//! Read off the layouts rather than remembered when the leaf was made: a tab
//! already says which card it is for, and a second record of the same fact
//! would have to be kept right on every split, close and restore.

use devpit_core::Store;

use crate::sessions::{card_of_tab, decode};

/// A pane that belongs to a card.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Route {
    pub project_id: String,
    pub card_id: String,
    pub tab_id: String,
}

/// The card whose tab holds this leaf, if one does.
///
/// Only a card still on a board, and only in the project the layout is filed
/// under: a tab id comes from the window, and a `tab_card_` of another
/// project's card must not ring that card.
pub(crate) fn card_of_leaf(store: &Store, leaf: &str) -> Option<Route> {
    if !crate::adopting::plain(leaf) {
        return None;
    }
    let layouts = store.card_tab_layouts().ok()?;
    let layout = layouts.into_iter().find(|layout| {
        decode(&layout.project_id, &layout.tree, &layout.focused_id)
            .is_ok_and(|decoded| decoded.tree.contains_leaf(leaf))
    })?;
    let card = card_of_tab(&layout.tab_id)?;
    let project = store.live_card_project(card).ok().flatten()?;
    (project == layout.project_id).then(|| Route {
        project_id: layout.project_id.clone(),
        card_id: card.to_owned(),
        tab_id: layout.tab_id.clone(),
    })
}

/// Who a pane's bell is about, when its state rings one.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Ring<'a> {
    pub project_id: Option<&'a str>,
    pub card_id: Option<&'a str>,
}

/// Only `waiting` rings. In a card's pane the notice opens that card; anywhere
/// else it rings as it always has, about no card.
pub(crate) fn notice_for<'a>(route: Option<&'a Route>, state: Option<&str>) -> Option<Ring<'a>> {
    (state == Some("waiting")).then(|| Ring {
        project_id: route.map(|route| route.project_id.as_str()),
        card_id: route.map(|route| route.card_id.as_str()),
    })
}

#[cfg(test)]
#[path = "card_route_tests.rs"]
mod tests;
