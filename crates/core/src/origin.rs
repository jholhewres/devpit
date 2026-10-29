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

/// `host/path`, lowercased: no scheme, user, password, port or `.git`.
pub fn normalised(url: &str) -> String {
    let (_, userinfo_host, path) = parts(url.trim());
    let host = userinfo_host.rsplit('@').next().unwrap_or_default();
    let host = host.split(':').next().unwrap_or_default();
    let path = path.trim_matches('/');
    let path = path
        .strip_suffix(".git")
        .unwrap_or(path)
        .trim_end_matches('/');
    match host {
        "" => path.to_lowercase(),
        host => format!("{host}/{path}").to_lowercase(),
    }
}

/// The origin as another machine may see it: a user or password in an http
/// address, or any password, is taken out; an ssh user stays, cloning needs it.
pub fn shareable(url: &str) -> String {
    let url = url.trim();
    let (scheme, authority, path) = parts(url);
    let Some((userinfo, host)) = authority.rsplit_once('@') else {
        return url.to_owned();
    };
    let web = matches!(scheme, Some("http" | "https"));
    if !web && !userinfo.contains(':') {
        return url.to_owned();
    }
    match scheme {
        Some(scheme) => format!("{scheme}://{host}{path}"),
        None => format!("{host}:{path}"),
    }
}

/// Scheme, authority and path of `scheme://authority/path` or of the scp
/// shape `user@host:path`. The path keeps its leading `/` when it had one.
fn parts(url: &str) -> (Option<&str>, &str, &str) {
    if let Some((scheme, rest)) = url.split_once("://") {
        let at = rest.find('/').unwrap_or(rest.len());
        return (Some(scheme), &rest[..at], &rest[at..]);
    }
    match url.split_once(':') {
        Some((authority, path)) if !authority.contains('/') => (None, authority, path),
        _ => (None, "", url),
    }
}

#[cfg(test)]
#[path = "origin_tests.rs"]
mod tests;
