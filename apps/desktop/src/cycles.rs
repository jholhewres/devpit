//! Whether a board's flow can send a card round in a circle.
//!
//! Two automatic lanes pointing at each other is a card that moves for ever,
//! spending money on every hop, with nobody watching — which is the worst
//! thing the automatic mode can do and the only one it can do entirely on its
//! own.
//!
//! Guarded twice, because one guard is not enough and the reasons differ:
//!
//!   - **Refused at the write.** A flow that closes a loop is refused when it
//!     is saved, while somebody is looking at what they chose. This is the one
//!     that explains itself.
//!   - **Capped at the run.** The graph can be edited while a chain is
//!     mid-flight, so the chain also counts its own hops. This one never
//!     explains anything and should never fire; it is the backstop.

use std::collections::{HashMap, HashSet};

/// How many lanes one pass may carry a card through before the chain stops.
///
/// Not a limit anybody should reach: a board with more than this many lanes in
/// a row, each approving automatically, is a board doing something nobody
/// designed. It exists because the write-time guard cannot see a graph edited
/// after the chain began.
pub const MOST_HOPS: u8 = 32;

/// Generous enough that no real board meets it. A compile-time check rather
/// than a test, because the thing being guarded is the constant itself and a
/// test would only restate it.
const _: () = assert!(MOST_HOPS >= 16);

/// Whether following `on_pass` from `start` ever returns to a lane it has
/// already been in.
///
/// `flow` is every lane's destination, including the one being proposed —
/// the caller applies the change to its copy first, so this answers about the
/// board that *would* exist rather than the one that does.
pub fn loops(flow: &HashMap<String, String>, start: &str) -> bool {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut here = start;
    loop {
        if !seen.insert(here) {
            return true;
        }
        match flow.get(here) {
            Some(next) => here = next,
            None => return false,
        }
    }
}

#[cfg(test)]
#[path = "cycles_tests.rs"]
mod tests;
