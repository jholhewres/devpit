//! A transparent window that repaints itself after it changes state.
//!
//! The frame draws rounded corners, which needs a transparent window. On Linux
//! a transparent WebKitGTK window that was maximized or restored could keep
//! its last frame on screen: the app at its old size over a leftover copy of
//! itself, until the next resize. GTK had laid the window out again but never
//! asked it to draw. So it is asked, right after the change and once more when
//! the compositor's animation is over.

use gtk::prelude::*;

/// How long a compositor's maximize animation is given before the last redraw.
const AFTER_ANIMATION: std::time::Duration = std::time::Duration::from_millis(180);

pub fn repaint_on_state_change(window: &tauri::WebviewWindow) {
    let Ok(gtk_window) = window.gtk_window() else {
        return;
    };
    gtk_window.connect_window_state_event(|window, _| {
        let now = window.clone();
        gtk::glib::idle_add_local_once(move || redraw(&now));
        let later = window.clone();
        gtk::glib::timeout_add_local_once(AFTER_ANIMATION, move || redraw(&later));
        gtk::glib::Propagation::Proceed
    });
}

fn redraw(window: &gtk::ApplicationWindow) {
    window.queue_resize();
    window.queue_draw();
}
