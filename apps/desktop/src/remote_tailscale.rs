//! Tailscale, asked through its own command: whether it is there and logged
//! in, the machine's name and owner, publishing the viewer with
//! `tailscale serve`, and who is on the other end of a connection.
//!
//! The viewer is never published on every interface. With HTTPS on in the
//! tailnet, `tailscale serve` carries it from a loopback port, with a real
//! certificate and the caller's login in a header. Without HTTPS, devpit
//! listens on the machine's tailnet address alone, where only the tailnet
//! reaches, and asks `tailscale whois` who called.

use std::time::Duration;

use devpit_rpc::TailscaleState;
use serde_json::Value;

/// The HTTPS port the viewer is published on in the tailnet.
pub(crate) const HTTPS_PORT: u16 = 7443;

/// How long one `tailscale` command may take.
const AT_MOST: Duration = Duration::from_secs(8);

/// `tailscale status --json`, read.
pub(crate) fn state_of(status: &Value) -> TailscaleState {
    let me = &status["Self"];
    let user = me["UserID"].as_i64().map(|id| id.to_string());
    let login = user.and_then(|id| status["User"][&id]["LoginName"].as_str().map(str::to_owned));
    let name = me["DNSName"]
        .as_str()
        .map(|name| name.trim_end_matches('.').to_owned())
        .filter(|name| !name.is_empty());
    let https = status["CertDomains"]
        .as_array()
        .is_some_and(|domains| !domains.is_empty());
    let ip = me["TailscaleIPs"]
        .as_array()
        .and_then(|ips| {
            ips.iter()
                .filter_map(Value::as_str)
                .find(|ip| ip.contains('.'))
        })
        .map(str::to_owned);
    TailscaleState {
        installed: true,
        running: status["BackendState"].as_str() == Some("Running"),
        name,
        https,
        login,
        ip,
    }
}

/// A `tailscale` command's output, or why there is none.
fn run(args: &[&str]) -> Result<String, String> {
    let mut child = devpit_pty::host_env::command("tailscale")
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => "not installed".to_owned(),
            _ => err.to_string(),
        })?;
    let began = std::time::Instant::now();
    while child.try_wait().map_err(|err| err.to_string())?.is_none() {
        if began.elapsed() > AT_MOST {
            let _ = child.kill();
            return Err("tailscale did not answer".to_owned());
        }
        std::thread::sleep(Duration::from_millis(40));
    }
    let out = child.wait_with_output().map_err(|err| err.to_string())?;
    if !out.status.success() {
        let said = String::from_utf8_lossy(&out.stderr);
        return Err(said
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("tailscale failed")
            .trim()
            .to_owned());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Where Tailscale stands here.
pub(crate) fn state() -> TailscaleState {
    match run(&["status", "--json"]) {
        Ok(text) => serde_json::from_str::<Value>(&text)
            .map(|status| state_of(&status))
            .unwrap_or(TailscaleState {
                installed: true,
                ..TailscaleState::default()
            }),
        Err(why) if why == "not installed" => TailscaleState::default(),
        Err(_) => TailscaleState {
            installed: true,
            ..TailscaleState::default()
        },
    }
}

/// Publishes `local`, a loopback port, as HTTPS on the tailnet.
pub(crate) fn serve(local: u16) -> Result<(), String> {
    run(&["serve", "--bg", &format!("--https={HTTPS_PORT}"), &format!("http://127.0.0.1:{local}")])
        .map(|_| ())
        .map_err(|why| {
            if why.to_lowercase().contains("access denied") {
                "Tailscale refused to publish it: run `sudo tailscale set --operator=$USER` once, then turn Remote on again".to_owned()
            } else {
                format!("Tailscale could not publish it: {why}")
            }
        })
}

/// Stops publishing it. Nothing published is not an error.
pub(crate) fn unserve() {
    let _ = run(&["serve", &format!("--https={HTTPS_PORT}"), "off"]);
}

/// The login of whoever is at `address` (`ip:port`) in the tailnet.
pub(crate) fn whois(address: &str) -> Option<String> {
    let text = run(&["whois", "--json", address]).ok()?;
    let found: Value = serde_json::from_str(&text).ok()?;
    found["UserProfile"]["LoginName"]
        .as_str()
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::state_of;

    #[test]
    fn the_status_says_the_name_the_owner_and_whether_https_is_on() {
        let status = json!({
            "BackendState": "Running",
            "Self": { "UserID": 42, "DNSName": "notebook.tail1.ts.net.", "TailscaleIPs": ["fd7a::1", "100.93.1.7"] },
            "User": { "42": { "LoginName": "jo@example.com" } },
            "CertDomains": ["notebook.tail1.ts.net"],
        });
        let state = state_of(&status);
        assert!(state.installed && state.running && state.https);
        assert_eq!(state.name.as_deref(), Some("notebook.tail1.ts.net"));
        assert_eq!(state.login.as_deref(), Some("jo@example.com"));
        assert_eq!(state.ip.as_deref(), Some("100.93.1.7"));

        let off =
            state_of(&json!({ "BackendState": "NeedsLogin", "Self": {}, "CertDomains": null }));
        assert!(!off.running && !off.https && off.login.is_none());
    }
}
