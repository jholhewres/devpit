//! A secret kept from the Remote: the value comes over the paired device's own
//! connection and goes straight into the vault, the same write the window
//! makes. It is never said back, logged, typed into a terminal or sent on to
//! devpit.app; the log says which device kept which name, and nothing else.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use devpit_core::home::ProjectHome;
use devpit_core::Store;

use crate::remote_devices::Devices;

/// Secrets one device may keep in a minute: a person adds a few at a time,
/// and a token in the wrong hands writes no faster than that.
const PER_MINUTE: usize = 5;
const MINUTE: f64 = 60.0;

/// Whether one more is let in, given when this device's last ones were.
pub(crate) fn admitted(times: &mut Vec<f64>, now: f64) -> bool {
    times.retain(|at| now - at < MINUTE);
    if times.len() >= PER_MINUTE {
        return false;
    }
    times.push(now);
    true
}

fn recent() -> &'static Mutex<HashMap<String, Vec<f64>>> {
    static RECENT: OnceLock<Mutex<HashMap<String, Vec<f64>>>> = OnceLock::new();
    RECENT.get_or_init(Mutex::default)
}

/// Keeps `value` as `name` in a project's `.env` for the device `device_id`,
/// and answers the name kept — or why not, never repeating the value.
///
/// The pairing is read again here rather than trusted to the connection: a
/// device forgotten or stopped from typing a moment ago writes nothing.
pub(crate) fn keep(
    store: &Store,
    root: &Path,
    device_id: &str,
    project_id: &str,
    name: &str,
    value: &str,
    now: f64,
) -> Result<String, String> {
    let devices = Devices::read(root);
    let Some(device) = devices.devices.iter().find(|one| one.id == device_id) else {
        return Err("this device is not paired with this machine — pair it again".to_owned());
    };
    if !device.typing {
        return Err("this device may only watch — allow it to type on the machine".to_owned());
    }
    let let_in = recent()
        .lock()
        .map(|mut all| admitted(all.entry(device.id.clone()).or_default(), now))
        .unwrap_or(false);
    if !let_in {
        return Err(format!(
            "at most {PER_MINUTE} secrets a minute — try again in a moment"
        ));
    }
    // Before anything reads the project: an oversize value goes no further.
    if value.len() > crate::vault::LONGEST_VALUE {
        return Err("a secret is one line, at most 8 KB".to_owned());
    }
    let project = store
        .project(project_id)
        .map_err(|err| err.to_string())?
        .ok_or("that project is not registered")?;
    let kept = ProjectHome::of(store, root, project_id)
        .map_err(|_| "that project has no home here".to_owned())?
        .secrets();
    let name = name.trim();
    crate::vault::keep(Path::new(&project.root_path), &kept, name, value)
        .map_err(|err| err.message)?;
    crate::vault_rotation::kept_again(store, project_id, &kept, name, now as i64)
        .map_err(|err| err.message)?;
    crate::remote::note(
        root,
        device,
        &format!("kept the secret {name} in {}", project.name),
    );
    Ok(name.to_owned())
}

#[cfg(test)]
#[path = "remote_secret_tests.rs"]
mod tests;
