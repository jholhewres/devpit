//! Which password, and whether the machine could even be asked.

use super::*;

/// Each browser writes under its own attribute. Getting this wrong asks the
/// keyring for a password that exists and belongs to another browser, which
/// decrypts nothing and reads as a corrupt store.
#[test]
fn each_browser_is_asked_for_under_its_own_name() {
    assert_eq!(keyring_name("Google Chrome · Default"), "chrome");
    assert_eq!(keyring_name("Chromium · Default"), "chromium");
    assert_eq!(keyring_name("Brave · Default"), "brave");
    assert_eq!(keyring_name("Microsoft Edge · Default"), "microsoft-edge");
    assert_eq!(keyring_name("Vivaldi · Default"), "vivaldi");
    /* Something new falls back to chrome's, which is what a Chromium fork
    that has not been renamed actually uses. */
    assert_eq!(keyring_name("Some New Fork"), "chrome");
}

/// The case this machine is in: gnome-keyring is running and secret-tool is
/// not installed. Saying so is the whole point — a profile encrypted under a
/// keyring secret refuses every value at once, and that is indistinguishable
/// from a corrupt store unless the screen can name the real reason.
#[test]
fn a_machine_that_cannot_ask_its_keyring_says_so_with_what_to_install() {
    let nowhere: [&str; 1] = ["/nonexistent-for-this-test"];
    let (password, found) = password_for(&nowhere, "Google Chrome");
    assert_eq!(password, crate::chromium::Password::Fallback);
    let Found::Unreachable { why } = found else {
        panic!("expected Unreachable, got {found:?}");
    };
    assert!(why.contains("libsecret-tools"), "{why}");
    assert!(why.contains("keyring"), "{why}");
}

/// The three outcomes are three different things to do about them.
#[test]
fn the_three_answers_are_told_apart() {
    let unreachable = Found::Unreachable {
        why: "x".to_owned(),
    };
    assert_ne!(Found::Keyring, Found::Fallback);
    assert_ne!(Found::Keyring, unreachable);
    assert_ne!(Found::Fallback, unreachable);
}

/// And the directories are the fixed ones, never $PATH: this runs a program to
/// obtain a decryption key.
#[test]
fn only_directories_root_owns_are_searched() {
    for dir in TRUSTED {
        assert!(dir.starts_with('/'), "{dir} is relative");
    }
    assert!(TRUSTED.contains(&"/usr/bin"));
}
