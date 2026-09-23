//! Where a commit lives on the web, read off the checkout's remote.
//!
//! Only the shape of the remote's address is used: nothing is fetched, and a
//! remote on a host this does not recognise gets the path most forges share.

use std::path::Path;

use crate::invoke::run;
use crate::GitError;

/// A commit, named in full and, where the remote has a web page, there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitLink {
    pub full: String,
    pub url: Option<String>,
}

/// The full id of `sha` and its page on the remote's forge.
pub fn commit_link(root: &Path, sha: &str) -> Result<CommitLink, GitError> {
    let spec = format!("{sha}^{{commit}}");
    let full = run(root, &["rev-parse", "--verify", "--quiet", &spec])?
        .trim()
        .to_owned();
    let remote = run(root, &["remote"])
        .ok()
        .and_then(|all| {
            let names: Vec<&str> = all
                .lines()
                .map(str::trim)
                .filter(|n| !n.is_empty())
                .collect();
            // `origin` when there is one: it is the convention, and the first
            // in the list is only whichever sorts first.
            names
                .iter()
                .find(|name| **name == "origin")
                .or(names.first())
                .map(|name| (*name).to_owned())
        })
        .and_then(|name| run(root, &["remote", "get-url", &name]).ok());
    let url = remote.and_then(|remote| web_url(remote.trim(), &full));
    Ok(CommitLink { full, url })
}

/// `git@github.com:o/r.git`, `ssh://git@host:22/o/r`, `https://u@host/o/r.git`
/// — each as the page of one commit on that host.
pub fn web_url(remote: &str, full: &str) -> Option<String> {
    let rest = remote
        .strip_prefix("https://")
        .or_else(|| remote.strip_prefix("http://"))
        .or_else(|| remote.strip_prefix("ssh://"))
        .or_else(|| remote.strip_prefix("git://"));
    let (host, path) = match rest {
        Some(rest) => {
            let rest = rest.rsplit_once('@').map_or(rest, |(_, after)| after);
            let (host, path) = rest.split_once('/')?;
            // A port belongs to ssh, not to the web page.
            (host.split(':').next()?.to_owned(), path.to_owned())
        }
        // The scp form: `user@host:path`. A local path has no `:` before a `/`.
        None => {
            let (before, path) = remote.split_once(':')?;
            if before.contains('/') {
                return None;
            }
            let host = before.rsplit_once('@').map_or(before, |(_, after)| after);
            (host.to_owned(), path.to_owned())
        }
    };
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    if host.is_empty() || path.is_empty() || path.contains(char::is_whitespace) {
        return None;
    }
    let page = if host.contains("bitbucket") {
        "commits"
    } else if host.contains("gitlab") {
        "-/commit"
    } else {
        "commit"
    };
    Some(format!("https://{host}/{path}/{page}/{full}"))
}

#[cfg(test)]
#[path = "weblink_tests.rs"]
mod tests;
