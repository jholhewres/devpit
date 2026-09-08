//! Core of quockpit.
//!
//! All product logic lives here and in the other `crates/`. The desktop shell,
//! the HTTP server and the CLI are thin adapters over it.
//!
//! The rule that holds the design together — checked by `cargo xtask check` —
//! is that nothing under `crates/` imports `tauri`. That is what keeps the
//! shell choice reversible: if WebKitGTK on Linux rules Tauri out, swapping
//! shells costs days instead of months.

pub mod bus;
pub mod store;
pub mod tree;

pub use bus::{Bus, Event, Severity};
pub use store::{
    preference, CardRow, ColumnRow, NoteRow, ProjectRow, RunRow, StepRow, Store, StoreError,
    DEFAULT_COLUMNS,
};
pub use tree::TreeError;
