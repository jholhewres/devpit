//! Core of devpit.
//!
//! All product logic lives here and in the other `crates/`. The desktop shell,
//! the HTTP server and the CLI are thin adapters over it.
//!
//! The rule that holds the design together — checked by `cargo xtask check` —
//! is that nothing under `crates/` imports `tauri`. That is what keeps the
//! shell choice reversible: if WebKitGTK on Linux rules Tauri out, swapping
//! shells costs days instead of months.

pub mod bus;
pub mod data_files;
pub mod home;
pub mod paths;
#[cfg(test)]
mod paths_guard_tests;
#[cfg(test)]
mod paths_tests;
pub mod store;
pub mod tree;
#[cfg(test)]
mod tree_tests;

pub use bus::{Bus, Event, Severity};
pub use store::{
    limits, preference, AttachmentRow, CardRow, ColumnRow, CommentRow, NoticeRow, PaneLayoutWrite,
    ProjectRow, RunRow, SessionLink, StepRow, Store, StoreError, DEFAULT_COLUMNS, DEV_ROOT,
    RELEASE_ROOT, ROOT_NAME,
};
pub use tree::TreeError;
