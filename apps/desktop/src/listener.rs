//! The other end of the hooks: a loopback listener the agent posts to.
//!
//! Small on purpose. It answers one route, on 127.0.0.1, on a port the OS
//! picks, and writes that address to a file the hook reads on every invocation
//! — so a session that outlived a restart finds the new port instead of posting
//! into a dead one.
//!
//! Loopback and nothing else. This receives a payload and turns it into
//! something the window draws; binding it anywhere reachable would be a way in
//! to a process that runs terminals.

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::Path;

use quockpit_agentcli::{read_hook, Event, Happening};
use tauri::{AppHandle, Emitter};

/// A payload bigger than this is not one of ours.
///
/// A `Stop` carries the last thing the agent said, which can be long, so the
/// ceiling is generous — but a ceiling there is, because this reads from a
/// socket and an unbounded read is a way to spend all the memory on the box.
const MOST_BYTES: usize = 256 * 1024;

/// Starts listening, and writes the address where the hook will look for it.
///
/// Failure is not fatal: hooks are how the board hears about work as it
/// happens, and without them it still polls. A window that refuses to open
/// because a port was busy would be worse than one that is a little less live.
pub fn start(app: AppHandle, root: &Path) {
    let listener = match TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("no hook listener, the board will poll instead: {err}");
            return;
        }
    };

    let Ok(address) = listener.local_addr() else {
        return;
    };
    let endpoint = quockpit_agentcli::endpoint_file(root);
    if let Some(parent) = endpoint.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(err) = std::fs::write(&endpoint, format!("http://{address}/hook")) {
        eprintln!("could not publish the hook endpoint: {err}");
        return;
    }

    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let app = app.clone();
            // One thread per post, and they are short: a hook that has to wait
            // for the one before it is a hook holding up the agent that sent it.
            std::thread::spawn(move || serve(app, stream));
        }
    });
}

fn serve(app: AppHandle, mut stream: TcpStream) {
    let Some(body) = read_request(&mut stream) else {
        let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\ncontent-length: 0\r\n\r\n");
        return;
    };

    // Answered before the payload is looked at. The agent is waiting on this
    // reply with a 1.5 second budget, and nothing it says changes what we
    // reply with.
    let _ = stream.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n");
    let _ = stream.flush();

    if let Some(happening) = read_hook(&body) {
        let _ = app.emit("agent:happening", describe(&happening));
    }
}

/// What the window is told, in the words it draws.
fn describe(happening: &Happening) -> (String, String) {
    let said = match &happening.event {
        Event::Using { tool } => format!("running {tool}"),
        Event::Used { tool } => format!("finished {tool}"),
        Event::Stopped { said } => said
            .clone()
            .unwrap_or_else(|| "finished the turn".to_owned()),
        Event::Waiting => "waiting on you".to_owned(),
    };
    (happening.session_id.clone(), said)
}

/// The body of a POST, or nothing.
///
/// Written by hand rather than with an HTTP crate: one route, one method, one
/// caller on loopback. A dependency here would be several thousand lines to
/// parse a request this already refuses to over-read.
fn read_request(stream: &mut TcpStream) -> Option<String> {
    read_post(BufReader::new(stream.try_clone().ok()?))
}

/// The same, from anything that yields bytes.
///
/// Split off the socket so a test can hand it a request instead of opening a
/// port. The ceiling below is the only thing between a `content-length`
/// somebody else wrote and an allocation of exactly that size, and a rule a
/// test cannot call is a rule the test cannot guard.
fn read_post(mut reader: impl BufRead) -> Option<String> {
    let mut length = 0usize;

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = value.trim().parse().ok()?;
        }
    }

    if length == 0 || length > MOST_BYTES {
        return None;
    }
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).ok()?;
    String::from_utf8(body).ok()
}

#[cfg(test)]
#[path = "listener_tests.rs"]
mod tests;
