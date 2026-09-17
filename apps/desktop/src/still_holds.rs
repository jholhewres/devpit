//! Whether a run's result is still about the code in front of you.
//!
//! The rule is [`devpit_rpc::validity`], which is pure and tested on its own.
//! This is the part that has to touch the world: it reads what the run
//! recorded and asks the repository where it stands now.
//!
//! **This does not prove isolation.** Two matching fingerprints mean git sees
//! the same tracked state — the same commit and the same uncommitted work.
//! They say nothing about an ignored file, an installed package, an
//! environment variable or the clock, every one of which can change a result.
//! The screen says "current", which is a claim about the code, and it does not
//! say "reproducible", which would be a claim about the machine.

use std::path::Path;

use devpit_core::store::Ran;
use devpit_rpc::{validity, Fingerprint, Validity};

/// What a run saw, as a fingerprint.
pub fn what_it_saw(ran: &Ran) -> Fingerprint {
    Fingerprint {
        revision: ran.head_revision.clone(),
        changes: ran.saw_changes.clone(),
    }
}

/// Where a directory stands now.
///
/// Both sides missing rather than one: a directory that is no repository
/// answers neither, and `validity` reads that as `Unknown` instead of letting
/// a half-answer decide anything.
pub fn where_it_stands(cwd: &Path) -> Fingerprint {
    Fingerprint {
        revision: devpit_git::head_of(cwd).ok(),
        changes: devpit_git::standing_at(cwd).ok(),
    }
}

/// Whether what a run said is still about what is there.
///
/// A run with no recorded directory has nowhere to look, and nowhere to look
/// is `Unknown` — never `Current` by omission.
pub fn still_holds(ran: &Ran) -> Validity {
    let Some(cwd) = ran.in_directory.as_deref() else {
        return Validity::Unknown;
    };
    validity(&what_it_saw(ran), &where_it_stands(Path::new(cwd)))
}

#[cfg(test)]
#[path = "still_holds_tests.rs"]
mod tests;
