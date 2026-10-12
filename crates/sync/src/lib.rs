//! Sync between a person's machines, through devpit.app, which cannot read
//! what it keeps.
//!
//! One key per account, made on the first machine and carried to the next
//! one by the person — through the relay's Noise session (`pairing`) or as a
//! recovery code (`key`) — and never to devpit.app. Each namespace — the
//! preferences, the project settings, the machines — is one blob, sealed
//! with that key (`seal`) under a name drawn from it. A round (`round`)
//! merges a namespace field by field, the later write winning by a clock
//! that never goes backwards (`clock`, `doc`), so every machine settles on
//! the same values whatever order they sync in.
//!
//! What syncs is an allowlist per namespace (`prefs`, `projects`); a vault
//! secret is on none of them. The design is written down in `docs/sync.md`.

pub mod blobs;
pub mod clock;
pub mod doc;
pub mod error;
pub mod files;
pub mod key;
pub mod machines;
pub mod pairing;
pub mod prefs;
pub mod projects;
pub mod round;
pub mod seal;
pub mod space;

pub use blobs::{Blobs, DevpitApp, Listed, MOST_BLOB};
pub use error::SyncError;
pub use key::SyncKey;
pub use round::{round, Outcome, State};
pub use space::Namespace;
