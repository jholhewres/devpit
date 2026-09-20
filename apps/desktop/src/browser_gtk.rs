//! Where a page actually lands on GTK, which is not where tauri is told to put
//! it.
//!
//! `Window::add_child` takes a position and a size, and on this platform both
//! are thrown away. The path is short enough to read end to end:
//!
//! 1. tauri builds a child webview into the window's default vertical box
//!    (`tauri-runtime-wry-2.11.4/src/lib.rs:5236`) — a `GtkBox`, chosen there
//!    because it is the only way to account for a menu bar.
//! 2. wry sees a `GtkBox` and calls `pack_start(…, expand, fill, 0)`
//!    (`wry-0.55.1/src/webkitgtk/mod.rs:596`), which *stacks* the page beside
//!    the app instead of placing it. The same function records
//!    `is_in_fixed_parent = false`.
//! 3. Every later `set_position` reaches `set_bounds`, which does nothing at
//!    all unless that flag is set or the window is an X11 child
//!    (`wry-0.55.1/src/webkitgtk/mod.rs:853-875`). Under Wayland neither holds.
//!
//! So the page drew below the app, at half its height, and `browser_place`
//! could not move it — the call succeeded and changed nothing.
//!
//! # The shape this window is rebuilt into
//!
//! ```text
//! vbox ── GtkOverlay ─┬─ the window's own webview   (main child: GTK sizes it)
//!                     └─ GtkFixed                   (overlay child, fills)
//!                          └─ a page per browser pane  (placed by this file)
//! ```
//!
//! Two containers, because the two jobs are opposites and one container cannot
//! do both:
//!
//! **The app's webview must be sized by GTK, once per pass.** It was a
//! `GtkFixed` child and that meant two allocations every pass — the fixed gave
//! it the size it asked for, then this file re-allocated it to the real one.
//! An allocation on a `WebKitWebView` is a full document relayout, and for
//! this webview that is devpit's entire interface; dragging the window edge
//! did it twice a frame and the app strobed. As an overlay's main child it is
//! allocated once, by `GtkBin`, and this file never touches it.
//!
//! **A page must be sized by this file, against GTK's wishes.** It has to be
//! exactly its pane's rectangle — including *narrower* than the page would
//! choose, which is what a page-width preset is. Neither alignment nor
//! `set_size_request` can do that: a size request smaller than a widget's
//! natural size is ignored, because `gtk_widget_get_preferred_width` returns
//! the larger of the two. Measured, not assumed — asking for 375 left the page
//! at 766, the width of its pane. So pages live in a `GtkFixed`, where
//! `size_allocate` is the last word.
//!
//! `GtkOverlay`'s own `get-child-position` signal would have placed them
//! without the second container, and gtk-rs 0.18 does not bind it
//! (`gtk/src/auto/overlay.rs:345`, "Unsupported or ignored types: Out
//! allocation: Gdk.Rectangle").

use std::cell::RefCell;

use gtk::prelude::*;

use crate::browser::Where;

/* Every page this window is holding, and the box its pane measured.

A thread local rather than managed state: these are GTK widgets, which are not
`Send`, and every function here runs inside `with_webview` — which is to say on
the one thread GTK allows. */
thread_local! {
    static SPOTS: RefCell<Vec<(gtk::Widget, Where)>> = const { RefCell::new(Vec::new()) };
}

/// The `GtkFixed` the pages live in, reached through the overlay.
fn fixed_in(vbox: &gtk::Box) -> Option<gtk::Fixed> {
    vbox.children()
        .into_iter()
        .find_map(|child| child.downcast::<gtk::Overlay>().ok())
        .and_then(|overlay| {
            overlay
                .children()
                .into_iter()
                .find_map(|child| child.downcast::<gtk::Fixed>().ok())
        })
}

/// Gives a widget an allocation, and only when it is not the one it has.
///
/// An allocation on a `WebKitWebView` lays the document out again, so handing
/// out one it already has is a relayout for nothing.
fn settled(widget: &gtk::Widget, want: gtk::Allocation) {
    let now = widget.allocation();
    if now.x() == want.x()
        && now.y() == want.y()
        && now.width() == want.width()
        && now.height() == want.height()
    {
        return;
    }
    widget.size_allocate(&want);
}

/// Writes down where a page belongs, and forgets pages that have been closed.
///
/// Nothing tells this module when a webview dies: `browser_close` closes it
/// through tauri, wry destroys the widget, and the clone held here would keep
/// a destroyed object alive — which GTK complains about on every layout. A
/// widget with no parent is one nobody is showing.
fn remember(page: &gtk::Widget, at: Where) {
    SPOTS.with(|spots| {
        let mut spots = spots.borrow_mut();
        spots.retain(|(held, _)| held.parent().is_some() || held == page);
        match spots.iter_mut().find(|(held, _)| held == page) {
            Some((_, was)) => *was = at,
            None => spots.push((page.clone(), at)),
        }
    });
}

/// Puts a page where its pane says, now, rather than waiting to be asked.
///
/// **Nothing reliably asks.** `queue_resize` marks the container and the next
/// layout pass allocates it — but GTK runs that pass only when something
/// actually changed size, and adding a child to a `GtkFixed` does not change
/// the fixed. Traced: the handler fired once, before the first page existed,
/// and never again. The page kept the allocation of a widget that has never
/// had one, `(-1, -1, 1, 1)`, and `browser_where` reported x = -1 against a
/// pane at x = 253.
///
/// **The allocation is the window's coordinates; the `move_` is the sheet's.**
/// A GTK3 allocation is in the toplevel's space, and `gtk_fixed_size_allocate`
/// says so itself by computing `child->x + allocation->x` — adding the sheet's
/// own origin to a position that was given relative to it. Taking that origin
/// off the `move_` and leaving it on the allocation is what makes the two
/// agree about where the page goes rather than fight over it.
///
/// The sheet has sat at the origin every time this has been measured, so the
/// two spaces have so far been the same space. The subtraction is here for the
/// day that stops being true, not for a bug it fixed.
fn lay(fixed: &gtk::Fixed, page: &gtk::Widget, at: Where) {
    let room = fixed.allocation();
    fixed.move_(page, at.x as i32 - room.x(), at.y as i32 - room.y());
    settled(
        page,
        gtk::Allocation::new(at.x as i32, at.y as i32, at.width as i32, at.height as i32),
    );
}

/// Rebuilds the window around a `GtkOverlay`, once, before any page opens.
///
/// The window's own webview becomes the overlay's main child, which is a
/// `GtkBin`'s child: GTK allocates it the whole area on every layout without
/// being asked and without being re-allocated afterwards.
pub fn settle(window: &tauri::WebviewWindow) {
    let handle = window.clone();
    let _ = window.with_webview(move |platform| {
        let app: gtk::Widget = platform.inner().upcast();
        let Ok(vbox) = handle.default_vbox() else {
            return;
        };
        if fixed_in(&vbox).is_some() {
            return;
        }
        let overlay = gtk::Overlay::new();
        /* The widget survives being unparented because this holds a reference
        to it — `gtk_container_remove` drops the container's, and the last one
        going would destroy the app's own webview. */
        vbox.remove(&app);
        overlay.add(&app);

        /* The sheet the pages are placed on, over the app and filling it.
        `Fill` is the one alignment `GtkOverlay` handles without its unbound
        signal: it hands the child the whole area rather than the size the
        child asks for.

        **And it must let input through.** A `GtkFixed` has no window of its
        own, which is what this file used to say made it harmless — but
        `GtkOverlay` gives every overlay child a `GdkWindow` of its own so it
        can stack it, and that window covers the whole app and swallows every
        click. The app went completely dead: a picture of itself, nothing
        clickable anywhere. `pass_through` empties that window's claim on
        input so it reaches the app below, while the pages — which have
        windows of their own, stacked above it — keep theirs. */
        let fixed = gtk::Fixed::new();
        fixed.set_halign(gtk::Align::Fill);
        fixed.set_valign(gtk::Align::Fill);
        overlay.add_overlay(&fixed);
        overlay.set_overlay_pass_through(&fixed, true);

        /* The one thing only the container can see: the window being resized.
        The app is not touched here — it is the overlay's main child and GTK
        has already given it the room. */
        fixed.connect_size_allocate(|fixed, _| {
            let showing = SPOTS.with(|spots| {
                let mut spots = spots.borrow_mut();
                spots.retain(|(held, _)| held.parent().is_some());
                spots.clone()
            });
            for (page, at) in showing {
                lay(fixed, &page, at);
            }
        });

        vbox.pack_start(&overlay, true, true, 0);
        overlay.show_all();
    });
}

/// Takes a page wry has just packed into the window and puts it where it goes.
///
/// Called straight after `add_child`, which is the only moment the widget
/// exists and is in the wrong place.
pub fn hold(window: &tauri::Window, webview: &tauri::Webview, place: Where) {
    let handle = window.clone();
    let _ = webview.with_webview(move |platform| {
        let page: gtk::Widget = platform.inner().upcast();
        let Ok(vbox) = handle.default_vbox() else {
            return;
        };
        let Some(fixed) = fixed_in(&vbox) else {
            return;
        };
        let already = page
            .parent()
            .is_some_and(|parent| parent.downcast_ref::<gtk::Fixed>() == Some(&fixed));
        if !already {
            /* Out of the box wry packed it into, and onto the sheet. The
            coordinates are the sheet's own; `lay` sets them properly a moment
            later, once the sheet's origin is taken off. */
            if let Some(container) = page
                .parent()
                .and_then(|parent| parent.downcast::<gtk::Container>().ok())
            {
                container.remove(&page);
            }
            fixed.put(&page, 0, 0);
        }
        remember(&page, place);
        /* Shown before it is allocated: an unrealised widget takes an
        allocation and does nothing with it. */
        page.show_all();
        lay(&fixed, &page, place);
    });
}

/// Moves a page that is already in, because its pane moved or the window did.
pub fn place(webview: &tauri::Webview, place: Where) {
    let _ = webview.with_webview(move |platform| {
        let page: gtk::Widget = platform.inner().upcast();
        remember(&page, place);
        let Some(fixed) = page
            .parent()
            .and_then(|parent| parent.downcast::<gtk::Fixed>().ok())
        else {
            return;
        };
        lay(&fixed, &page, place);
    });
}

/// Where a page ended up, measured off the window rather than remembered.
///
/// The point is to be able to disagree with this file. wry reports a child
/// webview's position as `(0, 0)` on GTK whatever it is — it fills in only the
/// size (`wry-0.55.1/src/webkitgtk/mod.rs:847`) — so asking tauri where a page
/// is gets an answer that is the same before and after any fix. This asks GTK
/// to translate the page's own origin into the window's coordinates, which is
/// the number a pane can be compared against.
pub fn spot(webview: &tauri::Webview) -> Option<Where> {
    let (said, hear) = std::sync::mpsc::channel();
    webview
        .with_webview(move |platform| {
            let page: gtk::Widget = platform.inner().upcast();
            let found = page.toplevel().and_then(|top| {
                let room = page.allocation();
                page.translate_coordinates(&top, 0, 0).map(|(x, y)| Where {
                    x: f64::from(x),
                    y: f64::from(y),
                    width: f64::from(room.width()),
                    height: f64::from(room.height()),
                })
            });
            let _ = said.send(found);
        })
        .ok()?;
    /* Bounded, because this crosses to the GTK thread and a window that is
    busy or gone would otherwise hold the command open forever. */
    hear.recv_timeout(std::time::Duration::from_secs(2))
        .ok()
        .flatten()
}
