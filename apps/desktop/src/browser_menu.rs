//! The browser pane's menu, in a window of its own.
//!
//! It is a dropdown, and a dropdown has no business being a window. This one
//! has to be, and the reason is the whole of `browser_gtk`: a browser pane's
//! page is a **second native webview** in this window, and two native webviews
//! have no z-order between them. Whatever is added last is on top, and that
//! has to be the page — so anything devpit draws over a pane comes out
//! underneath it. The menu opened behind the site.
//!
//! Three things were possible and this is the one that costs the page nothing:
//!
//! 1. Hide the page while the menu is open. The menu is visible and the page
//!    is a black rectangle every time somebody opens a menu.
//! 2. Shrink the page to sit below the menu. The page stays visible and
//!    re-lays out twice per open, which reads as the page reloading.
//! 3. **Put the menu in its own window.** The operating system stacks windows,
//!    so it is simply on top. The page is not touched at all.
//!
//! **One window, made once, shown and hidden.** A window per opening would
//! load the whole frontend bundle per opening, and a dropdown that takes half
//! a second to appear is not a dropdown. This one is built on first use and
//! then only moved.
//!
//! **It is wider and taller than the panel it holds**, and transparent around
//! it. The panel is anchored to the top right and its flyouts open leftwards
//! into that empty room — a window sized to the panel would clip every
//! submenu, because a child window cannot draw outside its parent.

use devpit_rpc::{ErrorCode, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{Emitter as _, Manager as _};

use crate::browser::Where;

/// The label the menu window answers to.
///
/// Its own prefix, disjoint from `devpit-browser:`, so no glob that reaches
/// this can ever reach a page. `capabilities/default.json` grants commands to
/// this label by name for that reason, and `xtask/src/packaging.rs` refuses a
/// capability whose patterns could match a browser webview.
pub const MENU: &str = "devpit-menu";

/// How much room the window gives the panel, in logical pixels.
///
/// Wider than the panel on purpose: `PANEL` is the menu itself and the rest is
/// transparent space to its left for the flyouts to open into. Tall enough for
/// the longest list this menu has — every browser profile on the machine.
const WIDE: f64 = 580.0;
const TALL: f64 = 560.0;

/// What the menu is being opened for, sent to it the moment it is shown.
///
/// Everything it needs to draw itself correctly on the first frame. A menu
/// that opened and *then* asked which session was current would show the wrong
/// tick for as long as the round trip took, and the tick is the one thing
/// somebody opens this to check.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MenuFor {
    pub pane: String,
    /// Where the pane's page is, which is the site the import offers to narrow
    /// to. Empty for a pane with nothing open.
    pub at: String,
    pub session: String,
    /// The page-width preset's id, or none for the whole pane.
    pub viewport: Option<String>,
    pub granted: bool,
}

/// The event the menu window listens on, for the openings after the first.
pub(crate) const FOR: &str = "browser:menu-for";

/// What the menu was last opened for, kept so it can be *asked* for.
///
/// The menu used to be told and nothing else, and on the first opening it was
/// told before it existed: `browser_menu_show` builds the window and emits in
/// the same breath, while the window is still loading the frontend that would
/// have listened. The event went nowhere and the panel stayed empty — the
/// dropdown "did not work", once, and then worked every time after.
///
/// So the window asks on mount and listens as well. Asking covers the opening
/// that built it; listening covers every opening after, when the document is
/// already loaded and there is nothing to mount.
#[derive(Default, Clone)]
pub struct Opening(std::sync::Arc<std::sync::Mutex<Option<MenuFor>>>);

impl Opening {
    fn now(&self, showing: &MenuFor) {
        if let Ok(mut held) = self.0.lock() {
            *held = Some(showing.clone());
        }
    }

    fn last(&self) -> Option<MenuFor> {
        self.0.lock().ok()?.clone()
    }
}

/// What the menu is open for, asked by the menu itself as it mounts.
///
/// Answers nothing when the menu has never been opened, which is a state the
/// window can be in: it is built hidden and it draws nothing until it knows.
#[tauri::command]
#[specta::specta]
pub async fn browser_menu_showing(
    opening: tauri::State<'_, Opening>,
) -> Result<Option<MenuFor>, RpcError> {
    Ok(opening.last())
}

/// The event the pane listens on, for what the menu was used to do.
pub(crate) const DID: &str = "browser:menu-did";

/// What somebody did in the menu, on its way back to the pane that owns it.
///
/// The menu runs in another window, so it cannot reach the pane's React state.
/// Commands that need no window — listing sessions, reading a store, granting
/// — it calls itself; anything that changes what the *pane* shows comes back
/// through here.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", tag = "did")]
pub enum Did {
    /// Put the pane in this session, which reopens the page in it.
    Session { session: String },
    /// Hold the page at this width, or at the pane's own when none.
    Viewport { viewport: Option<String> },
    /// Whether an agent may drive this page.
    Grant { granted: bool },
    /// Something to tell the person, in the pane's own strip.
    Said { said: String },
}

fn refused(what: String) -> RpcError {
    RpcError::new(ErrorCode::Invalid, what)
}

/// Where the menu window goes, given the window's own corner and the control
/// that asked.
///
/// Its own function because it is arithmetic across three coordinate spaces —
/// the screen, the window, and the panel inside the menu window — and that is
/// the kind of thing that is wrong by one term and looks plausible on the
/// screen it was written on. A test can check it; a screenshot cannot.
///
/// The window's right edge lines up with the control's, because the panel is
/// anchored to the right inside it. The room to the left is what the flyouts
/// open into.
pub(crate) fn under(corner: (f64, f64), at: Where) -> (f64, f64) {
    (
        corner.0 + at.x + at.width - WIDE,
        corner.1 + at.y + at.height + 4.0,
    )
}

/// Builds the menu window, once, hidden.
///
/// Hidden rather than shown, because the first thing that happens to it is
/// being positioned — and a window that appears at the origin and then jumps
/// to the control is a window somebody watched move.
fn made(app: &tauri::AppHandle) -> Result<tauri::WebviewWindow, RpcError> {
    if let Some(found) = app.get_webview_window(MENU) {
        return Ok(found);
    }
    tauri::WebviewWindowBuilder::new(app, MENU, tauri::WebviewUrl::App("index.html".into()))
        /* What tells the frontend to draw the menu instead of the app. A
        query string would have done it too and would have gone through the
        asset protocol's path handling; this is read before any of our code
        runs and cannot be confused for a route. */
        .initialization_script("window.__DEVPIT_MENU__ = true")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .visible(false)
        .inner_size(WIDE, TALL)
        .title("devpit menu")
        .build()
        .map_err(|err| RpcError::internal(format!("the menu would not open: {err}")))
        .inspect(|built| {
            /* Clicking away puts it back, which is the one thing every
            dropdown does and the one thing a window does not do for free.
            Without it the menu is a panel floating over the desktop that
            follows devpit around until somebody finds the control again. */
            let away = built.clone();
            built.on_window_event(move |event| {
                if matches!(event, tauri::WindowEvent::Focused(false)) {
                    let _ = away.hide();
                }
            });
        })
}

/// Shows the menu under the control that asked for it.
///
/// `at` is the control's own rectangle in the main window's logical
/// coordinates — the same thing a browser pane sends for where its page goes,
/// measured the same way. This turns it into a place on the screen.
#[tauri::command]
#[specta::specta]
pub async fn browser_menu_show(
    app: tauri::AppHandle,
    opening: tauri::State<'_, Opening>,
    pane: String,
    at: Where,
    showing: MenuFor,
) -> Result<(), RpcError> {
    if pane.trim().is_empty() {
        return Err(refused("a menu belongs to a pane".to_owned()));
    }
    /* Written down before the window is built, so the window can ask for it
    the moment its frontend is running — which on a first opening is after
    this function has already finished emitting. */
    opening.now(&showing);
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| RpcError::internal("there is no main window".to_owned()))?;
    let scale = main.scale_factor().unwrap_or(1.0);
    let corner = main.outer_position().map_err(|err| {
        RpcError::internal(format!("the window would not say where it is: {err}"))
    })?;

    /* `outer_position` is physical and `at` is logical, which is the mistake
    `browser_where` was written to catch in the other direction. Divided here,
    once, so everything `under` sees is in the same units. */
    let (x, y) = under(
        (f64::from(corner.x) / scale, f64::from(corner.y) / scale),
        at,
    );

    let menu = made(&app)?;
    menu.set_position(tauri::LogicalPosition::new(x, y))
        .map_err(|err| RpcError::internal(format!("the menu would not take its place: {err}")))?;
    menu.show()
        .map_err(|err| RpcError::internal(format!("the menu would not show: {err}")))?;
    menu.set_focus()
        .map_err(|err| RpcError::internal(format!("the menu would not take focus: {err}")))?;
    /* For every opening after the one that built the window. That one is
    covered by `browser_menu_showing`, which the window asks on mount: an
    event emitted at a document that has not loaded yet reaches nobody, and
    this used to be the whole of how the menu was told. */
    app.emit_to(tauri::EventTarget::webview(MENU), FOR, showing)
        .map_err(|err| RpcError::internal(format!("the menu would not be told: {err}")))?;
    Ok(())
}

/// Puts the menu away.
///
/// Hidden and not closed: closing would throw away the frontend it has loaded,
/// and the next opening would load it again.
#[tauri::command]
#[specta::specta]
pub async fn browser_menu_hide(app: tauri::AppHandle) -> Result<(), RpcError> {
    let Some(menu) = app.get_webview_window(MENU) else {
        return Ok(());
    };
    menu.hide()
        .map_err(|err| RpcError::internal(format!("the menu would not close: {err}")))
}

/// Carries back what the menu was used to do.
///
/// The menu is in another window and the pane's state lives in `main`, so this
/// is the way across. It emits to `main` by name rather than broadcasting: an
/// event every webview hears is an event a *page* could hear, and a page is
/// the one thing in this window that is not ours.
#[tauri::command]
#[specta::specta]
pub async fn browser_menu_did(
    app: tauri::AppHandle,
    pane: String,
    did: Did,
) -> Result<(), RpcError> {
    if pane.trim().is_empty() {
        return Err(refused("a menu belongs to a pane".to_owned()));
    }
    app.emit_to(
        tauri::EventTarget::webview("main"),
        DID,
        (pane.clone(), did),
    )
    .map_err(|err| RpcError::internal(format!("the pane would not be told: {err}")))?;
    Ok(())
}

/// Carries [`MenuFor`] and [`Did`] into the generated contract.
///
/// specta writes down what a *command* can reach, and neither of these is
/// reachable from one — `MenuFor` goes out on an event and `Did` comes back as
/// an argument that the window has to be able to name. Listed in
/// `xtask/uncalled-commands.txt` beside the others of its kind.
#[tauri::command]
#[specta::specta]
pub fn browser_menus() -> (MenuFor, Did) {
    (
        MenuFor {
            pane: String::new(),
            at: String::new(),
            session: String::new(),
            viewport: None,
            granted: false,
        },
        Did::Grant { granted: false },
    )
}

#[cfg(test)]
#[path = "browser_menu_tests.rs"]
mod tests;
