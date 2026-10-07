//! Remote: this machine, reached from the person's other devices over their
//! tailnet — terminals watched and typed into, the board, the questions
//! agents wait on, the chats. Off unless turned on, here.
//!
//! The settings' half: turning it on and off, pairing a device, what each
//! paired device may do, and a log of who connected and did what — never
//! what was on screen or typed.

use std::io::Write as _;
use std::path::Path;

use devpit_core::{preference, Store};
use devpit_rpc::{ErrorCode, RemoteDevice, RemotePairing, RemoteShapes, RemoteView, RpcError};
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

fn view(root: &Path, enabled: bool, problem: Option<String>) -> RemoteView {
    let connected = crate::remote_hub::connected();
    RemoteView {
        enabled,
        tailscale: crate::remote_tailscale::state(),
        address: crate::remote_host::address(),
        devices: Devices::read(root)
            .devices
            .into_iter()
            .map(|one| RemoteDevice {
                connected: connected.contains(&one.id),
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

fn port(store: &Store) -> u16 {
    store
        .preference(preference::REMOTE_PORT)
        .ok()
        .flatten()
        .and_then(|port| port.parse().ok())
        .unwrap_or(0)
}

/// Starts the host when it was left on: at launch.
pub(crate) fn restore(app: &tauri::AppHandle) {
    crate::remote_hub::listen(app);
    let app = app.clone();
    std::thread::spawn(move || {
        let (Ok(store), Ok(root)) = (Store::open_default(), Store::root()) else {
            return;
        };
        if enabled(&store) {
            if let Err(why) = crate::remote_host::start(&app, root, port(&store)) {
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
        Ok(view(&Store::root()?, enabled(&store), None))
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
            crate::remote_host::stop(&app);
            return Ok(view(&root, false, None));
        }
        let problem = match crate::remote_host::start(&app, root.clone(), port(&store)) {
            Ok((_, port)) => {
                store.set_preference(preference::REMOTE_PORT, &port.to_string())?;
                None
            }
            Err(why) => Some(why),
        };
        Ok(view(&root, true, problem))
    })
    .await
}

/// `remote.pair` — a code for one device, shown here, good once for two minutes.
#[tauri::command]
#[specta::specta]
pub async fn remote_pair() -> Result<RemotePairing, RpcError> {
    crate::off_main::blocking(|| {
        let address = crate::remote_host::address()
            .ok_or_else(|| RpcError::new(ErrorCode::Conflict, "turn Remote on first"))?;
        let offered = crate::remote_pairing::offer(now())
            .ok_or_else(|| RpcError::internal("no pairing code could be made"))?;
        let url = format!("{address}#pair={}", offered.code);
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
        Ok(view(&root, enabled(&Store::open_default()?), None))
    })
    .await
}

/// `remote.forget` — the device is unpaired, and dropped at once.
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
        Ok(view(&root, enabled(&Store::open_default()?), None))
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
