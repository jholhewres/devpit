//! The display server devpit's windows are drawn on, on Linux.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum DisplayBackend {
    Wayland,
    X11,
}

/// Why this start is on the display it is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DisplayWhy {
    /// `DEVPIT_GDK_BACKEND` named it.
    Asked,
    /// Outside the AppImage, `GDK_BACKEND` was already set by whoever started devpit.
    Inherited,
    /// The session has no Wayland compositor.
    NoWayland,
    /// GTK would abort on Wayland here: its xsettings schema lacks `antialiasing`.
    KnownCrash,
    /// The last start on Wayland of this version never got its window up.
    FailedBefore,
    /// The person chose it in Settings.
    Chosen,
    /// Wayland where there is one, X11 if GTK cannot reach it.
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Display {
    /// Whether there is a choice to make: Linux only.
    pub offered: bool,
    /// What GTK opened for this start; null before it did, or off Linux.
    pub running: Option<DisplayBackend>,
    /// The person's choice for the next start; null is automatic.
    pub chosen: Option<DisplayBackend>,
    pub why: DisplayWhy,
}
