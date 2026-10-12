//! Remote through devpit.app's relay, for a device away from the tailnet.
//!
//! This machine keeps one connection open to the relay, as the person's
//! devpit account, and opens one more for each device that asks for it. In
//! each, a Noise handshake proves this machine's key to the device and the
//! device's key to this machine; what follows is sealed for the two of them,
//! and devpit.app passes it along without a key to open it. Nothing listens
//! here: every connection goes out.
//!
//! A device is known by its key. One not known yet must send, first and
//! inside the session, the one-time secret the pairing QR code carried.
//! Forgetting a device ends its connections, and with them their keys; the
//! relay closes its side the moment this one does.

use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use devpit_remote::noise::{hex, unhex32, Keys};
use devpit_remote::{Link, Relay, RelayError};
use devpit_rpc::{RemoteIn, RemoteRelay};
use serde::{Deserialize, Serialize};
use tungstenite::stream::MaybeTlsStream;

use crate::remote_conn::{first, refuse, Heard, Wire, HELLO_WITHIN, MOST_MESSAGE};
use crate::remote_devices::{Device, Devices};

/// The longest wait between two tries at reaching the relay.
const LONGEST_WAIT: Duration = Duration::from_secs(60);

/// This machine on the relay: its id and its key, kept so a paired device
/// finds it again after a restart.
#[derive(Serialize, Deserialize)]
struct Identity {
    host: String,
    secret: String,
    public: String,
}

fn file(root: &Path) -> PathBuf {
    root.join("remote-relay.json")
}

/// This machine's id and key on the relay, made the first time.
pub(crate) fn identity(root: &Path) -> Result<(String, Keys), String> {
    let kept = std::fs::read_to_string(file(root))
        .ok()
        .and_then(|text| serde_json::from_str::<Identity>(&text).ok())
        .and_then(|one| {
            let keys = Keys {
                secret: unhex32(&one.secret)?,
                public: unhex32(&one.public)?,
            };
            devpit_remote::relay::id_ok(&one.host).then_some((one.host, keys))
        });
    if let Some(kept) = kept {
        return Ok(kept);
    }
    let keys = Keys::generate().map_err(|err| err.to_string())?;
    let host = format!("host_{}", ulid::Ulid::generate());
    let text = serde_json::to_string_pretty(&Identity {
        host: host.clone(),
        secret: hex(&keys.secret),
        public: hex(&keys.public),
    })
    .map_err(|err| err.to_string())?;
    devpit_core::home::write_private(&file(root), text.as_bytes())
        .map_err(|err| format!("this machine's key could not be kept: {err}"))?;
    Ok((host, keys))
}

static RUNNING: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);
static STATE: Mutex<(bool, Option<String>)> = Mutex::new((false, None));

/// The relay, as the settings show it.
pub(crate) fn state() -> RemoteRelay {
    let (online, problem) = STATE.lock().map(|state| state.clone()).unwrap_or_default();
    RemoteRelay {
        signed_in: crate::account::token::read().is_some(),
        online,
        problem,
    }
}

fn set_state(app: &tauri::AppHandle, online: bool, problem: Option<String>) {
    let changed = STATE.lock().is_ok_and(|mut state| {
        let next = (online, problem);
        let changed = *state != next;
        *state = next;
        changed
    });
    if changed {
        crate::remote::changed(app);
    }
}

pub(crate) fn running() -> bool {
    RUNNING.lock().is_ok_and(|running| running.is_some())
}

/// Connects this machine to the relay, or says why it cannot. Connected
/// already is fine.
pub(crate) fn start(app: &tauri::AppHandle, root: PathBuf) -> Result<(), String> {
    let mut running = RUNNING.lock().map_err(|_| "the relay is stuck")?;
    if running.is_some() {
        return Ok(());
    }
    if crate::account::token::read().is_none() {
        return Err(
            "Sign in to your devpit account first (Settings → Account): the relay knows this machine by it"
                .to_owned(),
        );
    }
    let (host, keys) = identity(&root)?;
    let stop = Arc::new(AtomicBool::new(false));
    let app = app.clone();
    let held = stop.clone();
    std::thread::spawn(move || hold(app, root, host, keys, held));
    *running = Some(stop);
    Ok(())
}

pub(crate) fn stop(app: &tauri::AppHandle) {
    let was = RUNNING.lock().ok().and_then(|mut running| running.take());
    if let Some(stop) = was {
        stop.store(true, Ordering::SeqCst);
    }
    set_state(app, false, None);
}

/// The wait before try number `tries`: doubling to a minute, with jitter so
/// a relay restart is not answered by every machine at once.
pub(crate) fn wait_after(tries: u32, jitter_ms: u64) -> Duration {
    let doubled = Duration::from_secs(1u64 << tries.min(6));
    doubled.min(LONGEST_WAIT) + Duration::from_millis(jitter_ms % 1000)
}

fn jitter() -> u64 {
    let mut bytes = [0u8; 8];
    let _ = getrandom::fill(&mut bytes);
    u64::from_le_bytes(bytes)
}

/// Keeps this machine online on the relay until stopped.
fn hold(app: tauri::AppHandle, root: PathBuf, host: String, keys: Keys, stop: Arc<AtomicBool>) {
    let mut tries = 0u32;
    while !stop.load(Ordering::SeqCst) {
        // Read again each time: signing in again gives a new session.
        match crate::account::token::read() {
            None => set_state(&app, false, Some(RelayError::SignedOut.to_string())),
            Some(token) => {
                let relay = Relay::new(&crate::account::origin(), &token, &host);
                match relay.online() {
                    Ok(control) => {
                        tries = 0;
                        set_state(&app, true, None);
                        let why = serve(&app, &root, &relay, &keys, control, &stop);
                        set_state(&app, false, why);
                    }
                    Err(err) => set_state(&app, false, Some(err.to_string())),
                }
            }
        }
        let wait = wait_after(tries, jitter());
        tries = tries.saturating_add(1);
        let began = std::time::Instant::now();
        while began.elapsed() < wait && !stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(200));
        }
    }
}

/// The relay's word that devices are waiting, each joined on a thread of its
/// own, until the connection drops or Remote is turned off; why it dropped.
fn serve(
    app: &tauri::AppHandle,
    root: &Path,
    relay: &Relay,
    keys: &Keys,
    mut control: devpit_remote::Control,
    stop: &AtomicBool,
) -> Option<String> {
    loop {
        if stop.load(Ordering::SeqCst) {
            control.close();
            return None;
        }
        match control.waiting() {
            Ok(Some(splice)) => {
                let (app, root, relay, keys) =
                    (app.clone(), root.to_owned(), relay.clone(), keys.clone());
                std::thread::spawn(move || join(app, root, relay, keys, &splice));
            }
            Ok(None) => {}
            Err(err) => return Some(err.to_string()),
        }
    }
}

/// A device's connection through the relay: the handshake, who it is, and
/// then its requests, like any viewer's.
fn join(app: tauri::AppHandle, root: PathBuf, relay: Relay, keys: Keys, splice: &str) {
    let Ok(socket) = relay.splice(splice) else {
        return;
    };
    devpit_remote::relay::timeout(&socket, Duration::from_millis(40));
    let Ok(link) = Link::answer(socket, &keys, relay.host(), MOST_MESSAGE, HELLO_WITHIN) else {
        return;
    };
    let peer = link.peer();
    let mut wire = Relayed(Some(link));
    let now = crate::remote::now();
    let mut devices = Devices::read(&root);
    let identified = identify(
        &mut devices,
        &peer,
        || first(&mut wire),
        |secret| crate::remote_pairing::take_secret(secret, now),
        now,
    );
    let device = match identified {
        Ok((device, fresh)) => {
            if devices.write(&root).is_err() && fresh {
                refuse(&mut wire, "the device could not be kept");
                return wire.end();
            }
            if fresh {
                crate::remote::note(&root, &device, "paired through devpit.app");
                crate::remote::changed(&app);
            }
            device
        }
        Err(why) => {
            refuse(&mut wire, &why);
            return wire.end();
        }
    };
    crate::remote_conn::serve(app, wire, root, device);
}

/// Who the device whose handshake proved `peer` is: one paired already, or
/// one pairing now with the secret its QR code carried as its first message
/// — and whether it is new. Anything else is refused, saying why.
pub(crate) fn identify(
    devices: &mut Devices,
    peer: &[u8; 32],
    first: impl FnOnce() -> Option<RemoteIn>,
    take_secret: impl FnOnce(&str) -> bool,
    now: f64,
) -> Result<(Device, bool), String> {
    if let Some(device) = devices.by_key(peer).cloned() {
        devices.seen(&device.id, now);
        return Ok((device, false));
    }
    let Some(RemoteIn::Pair { secret, name }) = first() else {
        return Err("this device is not paired with this machine — pair it again".to_owned());
    };
    if !take_secret(&secret) {
        return Err(
            "that pairing code ran out or was used already — show a new one on the machine"
                .to_owned(),
        );
    }
    devices
        .pair_key(&name, peer, now)
        .map(|device| (device, true))
}

/// A viewer through the relay: JSON sealed in Noise frames.
struct Relayed(Option<Link<MaybeTlsStream<TcpStream>>>);

impl Wire for Relayed {
    fn hear(&mut self) -> Result<Heard, ()> {
        let link = self.0.as_mut().ok_or(())?;
        match link.receive().map_err(|_| ())? {
            devpit_remote::Heard::Message(bytes) => {
                String::from_utf8(bytes).map(Heard::Text).map_err(|_| ())
            }
            devpit_remote::Heard::Alive => Ok(Heard::Alive),
            devpit_remote::Heard::Nothing => Ok(Heard::Nothing),
        }
    }

    fn say(&mut self, text: &str) {
        let gone = self
            .0
            .as_mut()
            .is_some_and(|link| link.send(text.as_bytes()).is_err());
        if gone {
            self.end();
        }
    }

    fn end(&mut self) {
        // The session's keys go with the link.
        if let Some(link) = self.0.take() {
            link.close();
        }
    }
}

/// The link a pairing QR code opens: devpit.app's viewer, with this machine's
/// id, key and the one-time `secret` in the fragment, which no browser sends.
pub(crate) fn pairing_url(root: &Path, secret: &str, name: &str) -> Result<String, String> {
    let (host, keys) = identity(root)?;
    Ok(format!(
        "{}/remote#relay={host}&key={}&secret={secret}&name={}",
        crate::account::origin().trim_end_matches('/'),
        hex(&keys.public),
        fragment_value(name)
    ))
}

/// `text` as a value in a fragment: letters, digits and `-_.~` as they are,
/// every other byte escaped.
pub(crate) fn fragment_value(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

#[cfg(test)]
#[path = "remote_relay_tests.rs"]
mod tests;
