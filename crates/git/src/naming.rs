//! What a card's branch is called.
//!
//! The name no longer says whose branch it is: the card records the branch it
//! made, and that record is what cleanup trusts. An older devpit's
//! `devpit/<slug>-<id>` branches are still recognised by their id suffix.

use std::path::Path;

use crate::lifecycle::{short, unaccented};
use crate::run;

/// The name a card's branch gets unless the person chose another.
pub const BRANCH_PATTERN: &str = "{type}/{number}-{slug}";

/// The most a slug holds: the card number already makes the name unique.
const SLUG_MOST: usize = 30;

/// `pattern` filled in from the card: `{type}`, `{number}`, `{slug}` and
/// `{id}`. An empty pattern is [`BRANCH_PATTERN`].
pub fn branch_for(pattern: &str, title: &str, number: Option<u32>, card_id: &str) -> String {
    let pattern = match pattern.trim() {
        "" => BRANCH_PATTERN,
        chosen => chosen,
    };
    let (kind, rest) = kind_of(title);
    // The type word leaves the slug only for a name that says the type.
    let rest = if pattern.contains("{type}") {
        rest
    } else {
        title
    };
    let id = short(card_id);
    // A card from before numbers still needs a name of its own.
    let number = number.map_or_else(|| id.clone(), |number| number.to_string());
    pattern
        .replace("{type}", kind)
        .replace("{number}", &number)
        .replace("{slug}", &slug(rest))
        .replace("{id}", &id)
}

/// The conventional type a title opens with, and the title after it. A title
/// that opens with none is a feature.
fn kind_of(title: &str) -> (&'static str, &str) {
    let title = title.trim_start();
    let end = title
        .find(|ch: char| !ch.is_alphabetic())
        .unwrap_or(title.len());
    let kind = match title[..end].to_lowercase().as_str() {
        "fix" | "fixes" | "fixed" | "bug" | "bugfix" | "hotfix" | "defect" => "fix",
        "docs" | "doc" => "docs",
        "test" | "tests" => "test",
        "chore" => "chore",
        "refactor" => "refactor",
        "feat" | "feature" => "feat",
        _ => return ("feat", title),
    };
    (kind, &title[end..])
}

/// The first words of a title, without accents, up to [`SLUG_MOST`]
/// characters. Only a first word longer than that is cut.
fn slug(title: &str) -> String {
    let mut out = String::new();
    let words = unaccented(title).to_ascii_lowercase();
    for word in words
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
    {
        if out.is_empty() {
            out = word.chars().take(SLUG_MOST).collect();
        } else if out.len() + 1 + word.len() <= SLUG_MOST {
            out.push('-');
            out.push_str(word);
        } else {
            break;
        }
    }
    if out.is_empty() {
        // A title with nothing nameable in it still needs a branch.
        "card".to_owned()
    } else {
        out
    }
}

/// `wanted`, or the first of `wanted-2`, `wanted-3`… that no branch here or
/// on origin has yet.
pub fn unused_branch(root: &Path, wanted: &str) -> String {
    let taken = |name: &str| {
        ["refs/heads", "refs/remotes/origin"]
            .iter()
            .any(|refs| is_ref(root, &format!("{refs}/{name}")))
    };
    std::iter::once(wanted.to_owned())
        .chain((2..).map(|n| format!("{wanted}-{n}")))
        .find(|name| !taken(name))
        .unwrap_or_else(|| wanted.to_owned())
}

/// Whether `name`, such as `refs/heads/main`, is a ref here.
pub(crate) fn is_ref(root: &Path, name: &str) -> bool {
    run(root, &["rev-parse", "--verify", "--quiet", name]).is_ok()
}

/// Whether `pattern` can name a card's branch; why not when it cannot.
pub fn pattern_allowed(pattern: &str) -> Result<(), String> {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return Ok(());
    }
    if !["{number}", "{slug}", "{id}"]
        .iter()
        .any(|token| pattern.contains(token))
    {
        return Err(
            "the name needs {number}, {slug} or {id}, or every card gets the same one".to_owned(),
        );
    }
    let example = branch_for(pattern, "Fix the login redirect", Some(42), "card_01HZXY9K");
    if example.contains(['{', '}']) {
        return Err(format!("{example} has a placeholder devpit does not know"));
    }
    run(
        &std::env::temp_dir(),
        &["check-ref-format", "--branch", &example],
    )
    .map(|_| ())
    .map_err(|_| format!("{example} is not a name git accepts for a branch"))
}

#[cfg(test)]
#[path = "naming_tests.rs"]
mod tests;
