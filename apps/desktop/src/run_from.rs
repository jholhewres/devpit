//! Where a run comes from.
//!
//! Three things that travel together and are read together: the column a
//! verdict sends the card back to, how many lanes this card has already passed
//! through, and what asked for the run at all. Apart from `runs.rs` because
//! that file is the *starting* and this is one of its arguments — and because
//! six arguments is where a function stops being readable.

use devpit_core::store::Asked;

/// Where one run comes from.
pub struct Asking<'a> {
    /// Where a verdict sends the card back to, recorded on the run's own row
    /// so it survives the process that started it. Without it a review that
    /// says "revise" leaves the card sitting in the reviewed column.
    pub came_from: Option<&'a str>,
    /// A person dropping a card starts at zero. The chain passes its own count
    /// on, so a flow edited into a circle while a chain is in flight still
    /// stops.
    pub hops: u8,
    pub asked: Asked,
}

impl<'a> Asking<'a> {
    /// A run somebody asked for, at the start of whatever chain follows.
    pub fn first(came_from: Option<&'a str>, asked: Asked) -> Self {
        Self {
            came_from,
            hops: 0,
            asked,
        }
    }
}
