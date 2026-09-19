//! An agent driving a page, through the page's own script.
//!
//! tauri gives a webview `eval` and `eval_with_callback` and nothing else —
//! there is no click, no type, no screenshot. So every act here is JavaScript
//! run inside the page, which is the same idea as Orca's bridge
//! (`agent-browser-bridge-*`) over a different transport: they speak CDP to a
//! Chromium they launched, and this speaks JavaScript to a webview it owns.
//!
//! **Two things decide this file, and neither is the driving.**
//!
//! 1. **Nothing here builds JavaScript by pasting a string into it.** Every
//!    value an agent supplies crosses into the page as JSON through
//!    [`as_json`], because a selector or a phrase concatenated into source is
//!    an agent writing script in somebody's logged-in session. That is not a
//!    hypothetical: a page opened with a session imported by US-026 is exactly
//!    where it would matter.
//! 2. **An agent drives a pane it was given, and no other.** [`Granted`] is
//!    the list, it starts empty, and a pane the person is typing into is not
//!    silently taken over.
//!
//! The page is read as **structure**, not as pixels: [`READ_THE_PAGE`] walks
//! what is on screen and returns roles, names and positions. That is what
//! Orca's `snapshot-engine.ts` does, and it is why an agent can understand a
//! site without an image — which matters here, because tauri cannot give one.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use devpit_rpc::{ErrorCode, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{Emitter as _, Manager as _, State};

/// The panes an agent has been given.
///
/// Empty at start and never filled by accident: a pane is granted by the
/// person, from the pane. An agent that asks about one it was not given is
/// told no rather than told nothing, so the refusal reaches the transcript.
#[derive(Default, Clone)]
pub struct Granted(Arc<Mutex<HashSet<String>>>);

impl Granted {
    pub(crate) fn give(&self, pane: &str) {
        if let Ok(mut held) = self.0.lock() {
            held.insert(pane.to_owned());
        }
    }

    pub(crate) fn take_back(&self, pane: &str) {
        if let Ok(mut held) = self.0.lock() {
            held.remove(pane);
        }
    }

    pub(crate) fn allows(&self, pane: &str) -> bool {
        self.0.lock().is_ok_and(|held| held.contains(pane))
    }
}

/// A value on its way into a page, as JSON.
///
/// The whole reason this exists: `format!("document.querySelector('{sel}')")`
/// with a selector somebody else chose is that somebody writing JavaScript in
/// a session that may be logged in. `serde_json` produces a quoted literal
/// that a parser reads as data, whatever is inside it.
pub(crate) fn as_json(value: &str) -> String {
    let literal = serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned());
    /* `serde_json` leaves U+2028 and U+2029 as themselves, and before ES2019
    those two ended a JavaScript string literal on sight. Every engine this
    ships on is newer than that, so this is not exploitable today — it is
    escaped anyway, because the alternative is a security property that holds
    only while an assumption about somebody else's engine holds, and nothing
    would tell us when it stopped. */
    literal
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

/// What an agent asked a page to do.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", tag = "act")]
pub enum Act {
    /// Put text into the element a selector names.
    Type { selector: String, text: String },
    /// Click the element a selector names.
    Click { selector: String },
    /// Scroll the page by a number of viewport heights.
    Scroll { by: f64 },
}

impl Act {
    /// The script this act becomes.
    ///
    /// Every value is a JSON literal, never a piece of the source. The script
    /// returns a string so the caller learns whether the element was there —
    /// an act against a page that has navigated away must say so rather than
    /// act on whatever is there now.
    pub(crate) fn script(&self) -> String {
        match self {
            Self::Type { selector, text } => format!(
                "(function(){{const n=document.querySelector({});if(!n)return 'no element';\
                 n.focus();n.value={};\
                 n.dispatchEvent(new Event('input',{{bubbles:true}}));\
                 n.dispatchEvent(new Event('change',{{bubbles:true}}));return 'ok';}})()",
                as_json(selector),
                as_json(text)
            ),
            Self::Click { selector } => format!(
                "(function(){{const n=document.querySelector({});if(!n)return 'no element';\
                 n.click();return 'ok';}})()",
                as_json(selector)
            ),
            Self::Scroll { by } => format!(
                "(function(){{window.scrollBy(0,window.innerHeight*{});return 'ok';}})()",
                /* A number, formatted as one. Not `as_json`, because a JSON
                string here would scroll by nothing and say it worked. */
                if by.is_finite() { *by } else { 0.0 }
            ),
        }
    }

    /// What the pane shows while this happens, so a person watching sees what
    /// is being done to the page rather than reading about it afterwards.
    pub(crate) fn said(&self) -> String {
        match self {
            Self::Type { selector, .. } => format!("typed into {selector}"),
            Self::Click { selector } => format!("clicked {selector}"),
            Self::Scroll { by } => format!("scrolled {by} screens"),
        }
    }
}

/// The page as structure: roles, names, positions.
///
/// Not a screenshot, because tauri cannot take one — and not a regret either:
/// an accessibility read is steadier than an image and far cheaper, which is
/// why Orca's own agent bridge reads one too.
///
/// Only what is actually on screen and actually interactive: a page's whole
/// DOM is thousands of nodes and almost none of them are things to act on.
///
/// **A field's value is never read when the field holds a secret.** An
/// unlabelled input falls back to its own `value` for a name, and a password
/// field written the `<label for>` way has no `aria-label` and no
/// `placeholder` — so the typed password became the element's name and went
/// into an agent's transcript. Found in review, not by a test.
///
/// What a secret field reports instead is that it is one, and whether it has
/// something in it: an agent needs to know there is a password box and whether
/// it is filled. It never needs to know what is in it. The test is by
/// `type`, and by what the field calls itself — `autocomplete`, `name` and
/// `id` — because `type=password` alone misses a one-time code, a CVC and
/// every site that rolls its own.
pub(crate) const READ_THE_PAGE: &str = "(function(){\
const wanted='a,button,input,textarea,select,summary,[role=button],[role=link],[role=textbox],[contenteditable=true]';\
const out=[];\
const secret=function(n){\
if(n.type==='password')return true;\
const a=((n.autocomplete||'')+' '+(n.name||'')+' '+(n.id||'')).toLowerCase();\
return /pass|secret|token|otp|cvc|cvv|card-number|creditcard/.test(a);};\
document.querySelectorAll(wanted).forEach(function(n){\
const b=n.getBoundingClientRect();\
if(b.width<1||b.height<1)return;\
if(b.bottom<0||b.top>window.innerHeight)return;\
const hidden=secret(n);\
const typed=hidden?'':(n.value||'');\
const name=(n.getAttribute('aria-label')||n.getAttribute('placeholder')||typed||n.innerText||n.title||'').trim().slice(0,120);\
out.push({role:n.getAttribute('role')||n.tagName.toLowerCase(),name:name,\
secret:hidden,filled:hidden?!!n.value:undefined,\
x:Math.round(b.x),y:Math.round(b.y),w:Math.round(b.width),h:Math.round(b.height),\
disabled:!!n.disabled});});\
return JSON.stringify({title:document.title,url:location.href,elements:out.slice(0,200)});})()";

/// The event a pane listens for, so what an agent does is drawn as it happens.
pub(crate) const DRIVEN: &str = "browser:driven";

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Drove {
    pub pane: String,
    pub said: String,
}

/// Carries [`Drove`] into the generated contract, and does nothing else — the
/// same shape as `browser_showings` and for the same reason.
#[tauri::command]
#[specta::specta]
pub fn browser_drivings() -> Drove {
    Drove {
        pane: String::new(),
        said: String::new(),
    }
}

/// Lets an agent drive this pane, or stops letting it.
#[tauri::command]
#[specta::specta]
pub async fn browser_grant(
    granted: State<'_, Granted>,
    pane: String,
    may: bool,
) -> Result<(), RpcError> {
    if may {
        granted.give(&pane);
    } else {
        granted.take_back(&pane);
    }
    Ok(())
}

fn not_given(pane: &str) -> RpcError {
    RpcError::new(
        ErrorCode::Conflict,
        format!("nobody gave an agent the {pane} pane to drive"),
    )
}

/// Reads the page as structure, and waits for the answer.
///
/// `eval_with_callback` hands the result to a closure rather than returning
/// it, so the answer comes back over a channel. With a deadline: a page that
/// never answers — one still loading, one that navigated away mid-read —
/// must not leave an agent waiting forever.
#[tauri::command]
#[specta::specta]
pub async fn browser_read(
    window: tauri::Window,
    granted: State<'_, Granted>,
    pane: String,
) -> Result<String, RpcError> {
    if !granted.allows(&pane) {
        return Err(not_given(&pane));
    }
    let label =
        crate::browser::label_for(&pane).map_err(|why| RpcError::new(ErrorCode::Invalid, why))?;
    let webview = window
        .get_webview(&label)
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "that pane has no page".to_owned()))?;

    let (say, heard) = tokio::sync::oneshot::channel();
    let once = std::sync::Mutex::new(Some(say));
    webview
        .eval_with_callback(READ_THE_PAGE, move |answer| {
            if let Ok(mut held) = once.lock() {
                if let Some(say) = held.take() {
                    let _ = say.send(answer);
                }
            }
        })
        .map_err(|err| RpcError::internal(format!("the page would not answer: {err}")))?;

    match tokio::time::timeout(std::time::Duration::from_secs(5), heard).await {
        Ok(Ok(said)) => Ok(said),
        Ok(Err(_)) => Err(RpcError::internal(
            "the page went away before it answered".to_owned(),
        )),
        Err(_) => Err(RpcError::new(
            ErrorCode::Conflict,
            "the page did not answer in five seconds — it may still be loading".to_owned(),
        )),
    }
}

/// Does one thing to a page on an agent's behalf.
#[tauri::command]
#[specta::specta]
pub async fn browser_act(
    window: tauri::Window,
    granted: State<'_, Granted>,
    pane: String,
    act: Act,
) -> Result<(), RpcError> {
    if !granted.allows(&pane) {
        return Err(not_given(&pane));
    }
    let label =
        crate::browser::label_for(&pane).map_err(|why| RpcError::new(ErrorCode::Invalid, why))?;
    let webview = window
        .get_webview(&label)
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "that pane has no page".to_owned()))?;

    /* Drawn before it happens, not after: a person watching should see the
    act arrive rather than learn about it once the page has changed. */
    let _ = window.emit(
        DRIVEN,
        Drove {
            pane: pane.clone(),
            said: act.said(),
        },
    );

    webview
        .eval(act.script())
        .map_err(|err| RpcError::internal(format!("the page would not answer: {err}")))
}

#[cfg(test)]
#[path = "browser_driving_tests.rs"]
mod tests;
