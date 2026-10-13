//! `#42` for a card: wherever an agent passes a card id it may pass the
//! card's number instead. Turned into the id here, once, before any method
//! reads its params, so every method takes both without knowing.

use devpit_core::store::card_numbers::card_number_of;
use devpit_rpc::Project;
use serde_json::Value;

/// The params that hold a card id.
const KEYS: [&str; 3] = ["cardId", "card", "epic"];

/// The project a number names, as in `devpit#42`, when the call names none
/// and it is not the project the agent stands in.
pub(crate) fn project_named<'a>(params: &'a Value, here: &Project) -> Option<&'a str> {
    KEYS.iter()
        .filter_map(|key| params.get(key).and_then(Value::as_str))
        .filter_map(|text| card_number_of(text)?.0)
        .find(|named| !is(here, named))
}

/// The params with each card number turned into the card's id in `project`,
/// asked of `numbered`. A number no card holds is refused, not passed on.
pub(crate) fn resolved(
    params: &Value,
    project: &Project,
    numbered: impl Fn(i64) -> Result<Option<String>, String>,
) -> Result<Value, String> {
    let mut params = params.clone();
    for key in KEYS {
        let Some(text) = params.get(key).and_then(Value::as_str) else {
            continue;
        };
        let Some((named, number)) = card_number_of(text) else {
            continue;
        };
        if named.is_some_and(|named| !is(project, named)) {
            return Err(format!(
                "`{text}` names another project than {}",
                project.name
            ));
        }
        let id = numbered(number)?
            .ok_or_else(|| format!("no card #{number} on {}'s board", project.name))?;
        params[key] = Value::String(id);
    }
    Ok(params)
}

fn is(project: &Project, named: &str) -> bool {
    project.id == named || project.name.eq_ignore_ascii_case(named)
}

#[cfg(test)]
#[path = "card_numbers_tests.rs"]
mod tests;
