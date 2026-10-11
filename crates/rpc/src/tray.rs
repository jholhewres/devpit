//! The tray: whether this desktop shows one, and whether devpit stays in it.

use serde::{Deserialize, Serialize};
use specta::Type;

/// What closing the window does, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Tray {
    /// Whether devpit's icon is in this desktop's tray. Without one, closing
    /// the window quits: a hidden window with no icon has no way back.
    pub shown: bool,
    /// Whether closing the window leaves devpit running in the tray.
    pub keep: bool,
}
