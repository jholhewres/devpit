//! The guard that keeps devpit off the network.
//!
//! `AGENTS.md`: this process runs terminals, and reaching it is reaching the
//! machine. So every socket devpit listens on is on loopback — the hook
//! listener, and the remote viewer's when `tailscale serve` carries it. The
//! one exception is the remote viewer on the machine's own tailnet address,
//! when the tailnet has no HTTPS: that line says `tailnet-only`, in
//! `remote_host.rs` and nowhere else. Every interface, never.

use std::path::Path;

use walkdir::WalkDir;

use crate::Finding;

const LOOPBACK: [&str; 3] = ["127.0.0.1", "LOCALHOST", "localhost"];

pub fn nothing_listens_beyond_loopback(root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    for top in ["apps", "crates"] {
        for entry in WalkDir::new(root.join(top))
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "rs"))
            .filter(|entry| {
                !entry
                    .path()
                    .components()
                    .any(|part| part.as_os_str() == "target")
            })
        {
            let path = entry.path();
            let Ok(text) = std::fs::read_to_string(path) else {
                continue;
            };
            findings.extend(offences(&text).into_iter().map(|(line, what)| Finding {
                file: path.strip_prefix(root).unwrap_or(path).to_path_buf(),
                line,
                what,
            }));
        }
    }
    findings
        .into_iter()
        .filter(|finding| {
            !(finding.what.contains("tailnet-only") && finding.file.ends_with("remote_host.rs"))
        })
        .collect()
}

/// The lines of `text` that listen beyond loopback, and why each is wrong.
pub(crate) fn offences(text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (at, line) in text.lines().enumerate() {
        let code = line.split("//").next().unwrap_or(line);
        let every =
            code.contains("0.0.0.0") || code.contains("UNSPECIFIED") || code.contains("[::]");
        let binds = code.contains("TcpListener::bind(") || code.contains("UdpSocket::bind(");
        if every {
            found.push((
                at + 1,
                format!(
                    "listening on every interface is never allowed: `{}`",
                    code.trim()
                ),
            ));
        } else if binds && !LOOPBACK.iter().any(|word| code.contains(word)) {
            let tag = if line.contains("// tailnet-only") {
                " (tailnet-only)"
            } else {
                ""
            };
            found.push((
                at + 1,
                format!("a listener must be on loopback{tag}: `{}`", code.trim()),
            ));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::offences;

    #[test]
    fn only_loopback_is_listened_on() {
        assert!(offences(r#"let l = TcpListener::bind(("127.0.0.1", 0));"#).is_empty());
        assert!(offences("let l = TcpListener::bind((Ipv4Addr::LOCALHOST, 0));").is_empty());
        assert_eq!(
            offences(r#"let l = TcpListener::bind("0.0.0.0:80");"#).len(),
            1
        );
        assert_eq!(
            offences("let l = TcpListener::bind(SocketAddr::new(ip, port));").len(),
            1
        );
        assert_eq!(offences("let any = Ipv4Addr::UNSPECIFIED;").len(), 1);
        let tagged =
            offences("let l = TcpListener::bind(SocketAddr::new(ip, port)) // tailnet-only");
        assert!(tagged[0].1.contains("tailnet-only"));
    }
}
