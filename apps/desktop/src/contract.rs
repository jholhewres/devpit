//! The command list, and the TypeScript generated from it.
//!
//! Separate from the window: which commands exist is not a question about how
//! the window is drawn, and keeping the list here means adding one does not
//! touch the file that opens the app.

use tauri_specta::Builder;

/// Where the generated TypeScript lands.
///
/// Anchored to the manifest directory rather than the working directory:
/// `tauri dev` and a bare `./devpit-desktop` run from different places, and a
/// relative path would quietly write the contract somewhere else.
pub const BINDINGS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../web/src/gen/bindings.ts");

/// The commands whose types are generated into TypeScript.
///
/// Separate from the invoke handler on purpose: one command cannot be in the
/// contract, because specta cannot describe the channel it streams over.
pub fn contract() -> Builder<tauri::Wry> {
    list::loaded(Builder::<tauri::Wry>::new())
}

#[path = "contract_list.rs"]
mod list;

#[cfg(test)]
#[path = "contract_tests.rs"]
mod tests;
