//! A git origin as the identity of a project across machines.
//!
//! Two clones of one repository must agree on which project they are, over
//! ssh on one machine and https on another, so the hash is taken over the
//! normalised origin: host and path, nothing else.

/// The normalised git origin, hashed.
pub fn origin_hash(url: &str) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(normalised(url).as_bytes());
    digest
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The hash a project is known by, or none for an origin that names nothing.
pub fn identity(url: &str) -> Option<String> {
    (!normalised(url).is_empty()).then(|| origin_hash(url))
}

/// `host/path`, lowercased: no scheme, user, password, port or `.git`. A
/// local path keeps its leading `/`, as every build before kept it.
pub fn normalised(url: &str) -> String {
    let (_, authority, path) = parts(url.trim());
    let host = host_of(authority);
    let path = match host {
        "" => path.trim_end_matches('/'),
        _ => path.trim_matches('/'),
    };
    let path = path
        .strip_suffix(".git")
        .unwrap_or(path)
        .trim_end_matches('/');
    match host {
        "" => path.to_lowercase(),
        host => format!("{host}/{path}").to_lowercase(),
    }
}

/// The origin as another machine may see it: a password goes everywhere,
/// and a user too except over ssh, where cloning needs it.
pub fn shareable(url: &str) -> String {
    let url = url.trim();
    let (scheme, authority, path) = parts(url);
    let Some((userinfo, host)) = authority.rsplit_once('@') else {
        return url.to_owned();
    };
    let ssh = scheme.is_none_or(|scheme| scheme.eq_ignore_ascii_case("ssh"));
    let user = userinfo.split(':').next().unwrap_or_default();
    let host = match ssh && !user.is_empty() {
        true => format!("{user}@{host}"),
        false => host.to_owned(),
    };
    match scheme {
        Some(scheme) => format!("{scheme}://{host}{path}"),
        None => format!("{host}:{path}"),
    }
}

/// Whether another machine could clone from it with git's plain transports:
/// http(s), ssh, git or the scp shape. Never a local path, a command
/// transport (`ext::`) or anything git would read as an option.
pub fn clonable(url: &str) -> bool {
    if url.starts_with('-') || url.contains("::") {
        return false;
    }
    match parts(url) {
        (Some(scheme), authority, _) => {
            ["https", "http", "ssh", "git"]
                .iter()
                .any(|known| scheme.eq_ignore_ascii_case(known))
                && !host_of(authority).is_empty()
        }
        (None, authority, _) => !host_of(authority).is_empty(),
    }
}

/// The host of an authority, without user, password or port.
fn host_of(authority: &str) -> &str {
    let host = authority.rsplit('@').next().unwrap_or_default();
    match host.strip_prefix('[') {
        Some(_) => host.split_inclusive(']').next().unwrap_or(host),
        None => host.split(':').next().unwrap_or_default(),
    }
}

/// Scheme, authority and path of `scheme://authority/path` or of the scp
/// shape `user@host:path`. The path keeps its leading `/` when it had one.
fn parts(url: &str) -> (Option<&str>, &str, &str) {
    if let Some((scheme, rest)) = url.split_once("://") {
        let at = rest.find('/').unwrap_or(rest.len());
        return (Some(scheme), &rest[..at], &rest[at..]);
    }
    // The host's colon comes after the user, who may carry one of their own.
    let from = url
        .find('@')
        .filter(|at| !url[..*at].contains('/'))
        .unwrap_or(0);
    match url[from..].find(':').map(|at| from + at) {
        Some(at) if !url[..at].contains('/') => (None, &url[..at], &url[at + 1..]),
        _ => (None, "", url),
    }
}

#[cfg(test)]
#[path = "origin_tests.rs"]
mod tests;
