//! The board a page at a time: each lane's first cards, how many it holds
//! and where the next page starts.
//!
//! Asked for only when the call says how (`limit`, `cursor`, `column`, `epic`,
//! `text` or `done`): without any of them `board` answers every card, as it
//! always did. The first page of a lane fixes its order in memory, a reading:
//! the pages after it read that order, less the cards that left the lane,
//! then the cards that entered it, so a card moved between two pages is read
//! once and pushes no other out of sight. A cursor whose reading is gone (the
//! app restarted, or it expired) resumes by the last card read and the first
//! not read, each with its place.

use devpit_rpc::{Board, Card, Column};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// How many cards a lane shows when the call does not say.
pub(crate) const LIMIT: usize = 20;
/// The most a lane shows in one page, whatever the call asks.
const MOST: usize = 200;

/// The parameters that ask for a page.
const PAGING: [&str; 6] = ["limit", "cursor", "column", "epic", "text", "done"];

/// Whether `params` asks for a page rather than the whole board.
pub(crate) fn asked(params: &Value) -> bool {
    // `full: true` is the whole board, whatever else came with it.
    params.get("full").and_then(Value::as_bool) != Some(true)
        && PAGING
            .iter()
            .any(|name| params.get(name).is_some_and(|value| !value.is_null()))
}

/// What a page is asked for.
struct Ask {
    limit: usize,
    after: Option<Cursor>,
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

/// Where a lane's next page starts: the last card read and the first not
/// read, each with the place it had, and the reading they belong to.
struct Cursor {
    column: String,
    last: (i64, String),
    next: (i64, String),
    reading: Option<String>,
}

/// `<column>:<place>:<last card>:<place>:<next card>:<reading>`.
fn cursor(column: &str, last: (i64, &str), next: (i64, &str), reading: &str) -> String {
    format!(
        "{column}:{}:{}:{}:{}:{reading}",
        last.0, last.1, next.0, next.1
    )
}

fn cursor_of(cursor: &str) -> Result<Cursor, String> {
    let refused = || {
        format!("`{cursor}` is not a cursor this board gave: pass a lane's `nextCursor` as it came")
    };
    let parts: Vec<&str> = cursor.split(':').collect();
    let (column, last_at, last, next_at, next, reading) = match parts[..] {
        [column, last_at, last, next_at, next] => (column, last_at, last, next_at, next, None),
        [column, last_at, last, next_at, next, reading] => (
            column,
            last_at,
            last,
            next_at,
            next,
            Some(reading.to_owned()),
        ),
        _ => return Err(refused()),
    };
    let place = |at: &str| at.parse::<i64>().map_err(|_| refused());
    Ok(Cursor {
        column: column.to_owned(),
        last: (place(last_at)?, last.to_owned()),
        next: (place(next_at)?, next.to_owned()),
        reading,
    })
}

impl Cursor {
    /// Where in `cards`, as they are now, the next page starts, when the
    /// reading is gone (the app restarted, or it expired). Each card the
    /// cursor names says where; when they disagree one of them moved, and it
    /// is the one whose place changed more. A tie repeats a card rather than
    /// skip one.
    fn start(&self, cards: &[&Card]) -> usize {
        let at = |id: &str| cards.iter().position(|card| card.id == id);
        let moved = |(was, id): &(i64, String)| {
            cards
                .iter()
                .find(|card| card.id == *id)
                .map(|card| i64::from(card.position).abs_diff(*was))
        };
        let after_last = at(&self.last.1).map(|at| at + 1);
        let at_next = at(&self.next.1);
        match (after_last, at_next) {
            (Some(one), Some(other)) if one == other => one,
            (Some(one), Some(other)) => match moved(&self.last).cmp(&moved(&self.next)) {
                std::cmp::Ordering::Greater => other,
                std::cmp::Ordering::Less => one,
                std::cmp::Ordering::Equal => one.min(other),
            },
            (Some(one), None) => one,
            (None, Some(other)) => other,
            // Both left the lane: by the place the next one had.
            (None, None) => cards.partition_point(|card| {
                (i64::from(card.position), card.id.as_str()) < (self.next.0, self.next.1.as_str())
            }),
        }
    }
}

/// How long a reading is kept after its last page.
const KEPT: Duration = Duration::from_secs(30 * 60);
/// The most readings kept at once; past it the oldest goes.
const READINGS: usize = 1024;

/// A lane being read page by page: its order as the first page saw it,
/// the cards that entered since at its end, and which were read.
#[derive(Clone)]
struct Reading {
    at: Instant,
    order: Vec<String>,
    read: HashSet<String>,
    page: Vec<String>,
}

fn readings() -> &'static Mutex<HashMap<String, Reading>> {
    static READINGS: OnceLock<Mutex<HashMap<String, Reading>>> = OnceLock::new();
    READINGS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn kept(name: &str) -> Option<Reading> {
    let readings = readings()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    readings
        .get(name)
        .filter(|reading| reading.at.elapsed() < KEPT)
        .cloned()
}

fn keep(reading: Reading) -> String {
    let name = ulid::Ulid::generate().to_string();
    let mut readings = readings()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    forget_old(&mut readings, Instant::now());
    readings.insert(name.clone(), reading);
    name
}

fn forget_old(readings: &mut HashMap<String, Reading>, now: Instant) {
    readings.retain(|_, reading| now.saturating_duration_since(reading.at) < KEPT);
    while readings.len() >= READINGS {
        let Some(oldest) = readings
            .iter()
            .min_by_key(|(_, reading)| reading.at)
            .map(|(name, _)| name.clone())
        else {
            break;
        };
        readings.remove(&oldest);
    }
}

/// The lane's cards not read yet, in the order the reading fixed: the lane
/// as the first page saw it, less the cards that left, then the cards that
/// entered since. With no reading (the first page, or one forgotten), the
/// lane as it is now, from where the cursor's cards say.
fn unread<'a>(cards: &[&'a Card], after: Option<&Cursor>) -> (Reading, Vec<&'a Card>) {
    let earlier = after.and_then(|after| {
        let reading = kept(after.reading.as_deref()?)?;
        Some((reading, &after.last.1))
    });
    let (order, read) = match earlier {
        Some((reading, last)) => {
            let mut read = reading.read;
            let upto = reading
                .page
                .iter()
                .position(|id| id == last)
                .map_or(reading.page.len(), |at| at + 1);
            read.extend(reading.page[..upto].iter().cloned());
            let mut order = reading.order;
            let known: HashSet<String> = order.iter().cloned().collect();
            order.extend(
                cards
                    .iter()
                    .filter(|card| !known.contains(&card.id))
                    .map(|card| card.id.clone()),
            );
            (order, read)
        }
        None => {
            let start = after.map_or(0, |after| after.start(cards));
            let order = cards.iter().map(|card| card.id.clone()).collect();
            let read = cards[..start].iter().map(|card| card.id.clone()).collect();
            (order, read)
        }
    };
    let here: HashMap<&str, &'a Card> =
        cards.iter().map(|card| (card.id.as_str(), *card)).collect();
    let rest = order
        .iter()
        .filter(|id| !read.contains(*id))
        .filter_map(|id| here.get(id.as_str()).copied())
        .collect();
    let reading = Reading {
        at: Instant::now(),
        order,
        read,
        page: Vec::new(),
    };
    (reading, rest)
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
        .or(ask.after.as_ref().map(|after| after.column.as_str()));
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
            let after = ask.after.as_ref().filter(|after| after.column == column.id);
            let (mut reading, rest) = unread(&cards, after);
            let shown = &rest[..ask.limit.min(rest.len())];
            // Each row says its place, so a page cut shorter can name its own cursor.
            lane["cards"] = json!(shown
                .iter()
                .map(|card| {
                    let mut listed = row(card);
                    listed["position"] = json!(card.position);
                    listed
                })
                .collect::<Vec<_>>());
            if shown.is_empty() {
                return lane;
            }
            reading.page = shown.iter().map(|card| card.id.clone()).collect();
            let name = keep(reading);
            if let (Some(last), Some(next)) = (shown.last(), rest.get(shown.len())) {
                lane["nextCursor"] = json!(cursor(
                    &column.id,
                    (last.position.into(), &last.id),
                    (next.position.into(), &next.id),
                    &name
                ));
            }
            // Where a page cut shorter resumes the same reading.
            lane["reading"] = json!(name);
            lane
        })
        .collect();
    Ok(json!({ "columns": columns }))
}

#[cfg(test)]
#[path = "board_page_tests.rs"]
mod tests;
