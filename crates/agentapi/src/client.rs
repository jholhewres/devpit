//! One question to the running app, over loopback.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;

use serde_json::{json, Value};

/// How long a question may take. Short, because an agent is waiting on the
/// answer; a devpit that has gone away should cost it a moment, not a hang.
const WAIT: Duration = Duration::from_secs(5);

/// Asks the devpit keeping its state in `root`, from `cwd`, and answers with
/// what it said or why it could not.
pub fn ask(root: &Path, method: &str, params: Value, cwd: &Path) -> Result<Value, String> {
    let (address, secret) = door(root)?;
    // Who is asking, when devpit started the agent and said so; a comment is
    // signed with it. The app accepts only an agent it knows.
    let author = std::env::var("DEVPIT_AGENT_ID").unwrap_or_default();
    let body = json!({ "method": method, "params": params, "cwd": cwd.display().to_string(), "author": author }).to_string();
    let mut stream = TcpStream::connect(&address).map_err(|_| closed())?;
    let _ = stream.set_read_timeout(Some(WAIT));
    let _ = stream.set_write_timeout(Some(WAIT));
    let request = format!(
        "POST /agent HTTP/1.1\r\nhost: {address}\r\ncontent-type: application/json\r\n{secret}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).map_err(|_| closed())?;
    let reply = read_reply(stream)?;
    let parsed: Value = serde_json::from_str(&reply)
        .map_err(|_| "devpit answered something that is not JSON".to_owned())?;
    match (parsed.get("ok"), parsed.get("error")) {
        (Some(ok), _) => Ok(ok.clone()),
        (_, Some(Value::String(why))) => Err(why.clone()),
        _ => Err("devpit answered without saying how it went".to_owned()),
    }
}

fn closed() -> String {
    "devpit is not open: start it, then ask again".to_owned()
}

/// Where the app listens and the header that gets a post in, read from the
/// files the app wrote when it started.
pub(crate) fn door(root: &Path) -> Result<(String, String), String> {
    let endpoint = std::fs::read_to_string(root.join("hook-endpoint")).map_err(|_| closed())?;
    let secret = std::fs::read_to_string(root.join("hook-auth")).map_err(|_| closed())?;
    let address = address_of(endpoint.trim()).ok_or_else(closed)?;
    Ok((address, secret.trim().to_owned()))
}

/// `host:port` out of `http://host:port/hook`.
pub(crate) fn address_of(endpoint: &str) -> Option<String> {
    let rest = endpoint.strip_prefix("http://")?;
    let address = rest.split('/').next()?;
    (!address.is_empty()).then(|| address.to_owned())
}

/// The body of an HTTP reply, refusing anything but a 200.
fn read_reply(stream: TcpStream) -> Result<String, String> {
    let mut reader = BufReader::new(stream);
    let mut status = String::new();
    reader.read_line(&mut status).map_err(|_| closed())?;
    if status.split_whitespace().nth(1) != Some("200") {
        return Err(match status.split_whitespace().nth(1) {
            Some("401") => "devpit refused the question: its secret changed, which means it restarted — ask again".to_owned(),
            _ => format!("devpit answered `{}`", status.trim()),
        });
    }
    let mut length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).map_err(|_| closed())? == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse::<usize>().ok();
            }
        }
    }
    let mut body = String::new();
    match length {
        Some(length) => {
            let mut bytes = vec![0u8; length];
            reader.read_exact(&mut bytes).map_err(|_| closed())?;
            body = String::from_utf8_lossy(&bytes).into_owned();
        }
        None => {
            reader.read_to_string(&mut body).map_err(|_| closed())?;
        }
    }
    Ok(body)
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
