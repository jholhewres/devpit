//! A second webview, inside this window, showing somebody else's page.
//!
//! Everything else this window draws is ours. This one is not, and that single
//! fact decides the whole file:
//!
//! 1. **It cannot be the `main` webview, ever.** `capabilities/default.json`
//!    names `main` and grants it every command devpit has. A browser webview
//!    that answered to that label would hand the command surface to whatever
//!    page is loaded. [`label_for`] makes a label that cannot collide with it,
//!    and the guard in `xtask/src/packaging.rs` keeps the capability scoped by
//!    webview rather than by window — tauri enables a window-scoped capability
//!    on *every* webview in that window, whatever its `webviews` list says.
//! 2. **It loads `http` and `https` and nothing else.** [`reachable`] is where
//!    that is decided, and it is a list of what is allowed rather than a list
//!    of what is not: `file:` reads the disk, `data:` and `javascript:` are
//!    script the caller wrote, and a scheme nobody thought about is refused
//!    for being one nobody thought about.
//! 3. **It dies with its pane.** A webview added to a window outlives the
//!    React tree that asked for it, so nothing removes it unless this does.
//!
//! `Window::add_child` is behind tauri's `unstable` feature, which is why that
//! feature is on. The cost is written at the dependency in `Cargo.toml`.

use std::path::{Path, PathBuf};

use devpit_rpc::{ErrorCode, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{Emitter as _, Manager as _};

/// Where a browser webview sits inside the window, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Where {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Where a named session keeps its cookies, its storage and its logins.
///
/// One directory per session, and wry turns it into a `WebsiteDataManager`
/// with its own cache, its own data and its own cookie file
/// (`wry-0.55.1/src/webkitgtk/web_context.rs:32-43`). Two sessions therefore
/// share nothing — not a cookie, not local storage, not a login.
///
/// The name is checked rather than trusted: it comes from a screen and ends up
/// as a path. A name that is not plain is refused instead of sanitised,
/// because a sanitised name silently becomes a *different* session and the
/// person is quietly signed into the wrong one.
pub(crate) fn session_dir(root: &Path, name: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("a browser session needs a name".to_owned());
    }
    if !name
        .chars()
        .all(|one| one.is_ascii_alphanumeric() || one == '-' || one == '_')
    {
        return Err(format!(
            "{name} is not a session name — letters, digits, dashes and underscores only"
        ));
    }
    Ok(root.join("browser").join(name))
}

/// The session a pane uses when nobody chose one.
pub(crate) const USUAL: &str = "default";

/// Which session each open pane's webview was made in.
///
/// A webview's data directory is fixed when it is built and cannot be read
/// back out of tauri, so the only way to know a pane's session is to have
/// written it down. Without this, opening a page in a *different* session on
/// a pane that already has one silently kept the old one — the screen would
/// say `work` and the cookies would be `default`'s, which is the wrong-account
/// failure this whole story exists to prevent.
#[derive(Default, Clone)]
pub struct Sessions(std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, String>>>);

impl Sessions {
    pub(crate) fn now_in(&self, pane: &str, session: &str) {
        if let Ok(mut held) = self.0.lock() {
            held.insert(pane.to_owned(), session.to_owned());
        }
    }

    pub(crate) fn of(&self, pane: &str) -> Option<String> {
        self.0.lock().ok()?.get(pane).cloned()
    }

    pub(crate) fn forget(&self, pane: &str) {
        if let Ok(mut held) = self.0.lock() {
            held.remove(pane);
        }
    }
}

/// The label a browser webview answers to.
///
/// Prefixed, so it can never be `main` no matter what a caller passes — and
/// the prefix is checked rather than assumed by [`ours`], because the only
/// webview this file may close is one it made.
const MINE: &str = "devpit-browser:";

/// The label for a pane's browser webview, or why that pane cannot have one.
///
/// A blank pane id would make the bare prefix, which [`ours`] refuses — so the
/// webview would be created and then nothing would agree to close it. An
/// orphan inside the window is the one outcome this file exists to prevent,
/// and it is cheaper to refuse the name than to clean up after it.
pub(crate) fn label_for(pane: &str) -> Result<String, String> {
    let pane = pane.trim();
    if pane.is_empty() {
        return Err("a browser pane needs a pane to belong to".to_owned());
    }
    Ok(format!("{MINE}{pane}"))
}

/// Whether a label is one of ours, which is the question `close` asks before
/// it closes anything.
pub(crate) fn ours(label: &str) -> bool {
    label.starts_with(MINE) && label.len() > MINE.len()
}

/// Whether a url is one a browser pane may be pointed at.
///
/// An allowlist. `file:` would read this machine's disk with the page's own
/// script; `data:` and `javascript:` are script the caller wrote, which makes
/// "navigate" into "run this"; and anything else is refused for being
/// something nobody here has thought about.
pub(crate) fn reachable(url: &str) -> Result<tauri::Url, String> {
    /* `tauri::Url`, not a `url` dependency of our own: tauri re-exports the
    same type it parses navigation with, so there is one parser here and not
    two that could disagree about what a url is. */
    let parsed = tauri::Url::parse(url).map_err(|_| format!("{url} is not a url"))?;
    match parsed.scheme() {
        "http" | "https" => Ok(parsed),
        other => Err(format!(
            "a browser pane opens http and https; {other} is not one of them"
        )),
    }
}

/// A size a window can actually hold.
///
/// Zero and negative arrive from a pane that has not been measured yet, and a
/// webview of zero height is one nobody can see and nobody can close.
pub(crate) fn sized(place: Where) -> Where {
    Where {
        x: place.x.max(0.0),
        y: place.y.max(0.0),
        width: place.width.max(1.0),
        height: place.height.max(1.0),
    }
}

fn refused(what: String) -> RpcError {
    RpcError::new(ErrorCode::Invalid, what)
}

/// Opens a page in a webview of its own, over the pane that asked.
///
/// Off the main webview's tree entirely: this is a sibling inside the window,
/// not an iframe, so the page cannot reach the document devpit draws.
#[tauri::command]
#[specta::specta]
pub async fn browser_open(
    window: tauri::Window,
    sessions: tauri::State<'_, Sessions>,
    pane: String,
    url: String,
    place: Where,
    session: String,
) -> Result<String, RpcError> {
    let address = reachable(&url).map_err(refused)?;
    let label = label_for(&pane).map_err(refused)?;
    let root = devpit_core::Store::root().map_err(|err| RpcError::internal(err.to_string()))?;
    /* A caller that named no session gets the usual one rather than a
    refusal: opening a page is the common act, and choosing a session is the
    rare one. */
    let named = if session.trim().is_empty() {
        USUAL
    } else {
        session.as_str()
    };
    let held = session_dir(&root, named).map_err(refused)?;

    if let Some(existing) = window.get_webview(&label) {
        if sessions.of(&pane).as_deref() == Some(named) {
            existing
                .navigate(address)
                .map_err(|err| RpcError::internal(format!("the page would not load: {err}")))?;
            return Ok(label);
        }
        /* A different session was asked for, and a webview's data directory
        cannot be changed after it is built. So the page is opened again in
        the session that was actually named, rather than navigating and
        quietly keeping the old one's cookies. */
        existing
            .close()
            .map_err(|err| RpcError::internal(format!("the old session would not close: {err}")))?;
        sessions.forget(&pane);
    }

    let place = sized(place);
    let built = tauri::webview::WebviewBuilder::new(&label, tauri::WebviewUrl::External(address))
        /* The page is not ours and may not be handed files by dragging. */
        .disable_drag_drop_handler()
        /* A ground under the page, because the page may not paint one. A
        Chromium error — "connection refused" on a dev server that is not up
        yet — is a white sheet, and a white sheet in the middle of a dark app
        is a flash somebody sees every time they open a pane before their
        server is running. Seen in a screenshot, not caught by any test. */
        .background_color(tauri::webview::Color(24, 24, 27, 255))
        /* The session, which is what keeps two panes from sharing a login. */
        .data_directory(held)
        /* Where the page ended up, which is not always where it was sent. */
        .on_page_load(|webview, payload| {
            let Some(pane) = pane_of(webview.label()) else {
                return;
            };
            let pane = pane.to_owned();
            let url = payload.url().to_string();
            /* Twice on purpose. The address is known the moment the page
            starts arriving and the bar should say so at once; the title only
            exists once the document has run, so it is read on the second
            pass. A bar that waits for the title to show the url is a bar that
            looks frozen on a slow page. */
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                let said = webview.clone();
                let of = pane.clone();
                let at = url.clone();
                /* `document.title` through the page, because a tauri webview
                does not report one. */
                let _ = webview.eval_with_callback("document.title", move |title| {
                    let _ = said.emit_to(
                        tauri::EventTarget::webview("main"),
                        SHOWING,
                        Showing {
                            pane: of.clone(),
                            url: at.clone(),
                            title: title.trim_matches('"').to_owned(),
                        },
                    );
                });
                return;
            }
            let _ = webview.emit_to(
                tauri::EventTarget::webview("main"),
                SHOWING,
                Showing {
                    pane,
                    url,
                    title: String::new(),
                },
            );
        });

    window
        .add_child(
            built,
            tauri::LogicalPosition::new(place.x, place.y),
            tauri::LogicalSize::new(place.width, place.height),
        )
        .map_err(|err| RpcError::internal(format!("the browser pane would not open: {err}")))?;

    /* Placed again, on purpose, and on GTK by hand. `add_child` takes a
    position; wry applies it only where the window's container is a `GtkFixed`
    or an X11 child, and tauri gives it neither — so the page was packed into
    the window's vertical box and drew *under* the app at half its height.
    `browser_gtk` is what puts it in its pane; the calls below are what tauri
    honours everywhere else. */
    if let Some(made) = window.get_webview(&label) {
        made.set_position(tauri::LogicalPosition::new(place.x, place.y))
            .and_then(|()| made.set_size(tauri::LogicalSize::new(place.width, place.height)))
            .map_err(|err| {
                RpcError::internal(format!("the browser pane would not take its place: {err}"))
            })?;
        #[cfg(all(unix, not(target_os = "macos")))]
        crate::browser_gtk::hold(&window, &made, place);
    }

    sessions.now_in(&pane, named);
    Ok(label)
}

/// Moves and resizes a pane's webview, which the window will not do for us:
/// a child webview is placed in window coordinates and knows nothing about the
/// layout that decided them.
#[tauri::command]
#[specta::specta]
pub async fn browser_place(
    window: tauri::Window,
    pane: String,
    place: Where,
) -> Result<(), RpcError> {
    let label = label_for(&pane).map_err(refused)?;
    let Some(webview) = window.get_webview(&label) else {
        /* The pane moved before its page opened, or after it closed. Neither
        is an error: there is simply nothing to move. */
        return Ok(());
    };
    let place = sized(place);
    #[cfg(all(unix, not(target_os = "macos")))]
    crate::browser_gtk::place(&webview, place);
    webview
        .set_position(tauri::LogicalPosition::new(place.x, place.y))
        .and_then(|()| webview.set_size(tauri::LogicalSize::new(place.width, place.height)))
        .map_err(|err| RpcError::internal(format!("the browser pane would not move: {err}")))
}

/// What a page is doing, told to the window as it happens.
///
/// A webview does not report its address back, and remembering the last url
/// devpit asked for is not the same thing: a link, a redirect or a form leaves
/// the bar saying where the page *was* sent rather than where it *is*. So the
/// real one is read on every load and sent over.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Showing {
    /// The pane whose page this is.
    pub pane: String,
    /// Where the page actually is, as the webview reports it.
    pub url: String,
    /// What the page calls itself, once it has finished loading and has had
    /// the chance to set one. Empty while it is still arriving, and empty for
    /// a page that names itself nothing — a tab then falls back to the url,
    /// which is what a browser has always done.
    pub title: String,
}

/// The event a browser pane listens for.
pub(crate) const SHOWING: &str = "browser:showing";

/// Carries [`Showing`] into the generated contract, and does nothing else.
///
/// specta writes down what a *command* can reach, and an event payload is
/// reachable from none — so the window would have no type for what arrives on
/// `browser:showing`. The same shape as `terminal_happenings` and
/// `card_happenings`, and listed beside them in
/// `xtask/uncalled-commands.txt` for the same reason.
#[tauri::command]
#[specta::specta]
pub fn browser_showings() -> Showing {
    Showing {
        pane: String::new(),
        url: String::new(),
        title: String::new(),
    }
}

/// The pane a browser webview belongs to, back out of its label.
pub(crate) fn pane_of(label: &str) -> Option<&str> {
    label.strip_prefix(MINE).filter(|pane| !pane.is_empty())
}

/// Runs one of the page's own history moves.
///
/// Through the page rather than through tauri, because tauri's webview has no
/// back, forward or stop — `history` and `window.stop()` are what a browser
/// exposes to the document, and they are what a browser chrome has always
/// driven.
async fn history(window: &tauri::Window, pane: &str, js: &str) -> Result<(), RpcError> {
    let label = label_for(pane).map_err(refused)?;
    let Some(webview) = window.get_webview(&label) else {
        return Ok(());
    };
    webview
        .eval(js)
        .map_err(|err| RpcError::internal(format!("the page would not answer: {err}")))
}

#[tauri::command]
#[specta::specta]
pub async fn browser_back(window: tauri::Window, pane: String) -> Result<(), RpcError> {
    history(&window, &pane, "history.back()").await
}

#[tauri::command]
#[specta::specta]
pub async fn browser_forward(window: tauri::Window, pane: String) -> Result<(), RpcError> {
    history(&window, &pane, "history.forward()").await
}

#[tauri::command]
#[specta::specta]
pub async fn browser_stop(window: tauri::Window, pane: String) -> Result<(), RpcError> {
    history(&window, &pane, "window.stop()").await
}

/// Finds the next run of text in the page, forwards or back.
///
/// `window.find` rather than anything of tauri's, for the same reason `back`
/// and `forward` go through `history`: a tauri webview has no find, and this
/// is the one every engine has had since Netscape. WebKitGTK implements it,
/// and it wraps and highlights the way a person expects.
///
/// What it cannot do is count. `window.find` answers *whether* it moved, not
/// how many matches there are, and there is no other way to ask from inside
/// the page — so the bar says "no match" or says nothing, and never shows the
/// `3 / 17` a browser with an engine hook would.
#[tauri::command]
#[specta::specta]
pub async fn browser_find(
    window: tauri::Window,
    pane: String,
    what: String,
    backwards: bool,
) -> Result<(), RpcError> {
    let what = what.trim();
    if what.is_empty() {
        return Ok(());
    }
    /* Through the same escaping the agent's own reads use, because this is a
    string a person typed going into script: a quote or a line separator in
    the box would otherwise end the literal and run what followed. */
    let needle = crate::browser_driving::as_json(what);
    history(
        &window,
        &pane,
        &format!("window.find({needle}, false, {backwards}, true)"),
    )
    .await
}

/// What a session is keeping, so emptying it can say so before it happens.
///
/// Not `Held`: `workspace.rs` already exports one, and two types with one name
/// is a contract that will not generate.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Kept {
    pub session: String,
    /// How many bytes the session's directory takes. `f64` rather than `u64`:
    /// specta refuses a type that could lose precision crossing into
    /// JavaScript, and a browser session is not measured in exabytes.
    pub bytes: f64,
    /// Whether anything is there at all.
    pub used: bool,
}

fn weighs(at: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(at) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.metadata() {
            Ok(found) if found.is_dir() => weighs(&entry.path()),
            Ok(found) => found.len(),
            Err(_) => 0,
        })
        .sum()
}

/// Every session a pane could be put in: the usual one, and whatever else has
/// been made.
///
/// Read off the disk rather than kept in a list somewhere, because the disk is
/// where a session *is* — a directory of cookies, storage and logins. A list
/// that drifted from it would offer a session that signs you into nothing, or
/// hide one that is still holding a login.
///
/// [`USUAL`] is always first and always present, even before anything has
/// opened in it. A menu whose first entry appears only after it has been used
/// is a menu with nothing in it the first time somebody looks.
#[tauri::command]
#[specta::specta]
pub async fn browser_sessions() -> Result<Vec<String>, RpcError> {
    let root = devpit_core::Store::root().map_err(|err| RpcError::internal(err.to_string()))?;
    let mut names = vec![USUAL.to_owned()];
    if let Ok(entries) = std::fs::read_dir(root.join("browser")) {
        let mut made: Vec<String> = entries
            .flatten()
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name != USUAL)
            .collect();
        made.sort();
        names.extend(made);
    }
    Ok(names)
}

/// What emptying a session would remove, asked before anything is removed.
///
/// A logout is not undoable and the session is somebody's signed-in state, so
/// the screen gets to say what goes rather than reporting it afterwards.
#[tauri::command]
#[specta::specta]
pub async fn browser_session_held(session: String) -> Result<Kept, RpcError> {
    let root = devpit_core::Store::root().map_err(|err| RpcError::internal(err.to_string()))?;
    let at = session_dir(&root, &session).map_err(refused)?;
    let bytes = weighs(&at) as f64;
    Ok(Kept {
        session,
        bytes,
        used: at.is_dir(),
    })
}

/// Empties one session, and only that one.
///
/// The other sessions are other directories and are not touched. A pane still
/// showing a page from this session keeps showing it — what goes is what is on
/// disk, and the next open starts signed out.
#[tauri::command]
#[specta::specta]
pub async fn browser_session_forget(session: String) -> Result<Kept, RpcError> {
    let root = devpit_core::Store::root().map_err(|err| RpcError::internal(err.to_string()))?;
    let at = session_dir(&root, &session).map_err(refused)?;
    /* Derived through `session_dir`, never taken from the caller: this
    removes a directory tree, and a path from a screen is not a path this
    process deletes on request. */
    if at.is_dir() {
        std::fs::remove_dir_all(&at)
            .map_err(|err| RpcError::internal(format!("the session would not empty: {err}")))?;
    }
    Ok(Kept {
        session,
        bytes: 0.0,
        used: false,
    })
}

/// Where a pane's webview actually ended up, against where it was asked to go.
///
/// Exists because a screenshot showed a page drawn somewhere other than its
/// pane and reading the code could not settle it: `add_child` and
/// `set_position` take **logical** pixels and `position()` reports
/// **physical** ones, so the two agree only while the scale factor is 1. This
/// reports both and lets a test do the arithmetic rather than assuming.
///
/// On GTK it asks the toolkit instead, because tauri cannot answer. wry fills
/// in a child webview's size and leaves its position at the origin whatever it
/// is (`wry-0.55.1/src/webkitgtk/mod.rs:824-851`) — so this command reported
/// `x = 0` for every page, which is a number that looks like a defect and is
/// really a blank. A test comparing it against a pane could not have passed,
/// and could not have failed for the right reason either.
#[tauri::command]
#[specta::specta]
pub async fn browser_where(window: tauri::Window, pane: String) -> Result<Where, RpcError> {
    let label = label_for(&pane).map_err(refused)?;
    let webview = window
        .get_webview(&label)
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "that pane has no page".to_owned()))?;
    #[cfg(all(unix, not(target_os = "macos")))]
    if let Some(found) = crate::browser_gtk::spot(&webview) {
        /* Already logical: GTK allocates in application pixels and leaves the
        scale factor to GDK, which is why wry converts *into* logical before
        allocating. Dividing again would report a page at half its size on
        every screen that is not 1×. */
        return Ok(found);
    }
    let at = webview
        .position()
        .map_err(|err| RpcError::internal(format!("the page would not say where it is: {err}")))?;
    let size = webview.size().map_err(|err| {
        RpcError::internal(format!("the page would not say how big it is: {err}"))
    })?;
    let scale = window.scale_factor().unwrap_or(1.0);
    /* Back into logical, which is what the window asked for and what the
    caller can compare against its own `getBoundingClientRect`. */
    Ok(Where {
        x: f64::from(at.x) / scale,
        y: f64::from(at.y) / scale,
        width: f64::from(size.width) / scale,
        height: f64::from(size.height) / scale,
    })
}

/// Closes a pane's webview.
///
/// Called when the pane closes, and answering that it was already gone is the
/// right answer rather than a failure — a pane closing twice is ordinary.
#[tauri::command]
#[specta::specta]
pub async fn browser_close(
    window: tauri::Window,
    sessions: tauri::State<'_, Sessions>,
    granted: tauri::State<'_, crate::browser_driving::Granted>,
    pane: String,
) -> Result<(), RpcError> {
    let label = label_for(&pane).map_err(refused)?;
    /* The prefix, checked and not assumed: this closes webviews it made and
    no others, whatever a caller sends as a pane id. */
    if !ours(&label) {
        return Err(refused("that is not a browser pane".to_owned()));
    }
    sessions.forget(&pane);
    /* And the grant goes with it. A pane id never returns, so a stale grant
    is only a slow leak — but a list of panes that may be driven should not
    contain ones that no longer exist. */
    granted.take_back(&pane);
    let Some(webview) = window.get_webview(&label) else {
        return Ok(());
    };
    webview
        .close()
        .map_err(|err| RpcError::internal(format!("the browser pane would not close: {err}")))
}

#[cfg(test)]
#[path = "browser_tests.rs"]
mod tests;
