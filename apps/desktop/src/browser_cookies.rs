//! Moving cookies from a browser this machine already has into a pane.
//!
//! [`devpit_cookies`] reads the stores; this decides what is taken and puts it
//! where it goes. The split matters: reading a format is the same everywhere
//! and belongs in a crate with tests, and **what a person agreed to** is an
//! app decision and belongs here.
//!
//! Three rules, all of which are about consent rather than about code:
//!
//! 1. **Nothing is imported because it was found.** [`browser_stores`] lists
//!    what exists. Nothing moves until a caller names one store and the
//!    domains it may take from.
//! 2. **Per domain, not per store.** Someone importing their GitHub session
//!    into a pane did not thereby agree to hand over their bank. A store read
//!    is a store read whole — the file gives no cheaper way — but only the
//!    domains named leave this function.
//! 3. **What was taken is said afterwards.** [`Taken`] carries the browser,
//!    the count and the domains, because an import whose effect nobody can see
//!    is one nobody can undo.

use devpit_cookies::{chromium, firefox, keys, safari, Cookie, Refused};
use devpit_rpc::{ErrorCode, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::webview::Cookie as Held;
use tauri::Manager as _;

/// A cookie store on this machine that a pane could be given.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Store {
    /// What to call it on screen: `Google Chrome · Profile 1`.
    pub family: String,
    /// Where it is, which is also how a caller names it back.
    pub path: String,
    /// Whether reading it needs a key from the desktop keyring, and whether
    /// this machine can ask for one. Empty when nothing stands in the way.
    pub warning: String,
}

/// What an import actually did.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Taken {
    /// Which store it came from.
    pub family: String,
    /// How many cookies moved. `u32` rather than `usize`: specta refuses to
    /// export a type that could lose precision crossing into JavaScript, and a
    /// count that needs more than four billion is not a cookie store.
    pub count: u32,
    /// Which domains they were for, in the order they were asked for.
    pub domains: Vec<String>,
}

/// Whether a stored cookie is one the browser would send to a domain.
///
/// Both directions, because a cookie is sent **up** the tree as well as down:
///
/// - a host of `.example.com` covers `example.com` and everything under it,
///   which is what the leading dot has always meant;
/// - and asking for `app.example.com` must take the `.example.com` cookie,
///   because that is the one that signs you in. The first version of this only
///   matched the first direction, so a pane on a subdomain imported nothing
///   useful and the screen cheerfully said "Brought 0 cookies".
///
/// What it must never do is match sideways. `example.com` and
/// `notexample.com` share a suffix and nothing else, and a naive `ends_with`
/// hands over a session for a site nobody named.
pub(crate) fn belongs(host: &str, domain: &str) -> bool {
    let host = host
        .trim_start_matches('.')
        .trim_end_matches('.')
        .to_ascii_lowercase();
    let domain = domain
        .trim_start_matches('.')
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host.is_empty() || domain.is_empty() {
        return false;
    }
    if host == domain {
        return true;
    }
    /* The dot is part of both tests, not decoration: without it
    `notexample.com` ends with `example.com`. */
    host.ends_with(&format!(".{domain}")) || domain.ends_with(&format!(".{host}"))
}

/// The cookies from a store that the domains asked for cover.
pub(crate) fn wanted(found: Vec<Cookie>, domains: &[String]) -> Vec<Cookie> {
    found
        .into_iter()
        .filter(|cookie| domains.iter().any(|domain| belongs(&cookie.host, domain)))
        .collect()
}

fn home() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_default()
}

/// Every cookie store on this machine, with what stands in the way of each.
///
/// Listing is not importing. This is what a person chooses from.
#[tauri::command]
#[specta::specta]
pub async fn browser_stores() -> Result<Vec<Store>, RpcError> {
    let home = home();
    let mut found = Vec::new();

    for (family, path) in chromium::stores_in(&home) {
        let path: std::path::PathBuf = path;
        /* Said now rather than after a failed import: a profile encrypted
        under a keyring secret refuses every value at once, and without this
        sentence that is indistinguishable from a corrupt store. */
        let warning = match keys::password_for(&keys::TRUSTED, &family).1 {
            keys::Found::Unreachable { why } => why,
            _ => String::new(),
        };
        found.push(Store {
            family,
            path: path.display().to_string(),
            warning,
        });
    }
    for (family, path) in firefox::stores_in(&home) {
        found.push(Store {
            family,
            path: path.display().to_string(),
            warning: String::new(),
        });
    }
    for (family, path) in safari::stores_in(&home) {
        found.push(Store {
            family,
            path: path.display().to_string(),
            warning: String::new(),
        });
    }
    Ok(found)
}

/// Reads one store, keeps the domains asked for, and gives them to a pane.
///
/// The store is named by the path `browser_stores` reported, and checked
/// against that list rather than trusted: a path from a screen is not a path
/// this process opens on request.
#[tauri::command]
#[specta::specta]
pub async fn browser_import(
    window: tauri::Window,
    pane: String,
    path: String,
    domains: Vec<String>,
) -> Result<Taken, RpcError> {
    if domains.is_empty() {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            "name at least one domain to import — an import of everything is not an offer devpit makes"
                .to_owned(),
        ));
    }

    /* The store has to be one this machine actually offered. */
    let offered = browser_stores().await?;
    let store = offered.iter().find(|one| one.path == path).ok_or_else(|| {
        RpcError::new(
            ErrorCode::NotFound,
            "that is not a cookie store devpit offered".to_owned(),
        )
    })?;

    let at = std::path::Path::new(&path);
    let read: Result<Vec<Cookie>, Refused> = if store.family.starts_with("Firefox") {
        firefox::read(at)
    } else if store.family.starts_with("Safari") {
        safari::read(at)
    } else {
        let (password, _) = keys::password_for(&keys::TRUSTED, &store.family);
        chromium::read(at, &password)
    };
    let found = read.map_err(|refused| RpcError::new(ErrorCode::Invalid, refused.to_string()))?;

    let keep = wanted(found, &domains);
    let label =
        crate::browser::label_for(&pane).map_err(|why| RpcError::new(ErrorCode::Invalid, why))?;
    let webview = window.get_webview(&label).ok_or_else(|| {
        RpcError::new(ErrorCode::NotFound, "that pane has no page open".to_owned())
    })?;

    let mut count = 0_u32;
    for cookie in &keep {
        let mut built = Held::new(cookie.name.clone(), cookie.value.seen().to_owned());
        built.set_domain(cookie.host.clone());
        built.set_path(cookie.path.clone());
        built.set_secure(cookie.secure);
        built.set_http_only(cookie.http_only);
        /* A failure here is not reported with the cookie in it: the value is a
        credential and an error message is a place values end up in logs. */
        webview.set_cookie(built).map_err(|_| {
            RpcError::internal(format!(
                "the page would not take the cookies from {}",
                store.family
            ))
        })?;
        count += 1;
    }

    Ok(Taken {
        family: store.family.clone(),
        count,
        domains,
    })
}

#[cfg(test)]
#[path = "browser_cookies_tests.rs"]
mod tests;
