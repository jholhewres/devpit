use super::{ends_devpit, quitting, readable, Quit};

/// Quit goes through the window while there is one, so it asks what closing
/// asks; with none it must still end the process, not do nothing.
#[test]
fn quit_closes_the_window_or_ends_devpit_without_one() {
    assert_eq!(quitting(true), Quit::CloseTheWindow);
    assert_eq!(quitting(false), Quit::Exit);
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
