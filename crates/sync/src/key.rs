//! The account's sync key: 32 random bytes made on the first machine and
//! carried to the others by the person — through the relay's Noise session,
//! or as a recovery code — and never to devpit.app.
//!
//! Nothing seals with the key itself. HKDF-SHA256 draws a sealing key and the
//! key's id from it, and HMAC-SHA256 under a third derived key names the
//! blobs, so the server sees neither the key nor which blob holds what.

use std::fmt;

use data_encoding::{Encoding, Specification};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

/// The letters of a recovery code: no 0/O, no 1/I, as a pairing code.
const LETTERS: &str = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
/// Bytes of the key's hash kept at the end of a recovery code, so a mistyped
/// one is caught before it is tried.
const CHECK: usize = 2;
/// A recovery code is written in groups of this many letters.
const GROUP: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyError {
    /// The system gave no randomness.
    NoRandom,
    /// A recovery code or a kept key that is not one.
    NotAKey(&'static str),
}

impl fmt::Display for KeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyError::NoRandom => f.write_str("the system gave no randomness for a key"),
            KeyError::NotAKey(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for KeyError {}

#[derive(Clone)]
pub struct SyncKey([u8; 32]);

impl Drop for SyncKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for SyncKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The key never reaches a log; its id may.
        write!(f, "SyncKey({})", self.id())
    }
}

impl SyncKey {
    pub fn generate() -> Result<SyncKey, KeyError> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| KeyError::NoRandom)?;
        Ok(SyncKey(bytes))
    }

    pub fn from_bytes(bytes: [u8; 32]) -> SyncKey {
        SyncKey(bytes)
    }

    /// 32 bytes drawn from the key for `purpose`; each purpose its own.
    fn derived(&self, purpose: &str) -> [u8; 32] {
        let hkdf = Hkdf::<Sha256>::from_prk(&self.0).expect("32 bytes is a whole PRK");
        let mut out = [0u8; 32];
        hkdf.expand(purpose.as_bytes(), &mut out)
            .expect("32 bytes is within what HKDF-SHA256 expands");
        out
    }

    /// The key that seals blobs.
    pub(crate) fn sealing(&self) -> [u8; 32] {
        self.derived("devpit-sync/1 seal")
    }

    /// Eight bytes that tell two keys apart without saying either: written on
    /// every blob, so a machine holding another key knows it at once.
    pub fn id(&self) -> String {
        hex(&self.id_bytes())
    }

    pub(crate) fn id_bytes(&self) -> [u8; 8] {
        let mut id = [0u8; 8];
        id.copy_from_slice(&self.derived("devpit-sync/1 key id")[..8]);
        id
    }

    /// The name devpit.app keeps `namespace`'s blob under: an HMAC under the
    /// key, so the name says nothing — not even which project a board is.
    pub fn blob_name(&self, namespace: &str) -> String {
        let mut naming =
            <Hmac<Sha256> as Mac>::new_from_slice(&self.derived("devpit-sync/1 names"))
                .expect("HMAC takes a key of any length");
        naming.update(namespace.as_bytes());
        hex(&naming.finalize().into_bytes()[..16])
    }

    /// The key as the machine keeps it, and as it crosses to another.
    pub fn to_hex(&self) -> String {
        hex(&self.0)
    }

    pub fn from_hex(text: &str) -> Result<SyncKey, KeyError> {
        devpit_remote::noise::unhex32(text.trim())
            .map(SyncKey)
            .ok_or(KeyError::NotAKey("that is not a sync key"))
    }

    /// The key as the person writes it down: groups of letters, with two
    /// bytes of its hash at the end that catch a mistyped letter.
    pub fn recovery_code(&self) -> String {
        let mut payload = self.0.to_vec();
        payload.extend_from_slice(&check(&self.0));
        let letters = letters().encode(&payload);
        payload.zeroize();
        letters
            .as_bytes()
            .chunks(GROUP)
            .map(|group| String::from_utf8_lossy(group).into_owned())
            .collect::<Vec<_>>()
            .join("-")
    }

    /// The key a recovery code was made from. Case, spaces and dashes are
    /// the person's to choose.
    pub fn from_recovery_code(code: &str) -> Result<SyncKey, KeyError> {
        let typed: String = code
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-')
            .map(|c| c.to_ascii_uppercase())
            .collect();
        let mut payload = letters()
            .decode(typed.as_bytes())
            .map_err(|_| KeyError::NotAKey("that recovery code has a letter it never uses — check it against what you wrote down"))?;
        if payload.len() != 32 + CHECK {
            payload.zeroize();
            return Err(KeyError::NotAKey(
                "that recovery code is too short or too long",
            ));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&payload[..32]);
        let matches = check(&key)[..] == payload[32..];
        payload.zeroize();
        if !matches {
            key.zeroize();
            return Err(KeyError::NotAKey(
                "that recovery code does not add up — a letter is wrong",
            ));
        }
        Ok(SyncKey(key))
    }
}

fn check(key: &[u8; 32]) -> [u8; CHECK] {
    let digest = Sha256::new()
        .chain_update(b"devpit-sync/1 recovery")
        .chain_update(key)
        .finalize();
    let mut out = [0u8; CHECK];
    out.copy_from_slice(&digest[..CHECK]);
    out
}

fn letters() -> Encoding {
    let mut spec = Specification::new();
    spec.symbols.push_str(LETTERS);
    spec.encoding().expect("32 distinct letters make a base32")
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    devpit_remote::noise::hex(bytes)
}

#[cfg(test)]
#[path = "key_tests.rs"]
mod tests;
