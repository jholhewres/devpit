//! The devices paired with this machine, and what each may do.
//!
//! A device holds a token its pairing gave it once; the machine keeps only
//! the token's SHA-256, in a file only this user can read. Every device sees.
//! Typing and answering are granted here, on the machine, one device at a
//! time — never from the device itself.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The most devices kept: pairing is for the person's own few.
pub(crate) const MOST: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Device {
    pub id: String,
    pub name: String,
    /// SHA-256 of the token, in hex.
    pub token_hash: String,
    #[serde(default)]
    pub typing: bool,
    #[serde(default)]
    pub answering: bool,
    pub paired_at: f64,
    #[serde(default)]
    pub last_seen: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub(crate) struct Devices {
    pub devices: Vec<Device>,
}

pub(crate) fn file(root: &Path) -> PathBuf {
    root.join("remote-devices.json")
}

pub(crate) fn hash(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Equal without saying, by how long it took, how much of it was.
fn same(one: &str, other: &str) -> bool {
    one.len() == other.len()
        && one
            .bytes()
            .zip(other.bytes())
            .fold(0u8, |differs, (a, b)| differs | (a ^ b))
            == 0
}

impl Devices {
    pub(crate) fn read(root: &Path) -> Devices {
        std::fs::read_to_string(file(root))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub(crate) fn write(&self, root: &Path) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        devpit_core::home::write_private(&file(root), text.as_bytes())
    }

    /// The device a token belongs to.
    pub(crate) fn by_token(&self, token: &str) -> Option<&Device> {
        let wanted = hash(token);
        // Every one compared, so which device matched takes no less time.
        let mut found = None;
        for device in &self.devices {
            if same(&device.token_hash, &wanted) {
                found = Some(device);
            }
        }
        found
    }

    /// A new device that sees and does nothing else, with its token.
    pub(crate) fn pair(&mut self, name: &str, token: &str, now: f64) -> Result<Device, String> {
        if self.devices.len() >= MOST {
            return Err(format!(
                "{MOST} devices are paired already — forget one first"
            ));
        }
        let name: String = name
            .trim()
            .chars()
            .filter(|c| !c.is_control())
            .take(60)
            .collect();
        let device = Device {
            id: format!("dev_{}", ulid::Ulid::generate()),
            name: if name.is_empty() {
                "A device".to_owned()
            } else {
                name
            },
            token_hash: hash(token),
            typing: false,
            answering: false,
            paired_at: now,
            last_seen: None,
        };
        self.devices.push(device.clone());
        Ok(device)
    }

    pub(crate) fn allow(&mut self, id: &str, typing: bool, answering: bool) -> bool {
        match self.devices.iter_mut().find(|one| one.id == id) {
            Some(device) => {
                device.typing = typing;
                device.answering = answering;
                true
            }
            None => false,
        }
    }

    pub(crate) fn forget(&mut self, id: &str) -> bool {
        let before = self.devices.len();
        self.devices.retain(|one| one.id != id);
        self.devices.len() != before
    }

    pub(crate) fn seen(&mut self, id: &str, now: f64) {
        if let Some(device) = self.devices.iter_mut().find(|one| one.id == id) {
            device.last_seen = Some(now);
        }
    }
}

/// A fresh secret, as hex: a device's token, or a pairing's.
pub(crate) fn fresh_token() -> Option<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).ok()?;
    Some(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(test)]
#[path = "remote_devices_tests.rs"]
mod tests;
