//! The sync key, carried to another machine of the same account through
//! devpit.app's relay, inside the Remote's Noise session.
//!
//! The machine holding the key makes an offer: it goes online on the relay
//! under an id of its own for this one offer, with a Noise key made for it,
//! and shows the person that id, the key's public half and a one-time
//! secret. The person carries the offer to the other machine, which reaches
//! the first through the relay as a device of the same account, holds it to
//! the key the offer names before saying anything, and sends the secret.
//! The first answers with the sync key and goes offline. devpit.app joins
//! the two and reads nothing: the sync key crosses only inside the session.

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use devpit_remote::noise::{hex, unhex32, Keys};
use devpit_remote::{Heard, Link, Relay};
use serde_json::{json, Value};

use crate::key::SyncKey;

/// How long an offer is good for.
pub const GOOD_FOR: Duration = Duration::from_secs(120);
/// Wrong secrets before an offer is withdrawn.
const GUESSES: u32 = 5;
/// The most either end says in one message: a secret, a name, a key.
const MOST: usize = 4 * 1024;
/// How long the handshake, and then each answer, may take.
const WITHIN: Duration = Duration::from_secs(10);
/// How long one read of a socket waits before looking again.
const READ_WAIT: Duration = Duration::from_millis(40);
/// What an offer's text begins with, so the screen knows one when pasted.
pub const PREFIX: &str = "devpit-sync:";

/// What the person carries from the machine with the key to the other.
#[derive(Clone)]
pub struct Offer {
    /// The relay id the offering machine is online under, for this offer.
    pub host: String,
    /// The public half of its Noise key, which the handshake must prove.
    pub machine: [u8; 32],
    pub secret: [u8; 32],
    /// What the offering machine calls itself.
    pub name: String,
}

impl std::fmt::Debug for Offer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The secret stays out of every log.
        write!(f, "Offer {{ host: {}, name: {} }}", self.host, self.name)
    }
}

impl Offer {
    /// A new offer from a machine called `name`, and the Noise key that
    /// answers it.
    pub fn new(name: &str) -> Result<(Offer, Keys), String> {
        let keys = Keys::generate().map_err(|err| err.to_string())?;
        let mut secret = [0u8; 32];
        getrandom::fill(&mut secret).map_err(|_| "the system gave no randomness")?;
        let offer = Offer {
            host: format!("host_{}", ulid::Ulid::generate()),
            machine: keys.public,
            secret,
            name: name.chars().filter(|c| !c.is_control()).take(60).collect(),
        };
        Ok((offer, keys))
    }

    /// The offer as the person copies it.
    pub fn text(&self) -> String {
        format!(
            "{PREFIX}{}.{}.{}.{}",
            self.host,
            hex(&self.machine),
            hex(&self.secret),
            hex(self.name.as_bytes())
        )
    }

    /// An offer from what the person pasted.
    pub fn parse(text: &str) -> Result<Offer, String> {
        let cut = || "that offer is cut short — copy it whole".to_owned();
        let rest = text
            .trim()
            .strip_prefix(PREFIX)
            .ok_or("that is not an offer from another machine's Settings → Account → Sync")?;
        let mut parts = rest.split('.');
        let (Some(host), Some(machine), Some(secret), name, None) = (
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
        ) else {
            return Err(cut());
        };
        let name = name
            .filter(|name| name.len() <= 240 && name.len() % 2 == 0)
            .and_then(|name| {
                (0..name.len() / 2)
                    .map(|at| u8::from_str_radix(&name[at * 2..at * 2 + 2], 16).ok())
                    .collect::<Option<Vec<u8>>>()
            })
            .map(|bytes| {
                String::from_utf8_lossy(&bytes)
                    .chars()
                    .filter(|c| !c.is_control())
                    .collect()
            })
            .unwrap_or_else(|| "the other machine".to_owned());
        match (unhex32(machine), unhex32(secret)) {
            (Some(machine), Some(secret)) if devpit_remote::relay::id_ok(host) => Ok(Offer {
                host: host.to_owned(),
                machine,
                secret,
                name,
            }),
            _ => Err(cut()),
        }
    }
}

/// Compared without saying, by how long it took, how much matched.
fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .fold(0u8, |differs, (x, y)| differs | (x ^ y))
            == 0
}

fn said<S: Read + Write>(link: &mut Link<S>, within: Duration) -> Result<Value, String> {
    let began = Instant::now();
    while began.elapsed() < within {
        match link.receive() {
            Ok(Heard::Message(bytes)) => {
                return serde_json::from_slice(&bytes)
                    .map_err(|_| "the other machine said something that is not JSON".to_owned())
            }
            Ok(Heard::Alive | Heard::Nothing) => {}
            Err(err) => return Err(format!("the other machine left: {err}")),
        }
    }
    Err("the other machine did not answer in time".to_owned())
}

fn say<S: Read + Write>(link: &mut Link<S>, message: &Value) -> Result<(), String> {
    link.send(message.to_string().as_bytes())
        .map_err(|err| format!("the other machine left: {err}"))
}

/// Why an offer's session ended without handing the key over.
#[derive(Debug, PartialEq, Eq)]
pub enum NotHanded {
    /// The machine sent a secret that is not the offer's.
    WrongSecret,
    /// It said nothing usable, or left.
    Gone(String),
}

/// The offering machine's side of one session: the secret first, then the
/// sync key; the name of the machine that took it.
pub fn hand_over<S: Read + Write>(
    link: &mut Link<S>,
    secret: &[u8; 32],
    key: &SyncKey,
) -> Result<String, NotHanded> {
    let asked = said(link, WITHIN).map_err(NotHanded::Gone)?;
    let given = asked["secret"].as_str().and_then(unhex32);
    if asked["t"] != "pair" || !given.is_some_and(|given| same(&given, secret)) {
        let _ = say(
            link,
            &json!({ "t": "refused", "why": "that offer's secret is not this one's — copy the offer again" }),
        );
        return Err(NotHanded::WrongSecret);
    }
    say(link, &json!({ "t": "key", "key": key.to_hex() })).map_err(NotHanded::Gone)?;
    Ok(asked["name"]
        .as_str()
        .unwrap_or("a machine")
        .chars()
        .filter(|c| !c.is_control())
        .take(80)
        .collect())
}

/// The joining machine's side: the secret, and the sync key back.
pub fn take_over<S: Read + Write>(
    link: &mut Link<S>,
    secret: &[u8; 32],
    name: &str,
) -> Result<SyncKey, String> {
    say(
        link,
        &json!({ "t": "pair", "secret": hex(secret), "name": name }),
    )?;
    let answer = said(link, WITHIN)?;
    if answer["t"] == "refused" {
        return Err(answer["why"]
            .as_str()
            .unwrap_or("the other machine refused")
            .chars()
            .take(200)
            .collect());
    }
    answer["key"]
        .as_str()
        .filter(|_| answer["t"] == "key")
        .ok_or_else(|| "the other machine answered without a key".to_owned())
        .and_then(|key| SyncKey::from_hex(key).map_err(|err| err.to_string()))
}

/// Offers `key` on the relay until a machine takes it — its name — or the
/// offer runs out, is guessed at too often, or `stop` is set.
pub fn offer(
    relay: &Relay,
    keys: &Keys,
    secret: &[u8; 32],
    key: &SyncKey,
    until: Instant,
    stop: &AtomicBool,
) -> Result<String, String> {
    let mut control = relay.online().map_err(|err| err.to_string())?;
    let mut wrong = 0;
    let ended = loop {
        if stop.load(Ordering::SeqCst) {
            break Err("the offer was withdrawn".to_owned());
        }
        if Instant::now() > until {
            break Err("the offer ran out — make a new one".to_owned());
        }
        let splice = match control.waiting() {
            Ok(Some(splice)) => splice,
            Ok(None) => continue,
            Err(err) => break Err(err.to_string()),
        };
        let Ok(socket) = relay.splice(&splice) else {
            continue;
        };
        devpit_remote::relay::timeout(&socket, READ_WAIT);
        let Ok(mut link) = Link::answer(socket, keys, relay.host(), MOST, WITHIN) else {
            continue;
        };
        let handed = hand_over(&mut link, secret, key);
        link.close();
        match handed {
            Ok(name) => break Ok(name),
            Err(NotHanded::WrongSecret) => {
                wrong += 1;
                if wrong >= GUESSES {
                    break Err("too many wrong secrets: the offer was withdrawn".to_owned());
                }
            }
            Err(NotHanded::Gone(_)) => {}
        }
    };
    control.close();
    ended
}

/// Reaches the machine `offer` names through the relay, as the account
/// signed in here, and takes the sync key it offers.
pub fn join(origin: &str, token: &str, offer: &Offer, name: &str) -> Result<SyncKey, String> {
    let keys = Keys::generate().map_err(|err| err.to_string())?;
    let socket = Relay::new(origin, token, &offer.host)
        .view()
        .map_err(|err| err.to_string())?;
    let mut link = Link::call(socket, &keys, &offer.host, &offer.machine, MOST, WITHIN).map_err(|err| match err {
        devpit_remote::LinkError::Closed(4404) => format!(
            "{} is not offering any more: make a new offer there, with this computer signed in to the same account",
            offer.name
        ),
        devpit_remote::LinkError::Stranger => format!("what answered is not the {} that made the offer", offer.name),
        other => format!("could not reach {}: {other}", offer.name),
    })?;
    devpit_remote::relay::timeout(link.socket(), READ_WAIT);
    let taken = take_over(&mut link, &offer.secret, name);
    link.close();
    taken
}

#[cfg(test)]
#[path = "pairing_tests.rs"]
mod tests;
