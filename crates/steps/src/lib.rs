//! Running a command step: your own command, as a column of the board.
//!
//! This is what makes "run the tests" and "deploy" lanes rather than things
//! you remember to do. The design decision that matters is in [`context`]:
//! **the context reaches the command only through the environment**, never
//! interpolated into the command string.

pub mod context;
mod descendants;
pub mod ended;
pub mod manifest;
pub mod runner;

pub use context::{Context, CONTEXT_KEYS};
pub use ended::{Ended, RunError};
pub use manifest::{validate, Manifest, ManifestError};
pub use runner::run;
