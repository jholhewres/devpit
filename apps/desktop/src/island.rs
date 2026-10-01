//! The island: a small window at the top of the screen, above everything,
//! that shows what every agent is doing while devpit is somewhere behind.
//!
//! Inspired by Coucou (github.com/Louis-CFM/coucou, MIT): the window, the
//! input shape and the motion are its ideas; the character and the words are
//! devpit's own.
//!
//! **One window, as big as the largest view, mostly transparent.** The shape
//! people see is drawn inside it and animated there; resizing a window per
//! frame is what makes an animation stutter. What keeps the transparent rest
//! from swallowing clicks differs by platform:
//!
//! - Linux: an X input shape around the drawn island. GTK will not shrink a
//!   webview's window below 200 px, and a shape is what makes the rest glass.
//! - Elsewhere: the window ignores the mouse unless the cursor is over the
//!   island, decided by a poll that only runs while the island is shown.
//!
//! **Linux needs X11.** A Wayland client may neither place itself nor stay on
//! top, so on a Wayland display the island does not open at all — the
//! AppImage runs through XWayland already, a `.deb` on Wayland does not.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use devpit_rpc::{ErrorCode, FileContents, IslandChange, IslandNow, IslandRect, RpcError};
use serde::Serialize;
use tauri::{Emitter as _, Manager as _};

/// The label the island window answers to: exact, like the menu's, so no
/// capability pattern that reaches it can reach a browser page.
pub const ISLAND: &str = "devpit-island";

/// The window, in logical pixels: the largest view and the room its shadow
/// and spring overshoot need.
const WIDE: f64 = 760.0;
const TALL: f64 = 380.0;

/// Room around the drawn island that still takes the mouse, so a click on its
/// edge is never lost to the window underneath.
const MARGIN: f64 = 10.0;

/// Where the mouse may land, and whether anything is drawn there.
#[derive(Default)]
struct Shape {
    rect: IslandRect,
    shown: bool,
}

static SHAPE: Mutex<Shape> = Mutex::new(Shape {
    rect: IslandRect {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    },
    shown: false,
});
static POLLING: AtomicBool = AtomicBool::new(false);

/// Opens the island if the person wants it and the screen allows it, or
/// closes it when they do not.
pub fn apply(app: &tauri::AppHandle) {
    let wanted = crate::settings::settings_read_now()
        .map(|settings| settings.island.unwrap_or(true))
        .unwrap_or(false);
    let open = app.get_webview_window(ISLAND);
    match (wanted && placeable(), open) {
        (true, None) => {
            if let Err(err) = made(app) {
                eprintln!("the island did not open: {err}");
            }
        }
        (false, Some(window)) => {
            let _ = window.close();
        }
        _ => {}
    }
}

/// Whether a window can put itself at the top of this screen and stay there.
#[cfg(target_os = "linux")]
fn placeable() -> bool {
    use gtk::prelude::*;
    gtk::gdk::Display::default().is_some_and(|display| display.type_().name() == "GdkX11Display")
}

#[cfg(not(target_os = "linux"))]
fn placeable() -> bool {
    true
}

fn made(app: &tauri::AppHandle) -> Result<tauri::WebviewWindow, RpcError> {
    let built =
        tauri::WebviewWindowBuilder::new(app, ISLAND, tauri::WebviewUrl::App("index.html".into()))
            // Read before any of our code runs, the way the menu is told apart.
            .initialization_script("window.__DEVPIT_ISLAND__ = true")
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false)
            .focused(false)
            // Clicking it must not take the keyboard from what somebody is
            // typing in: on Windows this is WS_EX_NOACTIVATE, on macOS a
            // window that never becomes key.
            .focusable(false)
            // On every desktop and Space, the way the top bar is.
            .visible_on_all_workspaces(true)
            .visible(false)
            .inner_size(WIDE, TALL)
            .title("devpit island")
            .build()
            .map_err(|err| RpcError::internal(format!("the island would not open: {err}")))?;
    placed(&built);
    quiet(&built);
    let moved = built.clone();
    built.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Moved(_)) {
            dropped(&moved);
        }
    });
    let _ = built.show();
    shaped(&built);
    watch(app.clone());
    Ok(built)
}

/// At the top, in the middle of a screen: the one it was last dragged to
/// while that one is connected, else the one devpit's window is on, else the
/// primary.
fn placed(window: &tauri::WebviewWindow) {
    let chosen = remembered().and_then(|name| {
        window
            .available_monitors()
            .ok()?
            .into_iter()
            .find(|screen| screen.name() == Some(&name))
    });
    let main = || {
        window
            .app_handle()
            .get_webview_window("main")
            .and_then(|main| main.current_monitor().ok().flatten())
    };
    let Some(screen) = chosen
        .or_else(main)
        .or_else(|| window.primary_monitor().ok().flatten())
    else {
        return;
    };
    set_on(window, &screen);
}

/// Puts the island at the top middle of one screen.
fn set_on(window: &tauri::WebviewWindow, screen: &tauri::Monitor) {
    let wide = (WIDE * screen.scale_factor()).round() as i32;
    let at = screen.position();
    let x = at.x + (screen.size().width as i32 - wide) / 2;
    // Under the desktop's own top bar, not behind it: the work area starts
    // where the panels end. Centred on the whole screen all the same, the way
    // the bar's clock is, so a dock on one side does not push it over.
    let y = screen.work_area().position.y.max(at.y);
    PLACING.store(true, Ordering::SeqCst);
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(400));
        PLACING.store(false, Ordering::SeqCst);
    });
}

fn remembered() -> Option<String> {
    let store = devpit_core::Store::open_default().ok()?;
    store
        .preference(devpit_core::preference::ISLAND_SCREEN)
        .ok()
        .flatten()
        .filter(|name| !name.is_empty())
}

/// Set while the island is being put somewhere, so its own move is not read
/// as somebody dragging it.
static PLACING: AtomicBool = AtomicBool::new(false);
/// Counts moves, so only the last one of a drag settles it.
static MOVES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Somebody dropped the island somewhere: once it stops moving, it goes to
/// the top middle of the screen it was dropped on, and stays on that screen.
fn dropped(window: &tauri::WebviewWindow) {
    if PLACING.load(Ordering::SeqCst) {
        return;
    }
    let this = MOVES.fetch_add(1, Ordering::SeqCst) + 1;
    let window = window.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(350));
        if MOVES.load(Ordering::SeqCst) != this || PLACING.load(Ordering::SeqCst) {
            return;
        }
        let (Ok(at), Ok(size)) = (window.outer_position(), window.outer_size()) else {
            return;
        };
        let middle = (
            f64::from(at.x) + f64::from(size.width) / 2.0,
            f64::from(at.y) + f64::from(size.height) / 4.0,
        );
        let Ok(Some(screen)) = window.monitor_from_point(middle.0, middle.1) else {
            return;
        };
        if let (Some(name), Ok(store)) = (screen.name(), devpit_core::Store::open_default()) {
            let _ = store.set_preference(devpit_core::preference::ISLAND_SCREEN, name);
        }
        set_on(&window, &screen);
    });
}

/// Never takes focus, never listed with the windows, always above them.
#[cfg(target_os = "linux")]
fn quiet(window: &tauri::WebviewWindow) {
    use gtk::prelude::*;
    let Ok(gtk_window) = window.gtk_window() else {
        return;
    };
    gtk_window.set_accept_focus(false);
    gtk_window.set_focus_on_map(false);
    gtk_window.set_skip_taskbar_hint(true);
    gtk_window.set_skip_pager_hint(true);
    gtk_window.set_keep_above(true);
}

#[cfg(not(target_os = "linux"))]
fn quiet(window: &tauri::WebviewWindow) {
    let _ = window.set_ignore_cursor_events(true);
}

/// The part of the window that takes the mouse, in the window's own pixels.
///
/// The margin only around a drawn island: hidden, what takes the mouse is the
/// wake strip, and a margin there would be a band across the top of every
/// window under it that no longer answers clicks.
pub(crate) fn taking(rect: IslandRect, shown: bool) -> IslandRect {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return IslandRect::default();
    }
    let margin = if shown { MARGIN } else { 0.0 };
    let x = (rect.x - margin).max(0.0).floor();
    let y = (rect.y - margin).max(0.0).floor();
    IslandRect {
        x,
        y,
        width: (rect.x + rect.width + margin - x).ceil(),
        height: (rect.y + rect.height + margin - y).ceil(),
    }
}

/// Whether a point, in the window's pixels, lands on what takes the mouse.
pub(crate) fn lands(rect: IslandRect, x: f64, y: f64) -> bool {
    rect.width > 0.0
        && x >= rect.x
        && y >= rect.y
        && x < rect.x + rect.width
        && y < rect.y + rect.height
}

#[cfg(target_os = "linux")]
fn shaped(window: &tauri::WebviewWindow) {
    let rect = SHAPE
        .lock()
        .map(|shape| taking(shape.rect, shape.shown))
        .unwrap_or_default();
    let window = window.clone();
    let _ = window.clone().run_on_main_thread(move || {
        use gtk::cairo::{RectangleInt, Region};
        use gtk::prelude::*;
        let Ok(gtk_window) = window.gtk_window() else {
            return;
        };
        let Some(surface) = gtk_window.window() else {
            return;
        };
        // An empty region means "no shape", which is the whole window: one
        // pixel in the corner is how nothing takes the mouse.
        let region = if rect.width > 0.0 {
            RectangleInt::new(
                rect.x as i32,
                rect.y as i32,
                rect.width as i32,
                rect.height as i32,
            )
        } else {
            RectangleInt::new(0, 0, 1, 1)
        };
        surface.input_shape_combine_region(&Region::create_rectangle(&region), 0, 0);
    });
}

#[cfg(not(target_os = "linux"))]
fn shaped(_window: &tauri::WebviewWindow) {}

/// Where the cursor is, in the island window's pixels, for the eyes.
#[derive(Debug, Clone, Copy, Serialize, specta::Type)]
pub struct Cursor {
    pub x: f64,
    pub y: f64,
}

/// Follows the cursor for the island, on its own thread: the eyes everywhere,
/// and outside Linux whether the window takes the mouse at all.
fn watch(app: tauri::AppHandle) {
    if POLLING.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        let mut last = (f64::NAN, f64::NAN);
        let mut took = false;
        loop {
            let Some(window) = app.get_webview_window(ISLAND) else {
                POLLING.store(false, Ordering::SeqCst);
                return;
            };
            let (shown, rect) = SHAPE
                .lock()
                .map(|shape| (shape.shown, taking(shape.rect, shape.shown)))
                .unwrap_or_default();
            if let (Ok(cursor), Ok(corner), Ok(scale)) = (
                app.cursor_position(),
                window.outer_position(),
                window.scale_factor(),
            ) {
                let x = (cursor.x - f64::from(corner.x)) / scale;
                let y = (cursor.y - f64::from(corner.y)) / scale;
                if shown && (x, y) != last {
                    last = (x, y);
                    let _ = app.emit_to(
                        tauri::EventTarget::webview(ISLAND),
                        "island:cursor",
                        Cursor { x, y },
                    );
                }
                let takes = lands(rect, x, y);
                if !cfg!(target_os = "linux") && takes != took {
                    took = takes;
                    let _ = window.set_ignore_cursor_events(!takes);
                }
            }
            std::thread::sleep(Duration::from_millis(if shown { 33 } else { 120 }));
        }
    });
}

/// Tells the island what changed. Only the island hears it.
pub(crate) fn tell(app: &tauri::AppHandle, change: IslandChange) {
    if app.get_webview_window(ISLAND).is_some() {
        let _ = app.emit_to(
            tauri::EventTarget::webview(ISLAND),
            "island:session",
            change,
        );
    }
}

/// `island.apply` — opens or closes the island to match the setting, right
/// after it was switched.
#[tauri::command]
#[specta::specta]
pub async fn island_apply(app: tauri::AppHandle) -> Result<(), RpcError> {
    let handle = app.clone();
    // On the thread windows are made on.
    app.run_on_main_thread(move || apply(&handle))
        .map_err(|err| RpcError::internal(format!("the island would not change: {err}")))
}

/// `island.now` — every session the island would draw, asked as it mounts.
#[tauri::command]
#[specta::specta]
pub async fn island_now() -> Result<IslandNow, RpcError> {
    Ok(IslandNow {
        sessions: crate::island_feed::now(),
    })
}

/// `island.shape` — what the island draws, so only that takes the mouse.
#[tauri::command]
#[specta::specta]
pub async fn island_shape(
    app: tauri::AppHandle,
    rect: IslandRect,
    shown: bool,
) -> Result<(), RpcError> {
    let appeared = SHAPE.lock().is_ok_and(|mut shape| {
        let appeared = shown && !shape.shown;
        shape.rect = rect;
        shape.shown = shown;
        appeared
    });
    if let Some(window) = app.get_webview_window(ISLAND) {
        // Coming out of hiding, it comes out where devpit is now.
        if appeared {
            placed(&window);
        }
        shaped(&window);
    }
    Ok(())
}

/// `island.open_pane` — brings devpit forward on the terminal a session runs in.
#[tauri::command]
#[specta::specta]
pub async fn island_open_pane(
    app: tauri::AppHandle,
    project_id: Option<String>,
    pane_id: String,
) -> Result<(), RpcError> {
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| RpcError::internal("there is no main window".to_owned()))?;
    let _ = main.unminimize();
    let _ = main.show();
    let _ = main.set_focus();
    // To `main` by name: a page in a browser pane must not hear it.
    app.emit_to(
        tauri::EventTarget::webview("main"),
        "island:open-pane",
        (project_id, pane_id),
    )
    .map_err(|err| RpcError::internal(format!("devpit would not be told: {err}")))
}

/// `island.peek` — a file a session's step touches, for the preview.
///
/// Read only inside the session's own checkout or its project, through the
/// same reader the file pane uses: symlinks followed, containment checked,
/// size capped. A path anywhere else is refused, not shown.
#[tauri::command]
#[specta::specta]
pub async fn island_peek(session_id: String, path: String) -> Result<FileContents, RpcError> {
    crate::off_main::blocking(move || peek_now(&session_id, path)).await
}

fn peek_now(session_id: &str, path: String) -> Result<FileContents, RpcError> {
    let session = crate::island_feed::now()
        .into_iter()
        .find(|one| one.session_id == session_id)
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "that session has left the island"))?;
    let store = devpit_core::Store::open_default()?;
    let checkout = session
        .card_id
        .as_deref()
        .and_then(|card| store.card(card).ok().flatten())
        .and_then(|card| card.worktree_path);
    let project = session
        .project_id
        .as_deref()
        .and_then(|id| store.project(id).ok().flatten())
        .map(|project| project.root_path);
    let mut refused = RpcError::new(
        ErrorCode::Forbidden,
        "that file is outside the session's project",
    );
    for root in checkout.into_iter().chain(project) {
        match crate::files::contents(std::path::Path::new(&root), path.clone()) {
            Ok(contents) => return Ok(contents),
            Err(err) => refused = err,
        }
    }
    Err(refused)
}

/// `island.cursors` — the shape `island:cursor` carries, for the contract.
#[tauri::command]
#[specta::specta]
pub fn island_cursors() -> (Cursor, Vec<IslandChange>) {
    (Cursor { x: 0.0, y: 0.0 }, Vec::new())
}

#[cfg(test)]
#[path = "island_tests.rs"]
mod tests;
