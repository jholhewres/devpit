//! The window's own page and its assets, served to a browser on loopback
//! from the same bundle the desktop window loads.
//!
//! A path the bundle does not have is the page itself: the shell routes in
//! the browser, so `/board/…` is `index.html` and not a 404.

use std::net::TcpStream;

use crate::remote_http::{kind_of, reply};

/// The bundle's file for `path`: the page for `/`, an asset for its own path,
/// nothing above the bundle.
pub(crate) fn wanted(path: &str) -> Option<String> {
    let path = path.trim_start_matches('/');
    if path
        .split('/')
        .any(|part| part == ".." || part.starts_with('.'))
    {
        return None;
    }
    Some(if path.is_empty() {
        "index.html".to_owned()
    } else {
        path.to_owned()
    })
}

pub(crate) fn serve(app: &tauri::AppHandle, stream: &mut TcpStream, path: &str) {
    let Some(wanted) = wanted(path) else {
        reply(stream, "404 Not Found", "text/plain", b"");
        return;
    };
    let resolver = app.asset_resolver();
    let (name, asset) = match resolver.get(wanted.clone()) {
        Some(asset) => (wanted, asset),
        None => match resolver.get("index.html".to_owned()) {
            Some(page) => ("index.html".to_owned(), page),
            None => {
                reply(stream, "404 Not Found", "text/plain", b"");
                return;
            }
        },
    };
    reply(stream, "200 OK", kind_of(&name), &asset.bytes);
}

#[cfg(test)]
mod tests {
    use super::wanted;

    #[test]
    fn the_page_is_served_for_the_root_and_nothing_above_the_bundle() {
        assert_eq!(wanted("/").as_deref(), Some("index.html"));
        assert_eq!(
            wanted("/assets/app-1a.js").as_deref(),
            Some("assets/app-1a.js")
        );
        assert_eq!(wanted("/assets/../../etc/passwd"), None);
        assert_eq!(wanted("/.env"), None);
    }
}
