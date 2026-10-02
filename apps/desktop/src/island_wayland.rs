//! The island on a Wayland display.
//!
//! A Wayland window cannot place itself or stay above the others. Where the
//! compositor speaks the layer-shell protocol (KDE, Hyprland, Sway and most
//! wlroots ones), the island becomes an overlay layer anchored to the top,
//! over the panel and never taking the keyboard. GNOME does not speak it:
//! there the island is an ordinary window that refuses focus, wherever the
//! compositor puts it. `DEVPIT_ISLAND_LAYER=0` skips the layer everywhere.
//!
//! `libgtk-layer-shell` is opened at run time rather than linked: a machine
//! without it must still start devpit, and only the island is the poorer.
//!
//! The approach — the overlay layer, the input region on every map, the
//! first-frame remap — follows Coucou's (github.com/Louis-CFM/coucou, MIT);
//! the code is devpit's own.

use std::ffi::{c_char, c_int, c_void, CStr};
use std::sync::OnceLock;

use gtk::glib::translate::ToGlibPtr;
use gtk::prelude::*;

/// Whether this display is a Wayland one.
pub fn wayland() -> bool {
    gtk::gdk::Display::default()
        .is_some_and(|display| display.type_().name() == "GdkWaylandDisplay")
}

type Window = *mut gtk::ffi::GtkWindow;

/// The functions of `libgtk-layer-shell` the island needs.
struct LayerShell {
    is_supported: unsafe extern "C" fn() -> c_int,
    init_for_window: unsafe extern "C" fn(Window),
    set_namespace: unsafe extern "C" fn(Window, *const c_char),
    set_layer: unsafe extern "C" fn(Window, c_int),
    set_anchor: unsafe extern "C" fn(Window, c_int, c_int),
    set_exclusive_zone: unsafe extern "C" fn(Window, c_int),
    set_monitor: unsafe extern "C" fn(Window, *mut gtk::gdk::ffi::GdkMonitor),
    /// `set_keyboard_mode` from 0.6, `set_keyboard_interactivity` before.
    no_keyboard: unsafe extern "C" fn(Window, c_int),
}

// The pointers are functions in a library that stays loaded for the life of
// the process, and they are only called on GTK's thread.
unsafe impl Send for LayerShell {}
unsafe impl Sync for LayerShell {}

const LAYER_OVERLAY: c_int = 3;
const EDGE_TOP: c_int = 2;

fn layer_shell() -> Option<&'static LayerShell> {
    static LOADED: OnceLock<Option<LayerShell>> = OnceLock::new();
    LOADED
        .get_or_init(|| {
            if std::env::var("DEVPIT_ISLAND_LAYER").as_deref() == Ok("0") {
                return None;
            }
            // SAFETY: a plain dlopen of a library by its soname, and dlsym of
            // functions whose C signatures are the ones declared above.
            unsafe {
                let library = libc::dlopen(c"libgtk-layer-shell.so.0".as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL);
                if library.is_null() {
                    return None;
                }
                let found = |name: &CStr| -> Option<*mut c_void> {
                    let symbol = libc::dlsym(library, name.as_ptr());
                    (!symbol.is_null()).then_some(symbol)
                };
                let no_keyboard = found(c"gtk_layer_set_keyboard_mode").or_else(|| found(c"gtk_layer_set_keyboard_interactivity"))?;
                Some(LayerShell {
                    is_supported: std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> c_int>(found(c"gtk_layer_is_supported")?),
                    init_for_window: std::mem::transmute::<*mut c_void, unsafe extern "C" fn(Window)>(found(c"gtk_layer_init_for_window")?),
                    set_namespace: std::mem::transmute::<*mut c_void, unsafe extern "C" fn(Window, *const c_char)>(found(c"gtk_layer_set_namespace")?),
                    set_layer: std::mem::transmute::<*mut c_void, unsafe extern "C" fn(Window, c_int)>(found(c"gtk_layer_set_layer")?),
                    set_anchor: std::mem::transmute::<*mut c_void, unsafe extern "C" fn(Window, c_int, c_int)>(found(c"gtk_layer_set_anchor")?),
                    set_exclusive_zone: std::mem::transmute::<*mut c_void, unsafe extern "C" fn(Window, c_int)>(found(c"gtk_layer_set_exclusive_zone")?),
                    set_monitor: std::mem::transmute::<*mut c_void, unsafe extern "C" fn(Window, *mut gtk::gdk::ffi::GdkMonitor)>(found(c"gtk_layer_set_monitor")?),
                    no_keyboard: std::mem::transmute::<*mut c_void, unsafe extern "C" fn(Window, c_int)>(no_keyboard),
                })
            }
        })
        .as_ref()
}

/// Whether the island can be a layer here: the library is there and the
/// compositor speaks the protocol.
pub fn layered() -> bool {
    // SAFETY: takes nothing, asks the compositor's globals.
    layer_shell().is_some_and(|layer| unsafe { (layer.is_supported)() } != 0)
}

/// Makes the island's window a layer at the top of `screen` — the model GDK
/// names it by — before it is first shown. Answers whether it did.
pub fn layer(window: &gtk::ApplicationWindow, screen: Option<&str>) -> bool {
    let Some(layer) = layer_shell().filter(|_| layered()) else {
        return false;
    };
    // A layer is made from a window that was never on screen.
    if window.is_realized() {
        window.unrealize();
    }
    let raw: Window = window.upcast_ref::<gtk::Window>().to_glib_none().0;
    let monitor = screen.and_then(|name| {
        let display = gtk::gdk::Display::default()?;
        (0..display.n_monitors())
            .filter_map(|at| display.monitor(at))
            .find(|monitor| monitor.model().as_deref() == Some(name))
    });
    // SAFETY: `raw` is the live GtkWindow above, on GTK's thread; the monitor
    // pointer, when there is one, is borrowed for the call.
    unsafe {
        (layer.init_for_window)(raw);
        (layer.set_namespace)(raw, c"devpit-island".as_ptr());
        (layer.set_layer)(raw, LAYER_OVERLAY);
        (layer.set_anchor)(raw, EDGE_TOP, 1);
        // Over the panel, and pushing nothing aside.
        (layer.set_exclusive_zone)(raw, -1);
        (layer.no_keyboard)(raw, 0);
        if let Some(monitor) = &monitor {
            (layer.set_monitor)(raw, monitor.to_glib_none().0);
        }
    }
    true
}

/// What a Wayland island window needs either way: no empty title bar, which
/// the toolkit adds to an undecorated window and which would redraw over the
/// input region; the region again on every map, which resets it; and, as a
/// layer, one hide and show after the first frame, which WebKitGTK does not
/// paint on a new layer surface.
pub fn settle(window: &tauri::WebviewWindow, layered: bool, shaped: fn(&tauri::WebviewWindow)) {
    let Ok(gtk_window) = window.gtk_window() else {
        return;
    };
    gtk_window.set_titlebar(None::<&gtk::Widget>);
    let again = window.clone();
    gtk_window.connect_map_event(move |_, _| {
        shaped(&again);
        gtk::glib::Propagation::Proceed
    });
    if layered {
        let first = gtk_window.clone();
        gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(250), move || {
            first.hide();
            first.show();
        });
    }
}
