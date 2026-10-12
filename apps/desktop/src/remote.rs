//! Remote: this machine, reached from the person's other devices — over
//! their tailnet, or from anywhere through devpit.app's relay, encrypted end
//! to end — terminals watched and typed into, the board, the questions
//! agents wait on, the chats. Off unless turned on, here.
//!
//! The settings' half: turning it on and off, which way devices come in,
//! pairing a device, what each paired device may do, and a log of who
//! connected and did what — never what was on screen or typed.

use std::io::Write as _;
use std::path::Path;

use devpit_core::{preference, Store};
use devpit_rpc::{
    ErrorCode, RemoteDevice, RemotePairing, RemoteShapes, RemoteVia, RemoteView, RpcError,
};
use tauri::Emitter as _;

use crate::remote_devices::{Device, Devices};

pub(crate) fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs_f64())
        .unwrap_or_default()
}

/// A line in the remote log: when, which device, what — no content.
pub(crate) fn note(root: &Path, device: &Device, what: &str) {
    let line =
        serde_json::json!({ "at": now(), "device": device.name, "id": device.id, "what": what });
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("remote-log.jsonl"))
    {
        let _ = writeln!(file, "{line}");
    }
}

/// The latest `most` entries of the Remote's log, the latest first.
pub(crate) fn activity_in(text: &str, most: usize) -> Vec<devpit_rpc::RemoteActivity> {
    text.lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|one| {
            Some(devpit_rpc::RemoteActivity {
                at: one.get("at")?.as_f64()?,
                device: one.get("device")?.as_str()?.to_owned(),
                what: one.get("what")?.as_str()?.to_owned(),
            })
        })
        .take(most)
        .collect()
}

/// `remote.activity` — what paired devices did lately, never what they saw.
#[tauri::command]
#[specta::specta]
pub async fn remote_activity() -> Result<devpit_rpc::RemoteActivities, devpit_rpc::RpcError> {
    crate::off_main::blocking(|| {
        let path = devpit_core::Store::root()?.join("remote-log.jsonl");
        // The tail only: the log grows for as long as the Remote is used.
        let mut tail = Vec::new();
        if let Ok(mut file) = std::fs::File::open(&path) {
            use std::io::{Read, Seek, SeekFrom};
            let size = file.metadata().map(|meta| meta.len()).unwrap_or(0);
            let _ = file.seek(SeekFrom::Start(size.saturating_sub(256 * 1024)));
            let _ = file.take(256 * 1024).read_to_end(&mut tail);
        }
        Ok(devpit_rpc::RemoteActivities {
            entries: activity_in(&String::from_utf8_lossy(&tail), 30),
        })
    })
    .await
}

/// The settings read again: a device paired, or one connected.
pub(crate) fn changed(app: &tauri::AppHandle) {
    let _ = app.emit("remote:changed", ());
}

fn view(root: &Path, store: &Store, problem: Option<String>) -> RemoteView {
    let connected = crate::remote_hub::connected();
    let via = via(store);
    RemoteView {
        enabled: enabled(store),
        via,
        relay: crate::remote_relay::state(),
        // Tailscale is asked only when it is the way in: a `tailscale`
        // command is seconds on a machine without it.
        tailscale: match via {
            RemoteVia::Tailscale => crate::remote_tailscale::state(),
            RemoteVia::Relay => Default::default(),
        },
        address: crate::remote_host::address(),
        devices: Devices::read(root)
            .devices
            .into_iter()
            .map(|one| RemoteDevice {
                connected: connected.contains(&one.id),
                relay: one.key.is_some(),
                id: one.id,
                name: one.name,
                typing: one.typing,
                answering: one.answering,
                paired_at: one.paired_at,
                last_seen: one.last_seen,
            })
            .collect(),
        problem,
    }
}

fn enabled(store: &Store) -> bool {
    store
        .preference(preference::REMOTE)
        .ok()
        .flatten()
        .as_deref()
        == Some("true")
}

fn via(store: &Store) -> RemoteVia {
    match store
        .preference(preference::REMOTE_VIA)
        .ok()
        .flatten()
        .as_deref()
    {
        Some("relay") => RemoteVia::Relay,
        _ => RemoteVia::Tailscale,
    }
}

fn port(store: &Store) -> u16 {
    store
        .preference(preference::REMOTE_PORT)
        .ok()
        .flatten()
        .and_then(|port| port.parse().ok())
        .unwrap_or(0)
}

/// Opens the way in `store` names; why it could not, if it could not. The
/// other is closed first: one way in at a time.
fn open(app: &tauri::AppHandle, store: &Store, root: &Path) -> Result<(), String> {
    match via(store) {
        RemoteVia::Tailscale => {
            crate::remote_relay::stop(app);
            let (_, port) = crate::remote_host::start(app, root.to_owned(), port(store))?;
            store
                .set_preference(preference::REMOTE_PORT, &port.to_string())
                .map_err(|err| err.to_string())
        }
        RemoteVia::Relay => {
            crate::remote_host::stop(app);
            crate::remote_relay::start(app, root.to_owned())
        }
    }
}

fn close(app: &tauri::AppHandle) {
    crate::remote_host::stop(app);
    crate::remote_relay::stop(app);
    crate::remote_pairing::withdraw();
    crate::remote_hub::drop_all(app);
}

/// Opens the way in when Remote was left on: at launch.
pub(crate) fn restore(app: &tauri::AppHandle) {
    crate::remote_hub::listen(app);
    let app = app.clone();
    std::thread::spawn(move || {
        let (Ok(store), Ok(root)) = (Store::open_default(), Store::root()) else {
            return;
        };
        if enabled(&store) {
            if let Err(why) = open(&app, &store, &root) {
                eprintln!("remote did not start: {why}");
            }
        }
    });
}

/// `remote.read` — Remote, as the settings show it.
#[tauri::command]
#[specta::specta]
pub async fn remote_read() -> Result<RemoteView, RpcError> {
    crate::off_main::blocking(|| {
        let store = Store::open_default()?;
        Ok(view(&Store::root()?, &store, None))
    })
    .await
}

/// `remote.set` — on or off. On says why, when it cannot be reached.
#[tauri::command]
#[specta::specta]
pub async fn remote_set(app: tauri::AppHandle, on: bool) -> Result<RemoteView, RpcError> {
    crate::off_main::blocking(move || {
        let store = Store::open_default()?;
        let root = Store::root()?;
        store.set_preference(preference::REMOTE, if on { "true" } else { "false" })?;
        if !on {
            close(&app);
            return Ok(view(&root, &store, None));
        }
        let problem = open(&app, &store, &root).err();
        Ok(view(&root, &store, problem))
    })
    .await
}

/// `remote.via` — the way devices come in: the tailnet, or devpit.app's relay.
/// Switching drops every device connected the other way.
#[tauri::command]
#[specta::specta]
pub async fn remote_via(app: tauri::AppHandle, via: RemoteVia) -> Result<RemoteView, RpcError> {
    crate::off_main::blocking(move || {
        let store = Store::open_default()?;
        let root = Store::root()?;
        let word = match via {
            RemoteVia::Tailscale => "tailscale",
            RemoteVia::Relay => "relay",
        };
        store.set_preference(preference::REMOTE_VIA, word)?;
        if !enabled(&store) {
            return Ok(view(&root, &store, None));
        }
        close(&app);
        let problem = open(&app, &store, &root).err();
        Ok(view(&root, &store, problem))
    })
    .await
}

/// `remote.pair` — a code for one device, shown here, good once for two
/// minutes. Through the relay the QR code is the whole of it: it carries this
/// machine's key, which the device checks the machine against, and a secret
/// too long to type.
#[tauri::command]
#[specta::specta]
pub async fn remote_pair() -> Result<RemotePairing, RpcError> {
    crate::off_main::blocking(|| {
        let store = Store::open_default()?;
        let relay = via(&store) == RemoteVia::Relay;
        let address = if relay {
            crate::remote_relay::running().then_some(String::new())
        } else {
            crate::remote_host::address()
        }
        .ok_or_else(|| RpcError::new(ErrorCode::Conflict, "turn Remote on first"))?;
        let offered = crate::remote_pairing::offer(now())
            .ok_or_else(|| RpcError::internal("no pairing code could be made"))?;
        let url = if relay {
            crate::remote_relay::pairing_url(
                &Store::root()?,
                &offered.secret,
                &crate::remote_conn::machine_name(),
            )
            .map_err(RpcError::internal)?
        } else {
            format!("{address}#pair={}", offered.code)
        };
        let qr_svg = qrcode::QrCode::new(url.as_bytes())
            .map(|code| {
                code.render::<qrcode::render::svg::Color>()
                    .min_dimensions(200, 200)
                    .quiet_zone(true)
                    .build()
            })
            .unwrap_or_default();
        Ok(RemotePairing {
            code: offered.code,
            url,
            qr_svg,
            expires_at: offered.expires_at,
        })
    })
    .await
}

/// `remote.pair_cancel` — the pairing code is put away.
#[tauri::command]
#[specta::specta]
pub fn remote_pair_cancel() {
    crate::remote_pairing::withdraw();
}

/// `remote.allow` — what a device may do besides watching. Its connections
/// end, and it connects again with what it has now.
#[tauri::command]
#[specta::specta]
pub async fn remote_allow(
    app: tauri::AppHandle,
    id: String,
    typing: bool,
    answering: bool,
) -> Result<RemoteView, RpcError> {
    crate::off_main::blocking(move || {
        let root = Store::root()?;
        let mut devices = Devices::read(&root);
        if !devices.allow(&id, typing, answering) {
            return Err(RpcError::new(ErrorCode::NotFound, "no such device"));
        }
        devices
            .write(&root)
            .map_err(|err| RpcError::internal(err.to_string()))?;
        crate::remote_hub::drop_device(&app, &id);
        Ok(view(&root, &Store::open_default()?, None))
    })
    .await
}

/// `remote.forget` — the device is unpaired, and dropped at once: its
/// connections end, and through the relay their session keys with them.
#[tauri::command]
#[specta::specta]
pub async fn remote_forget(app: tauri::AppHandle, id: String) -> Result<RemoteView, RpcError> {
    crate::off_main::blocking(move || {
        let root = Store::root()?;
        let mut devices = Devices::read(&root);
        devices.forget(&id);
        devices
            .write(&root)
            .map_err(|err| RpcError::internal(err.to_string()))?;
        crate::remote_hub::drop_device(&app, &id);
        Ok(view(&root, &Store::open_default()?, None))
    })
    .await
}

/// `remote.drop` — every device watching now is disconnected.
#[tauri::command]
#[specta::specta]
pub fn remote_drop(app: tauri::AppHandle) {
    crate::remote_hub::drop_all(&app);
}

/// `remote.shapes` — the viewer's messages, for the generated contract.
#[tauri::command]
#[specta::specta]
pub fn remote_shapes() -> RemoteShapes {
    RemoteShapes {
        incoming: Vec::new(),
        outgoing: Vec::new(),
    }
}

#[cfg(test)]
#[path = "remote_tests.rs"]
mod tests;
