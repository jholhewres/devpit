//! devpit's MCP between two machines, through the relay: the JSON-RPC an MCP
//! client speaks, one message a line on the client's own machine and one
//! sealed message each through the relay.
//!
//! The machine answers; this module only carries. What it answers, and that
//! it answers reads alone, is `devpit_agentapi::remote_mcp`.

use std::io::{Read, Write};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::link::{Heard, Link};

/// The most a client's message may be: the same as a phone's.
pub const MOST_FROM_CLIENT: usize = 256 * 1024;
/// The most a machine's answer may be.
pub const MOST_FROM_MACHINE: usize = 8 * 1024 * 1024;
/// How long the client waits for a line before it reads the link again.
const POLL: Duration = Duration::from_millis(10);

/// A JSON-RPC error for request `id`.
pub fn error(id: &Value, message: &str) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32000, "message": message } })
        .to_string()
}

/// The id of a request, or nothing for a notification or what is not JSON.
pub fn id_of(message: &str) -> Option<Value> {
    serde_json::from_str::<Value>(message)
        .ok()?
        .get("id")
        .filter(|id| !id.is_null())
        .cloned()
}

/// The machine's side: each message the client sends answered by `answer`,
/// until `still` says the client was cut off, it leaves, or it says nothing
/// for `idle`. The caller closes the link, and its keys go with it.
pub fn serve<S: Read + Write>(
    link: &mut Link<S>,
    still: &mut dyn FnMut() -> bool,
    answer: &mut dyn FnMut(&str) -> Option<String>,
    idle: Duration,
) {
    let mut last = Instant::now();
    while still() && last.elapsed() < idle {
        let message = match link.receive() {
            Ok(Heard::Message(bytes)) => bytes,
            Ok(Heard::Alive) => {
                last = Instant::now();
                continue;
            }
            Ok(Heard::Nothing) => continue,
            Err(_) => return,
        };
        last = Instant::now();
        let Ok(line) = String::from_utf8(message) else {
            return;
        };
        let Some(mut said) = answer(&line) else {
            continue;
        };
        // Past what the client keeps, the answer would end the session.
        if said.len() > MOST_FROM_MACHINE {
            said = error(
                &id_of(&line).unwrap_or(Value::Null),
                "that answer is too big to send",
            );
        }
        if link.send(said.as_bytes()).is_err() {
            return;
        }
    }
}

/// The client's side: each line of the MCP host's sent to the machine, each
/// answer handed to `out`, until the host hangs up or `out` fails.
///
/// Connected when there is something to send, and again after the link
/// drops. A request the machine had not answered when it dropped, or that
/// could not be sent, is answered here with why, so the host is never left
/// waiting.
pub fn forward<S: Read + Write>(
    lines: &Receiver<String>,
    connect: &mut dyn FnMut() -> Result<Link<S>, String>,
    out: &mut dyn FnMut(&str) -> bool,
) {
    let mut link: Option<Link<S>> = None;
    let mut asked: Vec<Value> = Vec::new();
    loop {
        let line = if link.is_some() {
            match lines.recv_timeout(POLL) {
                Ok(line) => Some(line),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match lines.recv() {
                Ok(line) => Some(line),
                Err(_) => break,
            }
        };
        if let Some(line) = line {
            let id = id_of(&line);
            let sent = send(&mut link, connect, &line);
            match (sent, id) {
                (Ok(()), Some(id)) => asked.push(id),
                (Err(why), Some(id)) if !out(&error(&id, &why)) => break,
                _ => {}
            }
        }
        let Some(open) = link.as_mut() else {
            continue;
        };
        let heard = match open.receive() {
            Ok(Heard::Message(bytes)) => bytes,
            Ok(Heard::Alive | Heard::Nothing) => continue,
            Err(_) => {
                link = None;
                let why = "the connection to the machine ended before it answered — ask again";
                if !answer_all(&mut asked, why, out) {
                    break;
                }
                continue;
            }
        };
        let Ok(said) = serde_json::from_slice::<Value>(&heard) else {
            continue;
        };
        // The machine's word for a client it will not serve, before it hangs up.
        if said["t"] == "refused" {
            let why = said["why"]
                .as_str()
                .unwrap_or("the machine refused this client");
            if let Some(gone) = link.take() {
                gone.close();
            }
            if !answer_all(&mut asked, why, out) {
                break;
            }
            continue;
        }
        if let Some(id) = said.get("id") {
            asked.retain(|one| one != id);
        }
        // Written as one line again: what came is JSON, and nothing else is.
        if !out(&said.to_string()) {
            break;
        }
    }
    if let Some(open) = link {
        open.close();
    }
}

/// `line` to the machine, connecting first when there is no link.
fn send<S: Read + Write>(
    link: &mut Option<Link<S>>,
    connect: &mut dyn FnMut() -> Result<Link<S>, String>,
    line: &str,
) -> Result<(), String> {
    if line.len() > MOST_FROM_CLIENT {
        return Err(format!(
            "a message is at most {} KiB",
            MOST_FROM_CLIENT / 1024
        ));
    }
    if link.is_none() {
        *link = Some(connect()?);
    }
    let open = link.as_mut().ok_or("not connected")?;
    if open.send(line.as_bytes()).is_err() {
        *link = None;
        return Err("the connection to the machine ended — ask again".to_owned());
    }
    Ok(())
}

/// Every request still waiting, answered with `why`.
fn answer_all(asked: &mut Vec<Value>, why: &str, out: &mut dyn FnMut(&str) -> bool) -> bool {
    asked.drain(..).all(|id| out(&error(&id, why)))
}

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod tests;
