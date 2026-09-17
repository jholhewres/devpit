//! The values devpit knows are secret, taken out before a run's log is kept.
//!
//! A command step is given an environment, and commands echo their
//! environment. `set -x`, a failing `curl` printing the request it sent, a
//! test runner dumping config on a failure — none of them are misbehaving, and
//! all of them can put a token into a log that then sits in SQLite forever.
//!
//! **This is not detection.** It knows exactly the values this app handed out:
//! the account token, the hook secret, and what a profile declared. A secret
//! that reached the command another way — read from a file, fetched at
//! runtime, typed by somebody — passes straight through, and the public
//! documentation must never say otherwise.
//!
//! Applied on the way to disk rather than on the way to the window. A line
//! that already reached the screen cannot be unseen, and putting this in the
//! streaming path would pay for it on every line of every run. What is stored
//! is what gets read back tomorrow, and that is the copy worth cleaning.

/// Shorter than this and taking it out would redact ordinary words. A secret
/// of four characters is not a secret anybody is protecting.
const SHORTEST_WORTH_HIDING: usize = 8;

/// What replaces one.
const HIDDEN: &str = "[hidden by devpit]";

/// The values to take out, in the order they are taken.
///
/// Longest first, always: a token that contains a shorter one would otherwise
/// be half-replaced, leaving the rest of it in the log next to the marker.
pub(crate) fn worth_hiding(known: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut values: Vec<String> = known
        .into_iter()
        .filter(|value| value.len() >= SHORTEST_WORTH_HIDING)
        .collect();
    values.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    values.dedup();
    values
}

/// Takes every known secret out of `text`.
///
/// Plain replacement, not a pattern: what is known is known exactly, and a
/// regular expression over a megabyte of build output for each of half a dozen
/// values would cost more than the whole run.
pub(crate) fn kept_out(text: &str, secrets: &[String]) -> String {
    let mut clean = text.to_owned();
    for secret in secrets {
        if clean.contains(secret.as_str()) {
            clean = clean.replace(secret.as_str(), HIDDEN);
        }
    }
    clean
}

/// Everything this app handed out that is worth taking back.
///
/// Every profile's environment, not the one this run used: which profile ran
/// is not always known where the log is stored, and a value that is a secret
/// in one profile is a secret in the log whichever one put it there. A value
/// this cannot see is a value this cannot hide, which is the sentence the
/// module header exists to keep honest.
pub(crate) fn what_devpit_gave(store: &devpit_core::Store) -> Vec<String> {
    let mut known: Vec<String> = crate::agent_profiles::declared(store)
        .into_iter()
        .flat_map(|profile| profile.env)
        .map(|set| set.value)
        .collect();
    known.extend(crate::account::token::read());
    known.extend(crate::listener::secret_now());
    worth_hiding(known)
}

#[cfg(test)]
#[path = "kept_out_tests.rs"]
mod tests;
