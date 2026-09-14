//! How wide the two side panels are.
//!
//! Its own pair of commands rather than two more arguments on `settings_write`,
//! which already takes five nullable ones: a sixth and a seventh would make
//! every caller count positions to say one thing.
//!
//! Clamped here as well as in the window. The window clamps so a drag stops at
//! the edge; this clamps because a number that reached disk from an older
//! build, or from somebody editing the file, must not be able to produce a
//! panel nobody can drag back.

use devpit_core::store::preference;
use devpit_rpc::RpcError;
use serde::{Deserialize, Serialize};
use specta::Type;

/// The limits, which are the window's limits said again.
const LEAST: Widths = Widths {
    sidebar: 180,
    files: 240,
};
const MOST: Widths = Widths {
    sidebar: 480,
    files: 640,
};
const WIDE: Widths = Widths {
    sidebar: 252,
    files: 340,
};

/// A floor at the ceiling would be a divider that does nothing. Checked at
/// compile time rather than by a test, because the thing being guarded is the
/// constants themselves and a test would only restate them.
const _: () = assert!(LEAST.sidebar < WIDE.sidebar && WIDE.sidebar < MOST.sidebar);
const _: () = assert!(LEAST.files < WIDE.files && WIDE.files < MOST.files);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Widths {
    pub sidebar: u32,
    pub files: u32,
}

impl Widths {
    fn held(self) -> Self {
        Self {
            sidebar: self.sidebar.clamp(LEAST.sidebar, MOST.sidebar),
            files: self.files.clamp(LEAST.files, MOST.files),
        }
    }
}

fn read(store: &devpit_core::Store, key: &str, fallback: u32) -> u32 {
    store
        .preference(key)
        .ok()
        .flatten()
        .and_then(|said| said.parse().ok())
        .unwrap_or(fallback)
}

/// `panel.widths` — how wide the panels were left.
#[tauri::command]
#[specta::specta]
pub fn panel_widths() -> Result<Widths, RpcError> {
    let store = crate::projects::store()?;
    Ok(Widths {
        sidebar: read(&store, preference::SIDEBAR_WIDTH, WIDE.sidebar),
        files: read(&store, preference::FILES_WIDTH, WIDE.files),
    }
    .held())
}

/// `panel.widths_write` — remembers where the divider was let go.
///
/// Written on the drop and never during the drag: a preference row rewritten
/// on every pointer move is a disk write per frame for a number nobody reads
/// until the next launch.
#[tauri::command]
#[specta::specta]
pub fn panel_widths_write(sidebar: u32, files: u32) -> Result<Widths, RpcError> {
    let held = Widths { sidebar, files }.held();
    let store = crate::projects::store()?;
    store.set_preference(preference::SIDEBAR_WIDTH, &held.sidebar.to_string())?;
    store.set_preference(preference::FILES_WIDTH, &held.files.to_string())?;
    Ok(held)
}

#[cfg(test)]
#[path = "panels_tests.rs"]
mod tests;
