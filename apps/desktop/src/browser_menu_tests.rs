//! Where the menu lands, and what may answer to its name.

use super::*;

fn rect(x: f64, y: f64, width: f64, height: f64) -> Where {
    Where {
        x,
        y,
        width,
        height,
    }
}

/// A dropdown hangs below the control and lines up with its right edge. The
/// panel is anchored right inside the window, so the window's right edge is
/// what has to meet the control's.
#[test]
fn the_menu_hangs_below_the_control_and_ends_where_it_ends() {
    let control = rect(900.0, 40.0, 26.0, 26.0);
    let (x, y) = under((0.0, 0.0), control);
    assert_eq!(
        x + WIDE,
        control.x + control.width,
        "the right edges differ"
    );
    assert!(
        y > control.y + control.height,
        "the menu is not below the control"
    );
}

/// The window's own place on the screen is added, or the menu opens over
/// whatever is at the desktop's origin instead of over devpit.
#[test]
fn the_window_being_somewhere_moves_the_menu_with_it() {
    let control = rect(900.0, 40.0, 26.0, 26.0);
    let (x, y) = under((0.0, 0.0), control);
    let (moved_x, moved_y) = under((300.0, 120.0), control);
    assert_eq!(moved_x - x, 300.0);
    assert_eq!(moved_y - y, 120.0);
}

/// The one that matters for the capability. `capabilities/default.json` grants
/// devpit's whole command surface to this exact label, so what must hold is
/// that no label devpit gives a *page* can ever be it.
#[test]
fn the_menu_and_a_page_cannot_be_confused_for_each_other() {
    let page = crate::browser::label_for("some-pane").expect("a pane label");
    assert_ne!(page, MENU);
    assert!(
        !page.starts_with(MENU),
        "{page} starts with the menu's label"
    );
    assert!(
        !MENU.starts_with(&page),
        "the menu's label starts with {page}"
    );
    /* And the menu is not something `browser_close` would agree to close. */
    assert!(!crate::browser::ours(MENU));
}

/// The label is exact and carries no separator, so there is no `devpit-menu:x`
/// for anything to be called. A prefix would have been a family of labels and
/// the capability grants by name.
#[test]
fn the_menu_label_names_one_window_and_not_a_family() {
    assert_eq!(MENU, "devpit-menu");
    assert!(!MENU.ends_with(':'));
}
