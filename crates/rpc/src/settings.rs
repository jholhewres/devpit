//! What the person chose, and what this build can offer them.
//!
//! One object rather than a command per switch: the first run reads all of it
//! once and writes what changed, and a screen that has to make four round
//! trips to draw itself is a screen that draws itself four times.

use serde::{Deserialize, Serialize};
use specta::Type;

/// The appearance the window uses.
///
/// An enum and not a string, so a build that does not ship a theme cannot be
/// asked for one. `System` is a third setting, not a third palette: it borrows
/// whichever of the two the machine is already using.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    System,
    Light,
    #[default]
    Dark,
}

impl Theme {
    /// What a stored value means. An unknown name reads as the default rather
    /// than failing the whole settings read for one bad row.
    pub fn parse(stored: &str) -> Self {
        match stored {
            "system" => Self::System,
            "light" => Self::Light,
            _ => Self::Dark,
        }
    }

    pub fn stored(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Three states, not two. `null` is "never asked", which is what makes the
    /// first run ask instead of assuming a silent yes or a silent no.
    pub telemetry: Option<bool>,
    pub theme: Theme,
    /// The account this install is signed in as, when it is.
    pub account: Option<String>,
    /// Unix seconds, or null while the first run has not been finished.
    pub onboarded_at: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::Theme;

    #[test]
    fn a_stored_theme_comes_back_as_itself() {
        for theme in [Theme::System, Theme::Light, Theme::Dark] {
            assert_eq!(Theme::parse(theme.stored()), theme);
        }
    }

    #[test]
    fn a_name_this_build_does_not_have_reads_as_dark() {
        // One bad row must not fail the whole settings read.
        assert_eq!(Theme::parse("solarized"), Theme::Dark);
    }
}
