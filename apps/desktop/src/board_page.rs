//! The board a page at a time: each lane's first cards, how many it holds
//! and where the next page starts.
//!
//! Asked for only when the call says how (`limit`, `cursor`, `column`, `epic`,
//! `text` or `done`): without any of them `board` answers every card, as it
//! always did. A cursor names the last card read by its place, never by a
//! count, so a card moved between two pages neither repeats nor pushes
//! another out of sight.

use devpit_rpc::{Board, Card, Column};
use serde_json::{json, Value};

/// How many cards a lane shows when the call does not say.
pub(crate) const LIMIT: usize = 20;
/// The most a lane shows in one page, whatever the call asks.
const MOST: usize = 200;

/// The parameters that ask for a page.
const PAGING: [&str; 6] = ["limit", "cursor", "column", "epic", "text", "done"];

/// Whether `params` asks for a page rather than the whole board.
pub(crate) fn asked(params: &Value) -> bool {
    PAGING
        .iter()
        .any(|name| params.get(name).is_some_and(|value| !value.is_null()))
}

/// What a page is asked for.
struct Ask {
    limit: usize,
    after: Option<(String, i32, String)>,
    column: Option<String>,
    epic: Option<String>,
    text: Option<String>,
    done: bool,
}

impl Ask {
    fn of(params: &Value) -> Result<Ask, String> {
        let text = |name: &str| {
            params
                .get(name)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|said| !said.is_empty())
                .map(str::to_owned)
        };
        let limit = match params.get("limit") {
            None | Some(Value::Null) => LIMIT,
            Some(limit) => limit
                .as_u64()
                .filter(|limit| *limit > 0)
                .ok_or("`limit` is a number of cards per lane, 1 or more")?
                .min(MOST as u64) as usize,
        };
        let after = text("cursor")
            .map(|cursor| cursor_of(&cursor))
            .transpose()?;
        Ok(Ask {
            limit,
            after,
            column: text("column"),
            epic: text("epic"),
            text: text("text").map(|text| text.to_lowercase()),
            done: params.get("done").and_then(Value::as_bool) == Some(true),
        })
    }

    fn keeps(&self, card: &Card) -> bool {
        self.epic
            .as_deref()
            .is_none_or(|epic| card.parent_id.as_deref() == Some(epic) || card.id == epic)
            && self.text.as_deref().is_none_or(|text| {
                card.title.to_lowercase().contains(text)
                    || card
                        .number
                        .is_some_and(|number| text.trim_start_matches('#') == number.to_string())
            })
    }
}

/// `<column>:<position>:<card>`: the last card a page showed.
fn cursor(column: &str, card: &Card) -> String {
    format!("{column}:{}:{}", card.position, card.id)
}

fn cursor_of(cursor: &str) -> Result<(String, i32, String), String> {
    let refused = || {
        format!("`{cursor}` is not a cursor this board gave: pass a lane's `nextCursor` as it came")
    };
    let mut parts = cursor.splitn(3, ':');
    let (Some(column), Some(position), Some(card)) = (parts.next(), parts.next(), parts.next())
    else {
        return Err(refused());
    };
    let position = position.parse().map_err(|_| refused())?;
    Ok((column.to_owned(), position, card.to_owned()))
}

/// The page `params` asks for: each lane with its total, its first cards
/// after the cursor, and `nextCursor` while there are more. A lane whose role
/// is done is counted only, unless `done: true` or the call names it.
pub(crate) fn page(
    board: &Board,
    params: &Value,
    row: impl Fn(&Card) -> Value,
) -> Result<Value, String> {
    let ask = Ask::of(params)?;
    let named = ask
        .column
        .as_deref()
        .or(ask.after.as_ref().map(|(column, _, _)| column.as_str()));
    let lanes: Vec<&Column> = match named {
        Some(named) => vec![board
            .columns
            .iter()
            .find(|column| column.id == named || column.name.eq_ignore_ascii_case(named))
            .ok_or_else(|| {
                format!("no lane `{named}` on this board: `devpit_board` lists them")
            })?],
        None => board.columns.iter().collect(),
    };
    let columns: Vec<Value> = lanes
        .into_iter()
        .map(|column| {
            let mut cards: Vec<&Card> = board
                .cards
                .iter()
                .filter(|card| card.column_id == column.id && ask.keeps(card))
                .collect();
            cards.sort_by(|one, other| (one.position, &one.id).cmp(&(other.position, &other.id)));
            let total = cards.len();
            let mut lane = json!({
                "id": column.id,
                "name": column.name,
                "runsAStep": column.step.is_some(),
                "total": total,
            });
            let counted_only =
                column.role.as_deref() == Some("done") && !ask.done && named.is_none();
            if counted_only {
                lane["counted"] = json!("its cards come with `done: true` or `column`");
                return lane;
            }
            let start = match &ask.after {
                Some((at, position, id)) if *at == column.id => {
                    cards.partition_point(|card| (card.position, &card.id) <= (*position, id))
                }
                _ => 0,
            };
            let shown = &cards[start..(start + ask.limit).min(total)];
            lane["cards"] = json!(shown.iter().map(|card| row(card)).collect::<Vec<_>>());
            if start + shown.len() < total {
                if let Some(last) = shown.last() {
                    lane["nextCursor"] = json!(cursor(&column.id, last));
                }
            }
            lane
        })
        .collect();
    Ok(json!({ "columns": columns }))
}

#[cfg(test)]
#[path = "board_page_tests.rs"]
mod tests;
