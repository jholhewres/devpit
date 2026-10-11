use super::{closing, ends_devpit, quitting, readable, Close, Quit};

/// Quit goes through the window while there is one, so it asks what closing
/// asks; with none it must still end the process, not do nothing.
#[test]
fn quit_closes_the_window_or_ends_devpit_without_one() {
    assert_eq!(quitting(true), Quit::CloseTheWindow);
    assert_eq!(quitting(false), Quit::Exit);
}

/// Closing hides only when the person wants it and a tray shows the icon to
/// come back by; Quit's own close always ends devpit.
#[test]
fn closing_hides_only_into_a_tray_that_shows() {
    assert_eq!(closing(true, true, false), Close::Hide);
    // Kept off: closing quits, as before there was a tray to keep it in.
    assert_eq!(closing(false, true, false), Close::Quit);
    // No icon to bring it back (GNOME without AppIndicator, a tray that
    // failed to build): a hidden window would be a devpit nobody can reach.
    assert_eq!(closing(true, false, false), Close::Quit);
    assert_eq!(closing(false, false, false), Close::Quit);
    // Quit closes the window too, and that close must not hide it.
    assert_eq!(closing(true, true, true), Close::Quit);
}

/// The main window going ends devpit; the island or a menu closing does not.
#[test]
fn only_the_main_window_going_ends_devpit() {
    assert!(ends_devpit("main"));
    for label in [crate::island::ISLAND, crate::browser_menu::MENU] {
        assert!(!ends_devpit(label), "{label}");
    }
}

#[test]
fn a_shortcut_is_read_as_the_plugin_spells_it() {
    for keys in [
        "CommandOrControl+Shift+D",
        "Alt+Space",
        "Ctrl+Alt+P",
        "Super+D",
    ] {
        assert!(readable(keys), "{keys}");
    }
    for keys in ["", "Shift+", "Hyper+Banana", "D+D+D+"] {
        assert!(!readable(keys), "{keys}");
    }
}
