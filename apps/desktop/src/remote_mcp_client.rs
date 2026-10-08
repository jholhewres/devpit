//! `devpit mcp --remote`: devpit's MCP on another machine, for any MCP client
//! on this one, through devpit.app's relay and encrypted end to end.
//!
//! `devpit mcp --remote pair '<link>'` pairs this machine once, with the link
//! Settings → Remote → Pair an MCP client shows on the other; this end's key
//! is kept in this machine's devpit home, mode 600. `devpit mcp --remote` is
//! then a stdio MCP server that seals every message for that machine alone,
//! which answers what it allows from outside — reads. `devpit mcp --remote
//! forget` drops the key here.

use std::collections::HashMap;
use std::io::{BufRead as _, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::time::{Duration, Instant};

use devpit_remote::mcp::{MOST_FROM_CLIENT, MOST_FROM_MACHINE};
use devpit_remote::noise::{hex, unhex32, Keys};
use devpit_remote::{Heard, Link, LinkError, Relay};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::remote_relay::RelayLink;

/// How long the handshake through the relay may take.
const HANDSHAKE_WITHIN: Duration = Duration::from_secs(10);
/// How long the machine has to confirm a pairing.
const PAIRED_WITHIN: Duration = Duration::from_secs(15);
/// How long one read of the link waits before the next line is looked at.
const READ_WAIT: Duration = Duration::from_millis(40);

const NOT_PAIRED: &str = "this machine is not paired with a devpit: run `devpit mcp --remote pair '<link>'` with the link Settings → Remote → Pair an MCP client shows on it";

const USAGE: &str =
    "usage: devpit mcp --remote               serve devpit's MCP of the paired machine on stdio
       devpit mcp --remote pair '<link>'  pair with the link Settings → Remote shows there
       devpit mcp --remote forget         drop this machine's key for it";

/// The machine this one is paired with, and this end's key for it.
#[derive(Serialize, Deserialize)]
struct Paired {
    host: String,
    /// The machine's public key, which every handshake must prove.
    machine: String,
    name: String,
    secret: String,
    public: String,
}

/// What a pairing link for an MCP client carries.
struct Offer {
    host: String,
    machine: [u8; 32],
    secret: String,
    name: String,
}

fn file(root: &Path) -> PathBuf {
    root.join("remote-mcp.json")
}

pub(crate) fn run(root: &Path, args: &[String]) -> i32 {
    match (args.first().map(String::as_str), args.get(1)) {
        (None, _) => serve(root),
        (Some("pair"), Some(link)) => match pair(root, link) {
            Ok(name) => {
                let exe = crate::agent_reach::exe()
                    .map(|exe| exe.display().to_string())
                    .unwrap_or_else(|| "devpit".to_owned());
                println!("Paired with {name}: its devpit is read from here, read-only.");
                println!("Add it to an MCP client here, for example:");
                println!("  claude mcp add devpit-remote -- {exe} mcp --remote");
                0
            }
            Err(why) => {
                eprintln!("devpit: {why}");
                1
            }
        },
        (Some("forget"), None) => {
            let _ = std::fs::remove_file(file(root));
            println!("This machine's key for it is gone. Forget it on the machine too: Settings → Remote.");
            0
        }
        _ => {
            eprintln!("{USAGE}");
            2
        }
    }
}

/// The offer in `link`, if it is one for an MCP client of a machine signed in
/// where this one signs in: the account's session goes nowhere else.
fn offer_of(link: &str, origin: &str) -> Result<Offer, String> {
    let (page, fragment) = link
        .trim()
        .split_once('#')
        .ok_or("that is not a pairing link")?;
    let page = page.trim_end_matches('/');
    let ours = format!("{}/remote", origin.trim_end_matches('/'));
    if page != ours {
        return Err(format!(
            "that link is for {page}, and this devpit signs in at {origin}: its session would go there"
        ));
    }
    let fields: HashMap<&str, String> = fragment
        .split('&')
        .filter_map(|field| field.split_once('='))
        .map(|(name, value)| (name, decoded(value)))
        .collect();
    let field = |name: &str| fields.get(name).map(String::as_str).unwrap_or_default();
    if field("for") != "mcp" {
        return Err(
            "that link pairs a phone — on the machine, use Settings → Remote → Pair an MCP client"
                .to_owned(),
        );
    }
    let host = field("relay");
    let secret = field("secret");
    let machine = unhex32(field("key"));
    match machine {
        Some(machine) if devpit_remote::relay::id_ok(host) && unhex32(secret).is_some() => {
            Ok(Offer {
                host: host.to_owned(),
                machine,
                secret: secret.to_owned(),
                name: Some(field("name"))
                    .filter(|name| !name.is_empty())
                    .unwrap_or("the machine")
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(60)
                    .collect(),
            })
        }
        _ => Err("that pairing link is cut short — copy it whole".to_owned()),
    }
}

/// A fragment's value, its `%XX` escapes undone.
fn decoded(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let escaped = (bytes[at] == b'%')
            .then(|| value.get(at + 1..at + 3))
            .flatten()
            .and_then(|digits| u8::from_str_radix(digits, 16).ok());
        match escaped {
            Some(byte) => {
                out.push(byte);
                at += 3;
            }
            None => {
                out.push(bytes[at]);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Pairs this machine with the one `link` names; that machine's name.
fn pair(root: &Path, link: &str) -> Result<String, String> {
    let origin = crate::account::origin();
    let offer = offer_of(link, &origin)?;
    let token = match crate::account::token::read() {
        Some(token) => token,
        None => sign_in()?,
    };
    let keys = Keys::generate().map_err(|err| err.to_string())?;
    let mut link = reach(
        &origin,
        &token,
        &offer.host,
        &offer.machine,
        &keys,
        &offer.name,
    )?;
    let name = format!("devpit MCP on {}", crate::remote_conn::machine_name());
    let pairing = json!({ "t": "pair", "secret": offer.secret, "name": name });
    // A ping right after: answered only once the machine serves this key MCP.
    let ping = json!({ "jsonrpc": "2.0", "id": "pair", "method": "ping" });
    let sent = link
        .send(pairing.to_string().as_bytes())
        .and_then(|()| link.send(ping.to_string().as_bytes()));
    let answer = sent
        .map_err(|err| why(&err, &offer.name))
        .and_then(|()| first(&mut link, &offer.name));
    link.close();
    let answer = answer?;
    if answer["t"] == "refused" {
        return Err(answer["why"]
            .as_str()
            .unwrap_or("the machine refused the pairing")
            .to_owned());
    }
    if answer["id"] != "pair" || answer.get("result").is_none() {
        return Err("the machine paired this as a phone — forget it there, and use Settings → Remote → Pair an MCP client".to_owned());
    }
    let paired = Paired {
        host: offer.host,
        machine: hex(&offer.machine),
        name: offer.name.clone(),
        secret: hex(&keys.secret),
        public: hex(&keys.public),
    };
    keep(root, &paired).map_err(|err| {
        format!("paired, but this machine's key could not be kept ({err}) — forget it on the machine and pair again")
    })?;
    Ok(offer.name)
}

/// The pairing, in a file only this user can read: it holds this end's key.
fn keep(root: &Path, paired: &Paired) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(paired).map_err(std::io::Error::other)?;
    devpit_core::home::write_private(&file(root), text.as_bytes())
}

/// The machine's first message, within [`PAIRED_WITHIN`].
fn first(link: &mut RelayLink, name: &str) -> Result<Value, String> {
    let began = Instant::now();
    while began.elapsed() < PAIRED_WITHIN {
        match link.receive() {
            Ok(Heard::Message(bytes)) => {
                return serde_json::from_slice(&bytes)
                    .map_err(|_| format!("{name} answered something that is not JSON"))
            }
            Ok(Heard::Alive | Heard::Nothing) => {}
            Err(err) => return Err(why(&err, name)),
        }
    }
    Err(format!("{name} did not confirm the pairing in time"))
}

/// Signs this machine in to the devpit account the way Settings → Account
/// does: a code to approve in a browser, here or on any other device.
fn sign_in() -> Result<String, String> {
    use devpit_rpc::SignInState;
    let started = tauri::async_runtime::block_on(crate::account::account_sign_in())
        .map_err(|err| err.message)?;
    eprintln!(
        "Sign this machine in to devpit.app, as the same account as the machine: open {} and enter {}",
        started.verify_url, started.user_code
    );
    let until = Instant::now() + Duration::from_secs(started.expires_in_seconds.into());
    let every = Duration::from_secs(started.interval_seconds.max(1).into());
    while Instant::now() < until {
        std::thread::sleep(every);
        match tauri::async_runtime::block_on(crate::account::account_poll())
            .map_err(|err| err.message)?
        {
            SignInState::Waiting => {}
            SignInState::Signed { .. } => {
                return crate::account::token::read()
                    .ok_or("signed in, and no session was kept".to_owned())
            }
            SignInState::Expired => break,
        }
    }
    Err("the sign-in ran out — run pair again".to_owned())
}

/// A session with the machine, through the relay, proven to be the one paired.
fn reach(
    origin: &str,
    token: &str,
    host: &str,
    machine: &[u8; 32],
    keys: &Keys,
    name: &str,
) -> Result<RelayLink, String> {
    let socket = Relay::new(origin, token, host)
        .view()
        .map_err(|err| err.to_string())?;
    let link = Link::call(
        socket,
        keys,
        host,
        machine,
        MOST_FROM_MACHINE,
        HANDSHAKE_WITHIN,
    )
    .map_err(|err| why(&err, name))?;
    devpit_remote::relay::timeout(link.socket(), READ_WAIT);
    Ok(link)
}

/// Why the machine could not be reached, as something to do about it.
fn why(err: &LinkError, name: &str) -> String {
    match err {
        LinkError::Closed(4401) => "devpit.app asked this machine to sign in again — sign in, then ask again".to_owned(),
        LinkError::Closed(4402) => "reaching another machine through devpit.app is part of devpit Plus, which this account does not have".to_owned(),
        LinkError::Closed(4404) => format!("{name} is not connected to devpit.app: devpit must be open there with Remote on through devpit.app, and this machine signed in to the same account"),
        LinkError::Closed(4429) => "devpit.app has too many connections of this account open".to_owned(),
        LinkError::Closed(4504) => format!("{name} did not answer through devpit.app in time"),
        LinkError::Stranger => format!("what answered is not the {name} this machine paired with"),
        other => format!("could not reach {name}: {other}"),
    }
}

/// The paired machine's MCP on stdio, until the client hangs up.
fn serve(root: &Path) -> i32 {
    if read(root).is_none() {
        eprintln!("devpit: {NOT_PAIRED}");
    }
    let (said, lines) = channel();
    std::thread::spawn(move || read_lines(&said));
    let origin = crate::account::origin();
    let mut connect = || -> Result<RelayLink, String> {
        // Read each time: pairing again, or signing in again, is picked up.
        let paired = read(root).ok_or(NOT_PAIRED)?;
        let token = crate::account::token::read()
            .ok_or("this machine is signed out of devpit.app — sign in again, as the same account as the machine")?;
        let keys = Keys {
            secret: unhex32(&paired.secret).ok_or(NOT_PAIRED)?,
            public: unhex32(&paired.public).ok_or(NOT_PAIRED)?,
        };
        let machine = unhex32(&paired.machine).ok_or(NOT_PAIRED)?;
        reach(&origin, &token, &paired.host, &machine, &keys, &paired.name)
    };
    let mut out = |line: &str| {
        let mut stdout = std::io::stdout().lock();
        writeln!(stdout, "{line}")
            .and_then(|()| stdout.flush())
            .is_ok()
    };
    devpit_remote::mcp::forward(&lines, &mut connect, &mut out);
    0
}

fn read(root: &Path) -> Option<Paired> {
    let text = std::fs::read_to_string(file(root)).ok()?;
    serde_json::from_str(&text).ok()
}

/// The MCP client's lines, each held to the ceiling a message has before it
/// is kept: a longer one is read past and dropped.
fn read_lines(said: &Sender<String>) {
    let mut stdin = std::io::stdin().lock();
    let most = MOST_FROM_CLIENT as u64 + 1;
    loop {
        let mut line = Vec::new();
        match (&mut stdin).take(most).read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        if !line.ends_with(b"\n") && line.len() as u64 == most {
            eprintln!(
                "devpit: a message past {} KiB was dropped",
                MOST_FROM_CLIENT / 1024
            );
            let mut rest = Vec::new();
            while (&mut stdin)
                .take(most)
                .read_until(b'\n', &mut rest)
                .is_ok_and(|n| n > 0)
                && !rest.ends_with(b"\n")
            {
                rest.clear();
            }
            continue;
        }
        let text = String::from_utf8_lossy(&line).trim().to_owned();
        if !text.is_empty() && said.send(text).is_err() {
            return;
        }
    }
}

#[cfg(test)]
#[path = "remote_mcp_client_tests.rs"]
mod tests;
